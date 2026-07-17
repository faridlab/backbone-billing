//! Validated write path + AR/AP posting engine for billing (hand-authored, user-owned).
//!
//! Region-neutral: invoices carry NO tax columns; tax lines live in the removable `InvoiceTaxLine`
//! overlay (supplied for now; `backbone-tax` computes them later). On post, billing assembles the
//! net lines + overlay tax into ONE balanced `AccountingPost`:
//!   - **Sales:** `Dr A/R (grand) [customer] · Cr Revenue (net per account) · Cr PPN Output (Σoutput)`
//!   - **Purchase:** `Dr Expense (net) · Dr PPN Input (Σinput) · Cr A/P (grand) [supplier] · Cr PPh (Σwithholding)`
//! `grand = net + output` (sales); `grand = net + input − withholding` (purchase, the A/P owed).
//! Posting is idempotent (source_id = invoice id) + reconciled from the ack, like every seam.
//!
//! **Layering (the module's 4-layer rule).** This service ORCHESTRATES: it prices, validates, owns
//! the unit of work (`begin`/`commit`), decides the company scope, and publishes seam events. It
//! holds no SQL — every statement lives in `infrastructure::persistence`, and the repository methods
//! that participate in a transaction take THIS service's connection so cross-entity writes commit
//! together.

use backbone_orm::company_scope;
use rust_decimal::{Decimal, RoundingStrategy};
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entity::InvoiceKind;
use crate::infrastructure::persistence::{
    parse_invoice_kind, InvoiceSettlementRepository, InvoiceTaxLineRepository, NewInvoiceTaxLineRow,
    NewPaymentScheduleRow, NewPurchaseInvoiceLineRow, NewPurchaseInvoiceRow, NewSalesInvoiceLineRow,
    NewSalesInvoiceRow, PaymentScheduleRepository, PostingStateRow, PurchaseInvoiceLineRepository,
    PurchaseInvoiceRepository, SalesInvoiceLineRepository, SalesInvoiceRepository,
};

use super::billing_events::{
    BilledLine, BillingEvent, BillingEventSink, InvoiceCancelled, LoggingSink, PurchaseInvoicePosted,
    SalesInvoicePosted,
};
use super::billing_gl::{AccountingPostEnvelope, GlPostLine, GlPostSink};

fn money(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

// --- input structs -----------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewInvoiceLine {
    pub item_id: Uuid,
    /// Revenue account (sales) or expense/GR-IR account (purchase).
    pub account_id: Uuid,
    pub description: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

/// A supplied tax line for the overlay. `basis`: "output" | "input" | "withholding".
#[derive(Debug, Clone)]
pub struct NewTaxLine {
    pub account_id: Uuid,
    pub basis: String,
    pub description: Option<String>,
    pub rate: Decimal,
    pub tax_amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct NewSalesInvoice {
    pub invoice_number: String,
    pub company_id: Uuid,
    pub branch_id: Option<Uuid>,
    pub customer_id: Uuid,
    pub source_so_id: Option<Uuid>,
    pub posting_date: chrono::NaiveDate,
    pub due_date: Option<chrono::NaiveDate>,
    pub currency: Option<String>,
    pub receivable_account_id: Uuid,
    pub lines: Vec<NewInvoiceLine>,
    pub tax_lines: Vec<NewTaxLine>,
}

#[derive(Debug, Clone)]
pub struct NewPurchaseInvoice {
    pub invoice_number: String,
    pub company_id: Uuid,
    pub branch_id: Option<Uuid>,
    pub supplier_id: Uuid,
    pub source_po_id: Option<Uuid>,
    pub posting_date: chrono::NaiveDate,
    pub due_date: Option<chrono::NaiveDate>,
    pub currency: Option<String>,
    pub payable_account_id: Uuid,
    pub lines: Vec<NewInvoiceLine>,
    pub tax_lines: Vec<NewTaxLine>,
}

#[derive(Debug, Clone)]
pub struct PostOutcome {
    pub invoice_id: Uuid,
    pub post_id: Uuid,
    pub journal_id: Uuid,
    pub idempotent_reuse: bool,
}

// --- errors ------------------------------------------------------------------

#[derive(Debug)]
pub enum BillingError {
    EmptyDocument,
    NegativeAmount,
    UnsupportedCurrency(String),
    UnbalancedPost,
    DuplicateNumber(String),
    InvoiceNotFound(Uuid),
    NotDraft(String),
    GlRejected { code: String, message: String },
    Db(sqlx::Error),
}

impl BillingError {
    pub fn code(&self) -> String {
        match self {
            BillingError::EmptyDocument => "empty_document".into(),
            BillingError::NegativeAmount => "negative_amount".into(),
            BillingError::UnsupportedCurrency(_) => "unsupported_currency".into(),
            BillingError::UnbalancedPost => "unbalanced_post".into(),
            BillingError::DuplicateNumber(_) => "duplicate_number".into(),
            BillingError::InvoiceNotFound(_) => "invoice_not_found".into(),
            BillingError::NotDraft(_) => "invoice_not_draft".into(),
            BillingError::GlRejected { code, .. } => code.clone(),
            BillingError::Db(_) => "internal_error".into(),
        }
    }
    pub fn http_status(&self) -> u16 {
        match self {
            BillingError::InvoiceNotFound(_) => 404,
            BillingError::Db(_) => 500,
            _ => 422,
        }
    }
}
impl std::fmt::Display for BillingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BillingError::GlRejected { code, message } => write!(f, "{code}: {message}"),
            other => write!(f, "{}", other.code()),
        }
    }
}
impl std::error::Error for BillingError {}
impl From<sqlx::Error> for BillingError {
    fn from(e: sqlx::Error) -> Self { BillingError::Db(e) }
}
fn is_dup(e: &sqlx::Error) -> bool {
    e.as_database_error().map(|d| d.is_unique_violation()).unwrap_or(false)
}

/// Resolve the settlement seam's wire `kind` into the closed enum the repository dispatches on.
/// Unknown kinds keep their original domain error.
fn invoice_kind(kind: &str) -> Result<InvoiceKind, BillingError> {
    parse_invoice_kind(kind).ok_or_else(|| BillingError::NotDraft(format!("unknown invoice kind {kind}")))
}

struct PricedLine {
    item_id: Uuid,
    account_id: Uuid,
    description: Option<String>,
    quantity: Decimal,
    unit_price: Decimal,
    net_amount: Decimal,
}

/// Price lines (`net = money(qty·price)`), reject empty/negative, and total the supplied tax lines
/// by basis. Returns (priced, net_total, output, input, withholding).
fn price(lines: &[NewInvoiceLine], tax: &[NewTaxLine]) -> Result<(Vec<PricedLine>, Decimal, Decimal, Decimal, Decimal), BillingError> {
    if lines.is_empty() {
        return Err(BillingError::EmptyDocument);
    }
    let mut priced = Vec::with_capacity(lines.len());
    let mut net_total = Decimal::ZERO;
    for l in lines {
        if l.quantity < Decimal::ZERO || l.unit_price < Decimal::ZERO {
            return Err(BillingError::NegativeAmount);
        }
        let net = money(l.quantity * l.unit_price);
        net_total += net;
        priced.push(PricedLine {
            item_id: l.item_id, account_id: l.account_id, description: l.description.clone(),
            quantity: l.quantity, unit_price: l.unit_price, net_amount: net,
        });
    }
    let mut output = Decimal::ZERO; let mut input = Decimal::ZERO; let mut wht = Decimal::ZERO;
    for t in tax {
        if t.tax_amount < Decimal::ZERO { return Err(BillingError::NegativeAmount); }
        match t.basis.as_str() {
            "output" => output += t.tax_amount,
            "input" => input += t.tax_amount,
            "withholding" => wht += t.tax_amount,
            _ => {}
        }
    }
    Ok((priced, money(net_total), money(output), money(input), money(wht)))
}

/// Reconcile a `PostOutcome` from an invoice's persisted posting state — the shared tail of the
/// posted short-circuit, for both invoice kinds.
fn posted_outcome(invoice_id: Uuid, row: PostingStateRow) -> Option<PostOutcome> {
    if row.posting_state == "posted" {
        if let (Some(j), Some(p)) = (row.journal_id, row.accounting_post_id) {
            return Some(PostOutcome { invoice_id, post_id: p, journal_id: j, idempotent_reuse: true });
        }
    }
    None
}

#[derive(Clone)]
pub struct BillingWriteService {
    db_pool: PgPool,
    sink: Arc<dyn BillingEventSink>,
    sales: Arc<SalesInvoiceRepository>,
    sales_lines: Arc<SalesInvoiceLineRepository>,
    purchases: Arc<PurchaseInvoiceRepository>,
    purchase_lines: Arc<PurchaseInvoiceLineRepository>,
    tax_lines: Arc<InvoiceTaxLineRepository>,
    schedules: Arc<PaymentScheduleRepository>,
    settlement: Arc<InvoiceSettlementRepository>,
}

impl BillingWriteService {
    pub fn new(db_pool: PgPool) -> Self {
        Self::with_sink(db_pool, Arc::new(LoggingSink))
    }
    pub fn with_sink(db_pool: PgPool, sink: Arc<dyn BillingEventSink>) -> Self {
        Self {
            sales: Arc::new(SalesInvoiceRepository::new(db_pool.clone())),
            sales_lines: Arc::new(SalesInvoiceLineRepository::new(db_pool.clone())),
            purchases: Arc::new(PurchaseInvoiceRepository::new(db_pool.clone())),
            purchase_lines: Arc::new(PurchaseInvoiceLineRepository::new(db_pool.clone())),
            tax_lines: Arc::new(InvoiceTaxLineRepository::new(db_pool.clone())),
            schedules: Arc::new(PaymentScheduleRepository::new(db_pool.clone())),
            settlement: Arc::new(InvoiceSettlementRepository::new()),
            db_pool,
            sink,
        }
    }

    async fn insert_tax_lines(&self, tx: &mut sqlx::PgConnection, invoice_id: Uuid, kind: &str, tax: &[NewTaxLine]) -> Result<(), BillingError> {
        for t in tax {
            self.tax_lines.insert_tax_line(&mut *tx, &NewInvoiceTaxLineRow {
                id: Uuid::new_v4(),
                invoice_ref: invoice_id,
                kind,
                account_id: t.account_id,
                basis: &t.basis,
                description: t.description.as_deref(),
                rate: t.rate,
                tax_amount: money(t.tax_amount),
            }).await?;
        }
        Ok(())
    }

    // ---- Sales Invoice (AR) -------------------------------------------------

    pub async fn create_sales_invoice(&self, inv: NewSalesInvoice) -> Result<Uuid, BillingError> {
        let (priced, net_total, output, _in, _wht) = price(&inv.lines, &inv.tax_lines)?;
        let grand = net_total + output;
        let id = Uuid::new_v4();
        let currency = inv.currency.unwrap_or_else(|| "IDR".into());
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): company on the DTO.
        company_scope::bind_company_on(&mut tx, inv.company_id).await?;
        let r = self.sales.insert_draft(&mut tx, &NewSalesInvoiceRow {
            id,
            invoice_number: &inv.invoice_number,
            company_id: inv.company_id,
            branch_id: inv.branch_id,
            customer_id: inv.customer_id,
            source_so_id: inv.source_so_id,
            posting_date: inv.posting_date,
            due_date: inv.due_date,
            currency: &currency,
            net_total,
            tax_total: output,
            grand_total: grand,
            receivable_account_id: inv.receivable_account_id,
        }).await;
        if let Err(e) = r {
            return Err(if is_dup(&e) { BillingError::DuplicateNumber(inv.invoice_number) } else { e.into() });
        }
        for p in &priced {
            self.sales_lines.insert_line(&mut tx, &NewSalesInvoiceLineRow {
                id: Uuid::new_v4(),
                invoice_id: id,
                item_id: p.item_id,
                account_id: p.account_id,
                description: p.description.as_deref(),
                quantity: p.quantity,
                unit_price: p.unit_price,
                net_amount: p.net_amount,
            }).await?;
        }
        self.insert_tax_lines(&mut tx, id, "sales", &inv.tax_lines).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Build the balanced revenue post: `Dr A/R (grand) [customer] · Cr Revenue (net per account) ·
    /// Cr PPN Output (per overlay output line)`.
    pub async fn build_ar_post(&self, invoice_id: Uuid) -> Result<AccountingPostEnvelope, BillingError> {
        // RLS scope (ADR-0008), ID-only: fenced by the request/inherited scope.
        let inv = self.sales.fetch_ar_header(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        let currency = inv.currency;
        if currency != "IDR" { return Err(BillingError::UnsupportedCurrency(currency)); }

        // Cr Revenue per income account.
        let rev_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.sales_lines.fetch_revenue_amounts(&self.db_pool, invoice_id),
        ).await?;
        let mut revenue: BTreeMap<Uuid, Decimal> = BTreeMap::new();
        for r in &rev_rows {
            *revenue.entry(r.revenue_account_id).or_insert(Decimal::ZERO) += r.net_amount;
        }
        // Cr PPN Output per overlay output line.
        let tax_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.tax_lines.fetch_amounts_by_basis(&self.db_pool, invoice_id, "sales", "output"),
        ).await?;

        let mut lines = vec![
            GlPostLine::debit(inv.receivable_account_id, inv.grand_total)
                .with_party("customer", inv.customer_id)
                .with_description(format!("A/R {}", inv.invoice_number)),
        ];
        for (acct, amt) in &revenue { lines.push(GlPostLine::credit(*acct, *amt).with_description("Revenue")); }
        for t in &tax_rows { lines.push(GlPostLine::credit(t.account_id, t.tax_amount).with_description("PPN Output")); }

        let env = AccountingPostEnvelope {
            idempotency_key: invoice_id.to_string(), company_id: inv.company_id, branch_id: inv.branch_id,
            source_type: "order".into(), source_id: invoice_id, source_reference: Some(inv.invoice_number),
            posting_date: inv.posting_date, currency, posting_type: "original".into(), reverses_post_id: None,
            description: Some("Sales invoice".into()), lines,
        };
        if !env.is_balanced() { return Err(BillingError::UnbalancedPost); }
        Ok(env)
    }

    pub async fn post_sales_invoice(&self, invoice_id: Uuid, sink: &dyn GlPostSink) -> Result<PostOutcome, BillingError> {
        if let Some(o) = self.short_circuit_sales_posted(invoice_id).await? { return Ok(o); }
        let env = self.build_ar_post(invoice_id).await?;
        match sink.post(&env).await {
            Ok(ack) => {
                // Gate the reconcile + seam event on THIS invocation actually performing the
                // pending→posted transition. Two concurrent posts both clear short_circuit and both
                // get a (deduped) ack from accounting; only the one whose UPDATE hits a row may emit
                // `SalesInvoicePosted` — otherwise the event double-fires to downstream consumers.
                let affected = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.sales.mark_posted(&self.db_pool, invoice_id, ack.journal_id, ack.post_id),
                ).await?;
                if affected == 0 {
                    // Lost the race: a concurrent post already transitioned + emitted. Reconcile
                    // from the persisted row; do NOT re-publish the seam event.
                    return self.short_circuit_sales_posted(invoice_id).await?
                        .ok_or(BillingError::InvoiceNotFound(invoice_id));
                }
                let hdr = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.sales.fetch_seam_header(&self.db_pool, invoice_id),
                ).await?;
                // Billed lines (item + qty) for the selling seam — advance the source SO's billed_qty.
                let line_rows = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.sales_lines.fetch_billed_lines(&self.db_pool, invoice_id),
                ).await?;
                let billed_lines: Vec<BilledLine> = line_rows.iter()
                    .map(|r| BilledLine { item_id: r.item_id, quantity: r.quantity }).collect();
                self.sink.publish(BillingEvent::SalesInvoicePosted(SalesInvoicePosted {
                    invoice_id, company_id: env.company_id, journal_id: ack.journal_id, post_id: ack.post_id,
                    source_so_id: hdr.source_so_id, billed_lines, grand_total: hdr.grand_total,
                }));
                Ok(PostOutcome { invoice_id, post_id: ack.post_id, journal_id: ack.journal_id, idempotent_reuse: ack.idempotent_reuse })
            }
            Err(rej) => {
                let _ = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.sales.mark_posting_failed(&self.db_pool, invoice_id),
                ).await;
                Err(BillingError::GlRejected { code: rej.code, message: rej.message })
            }
        }
    }

    // ---- Purchase Invoice (A/P) ---------------------------------------------

    pub async fn create_purchase_invoice(&self, inv: NewPurchaseInvoice) -> Result<Uuid, BillingError> {
        let (priced, net_total, _out, input, wht) = price(&inv.lines, &inv.tax_lines)?;
        let grand = net_total + input - wht; // A/P owed
        // Withholding is a deduction from the payable, never more than the base: a negative A/P would
        // persist a negative outstanding + emit a negative-A/P seam event (fails open). Refuse it.
        if grand < Decimal::ZERO { return Err(BillingError::NegativeAmount); }
        let id = Uuid::new_v4();
        let currency = inv.currency.unwrap_or_else(|| "IDR".into());
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): company on the DTO.
        company_scope::bind_company_on(&mut tx, inv.company_id).await?;
        let r = self.purchases.insert_draft(&mut tx, &NewPurchaseInvoiceRow {
            id,
            invoice_number: &inv.invoice_number,
            company_id: inv.company_id,
            branch_id: inv.branch_id,
            supplier_id: inv.supplier_id,
            source_po_id: inv.source_po_id,
            posting_date: inv.posting_date,
            due_date: inv.due_date,
            currency: &currency,
            net_total,
            tax_total: input,
            withholding_total: wht,
            grand_total: grand,
            payable_account_id: inv.payable_account_id,
        }).await;
        if let Err(e) = r {
            return Err(if is_dup(&e) { BillingError::DuplicateNumber(inv.invoice_number) } else { e.into() });
        }
        for p in &priced {
            self.purchase_lines.insert_line(&mut tx, &NewPurchaseInvoiceLineRow {
                id: Uuid::new_v4(),
                invoice_id: id,
                item_id: p.item_id,
                account_id: p.account_id,
                description: p.description.as_deref(),
                quantity: p.quantity,
                unit_price: p.unit_price,
                net_amount: p.net_amount,
            }).await?;
        }
        self.insert_tax_lines(&mut tx, id, "purchase", &inv.tax_lines).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Build the balanced A/P post: `Dr Expense (net per account) · Dr PPN Input (per input line) ·
    /// Cr A/P (grand) [supplier] · Cr PPh Payable (per withholding line)`.
    pub async fn build_ap_post(&self, invoice_id: Uuid) -> Result<AccountingPostEnvelope, BillingError> {
        // RLS scope (ADR-0008), ID-only: fenced by the request/inherited scope.
        let inv = self.purchases.fetch_ap_header(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        let currency = inv.currency;
        if currency != "IDR" { return Err(BillingError::UnsupportedCurrency(currency)); }

        let exp_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.purchase_lines.fetch_expense_amounts(&self.db_pool, invoice_id),
        ).await?;
        let mut expense: BTreeMap<Uuid, Decimal> = BTreeMap::new();
        for r in &exp_rows {
            *expense.entry(r.expense_account_id).or_insert(Decimal::ZERO) += r.net_amount;
        }
        let input_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.tax_lines.fetch_amounts_by_basis(&self.db_pool, invoice_id, "purchase", "input"),
        ).await?;
        let wht_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.tax_lines.fetch_amounts_by_basis(&self.db_pool, invoice_id, "purchase", "withholding"),
        ).await?;

        let mut lines: Vec<GlPostLine> = Vec::new();
        for (acct, amt) in &expense { lines.push(GlPostLine::debit(*acct, *amt).with_description("Expense/GR-IR")); }
        for t in &input_rows { lines.push(GlPostLine::debit(t.account_id, t.tax_amount).with_description("PPN Input")); }
        lines.push(GlPostLine::credit(inv.payable_account_id, inv.grand_total)
            .with_party("supplier", inv.supplier_id)
            .with_description(format!("A/P {}", inv.invoice_number)));
        for t in &wht_rows { lines.push(GlPostLine::credit(t.account_id, t.tax_amount).with_description("PPh Payable")); }

        let env = AccountingPostEnvelope {
            idempotency_key: invoice_id.to_string(), company_id: inv.company_id, branch_id: inv.branch_id,
            source_type: "expense".into(), source_id: invoice_id, source_reference: Some(inv.invoice_number),
            posting_date: inv.posting_date, currency, posting_type: "original".into(), reverses_post_id: None,
            description: Some("Purchase invoice".into()), lines,
        };
        if !env.is_balanced() { return Err(BillingError::UnbalancedPost); }
        Ok(env)
    }

    pub async fn post_purchase_invoice(&self, invoice_id: Uuid, sink: &dyn GlPostSink) -> Result<PostOutcome, BillingError> {
        if let Some(o) = self.short_circuit_purchase_posted(invoice_id).await? { return Ok(o); }
        let env = self.build_ap_post(invoice_id).await?;
        match sink.post(&env).await {
            Ok(ack) => {
                // Gate the reconcile + seam event on THIS invocation performing the pending→posted
                // transition — the A/P seam routes `PurchaseInvoicePosted.billed_lines` into
                // buying::mark_billed, so a double-emit would double-advance the PO's billed_qty and
                // corrupt the 3-way match. Only the winner of the UPDATE race publishes.
                let affected = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchases.mark_posted(&self.db_pool, invoice_id, ack.journal_id, ack.post_id),
                ).await?;
                if affected == 0 {
                    return self.short_circuit_purchase_posted(invoice_id).await?
                        .ok_or(BillingError::InvoiceNotFound(invoice_id));
                }
                // Billed lines (item + qty) for the buying seam.
                let hdr = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchases.fetch_seam_header(&self.db_pool, invoice_id),
                ).await?;
                let line_rows = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchase_lines.fetch_billed_lines(&self.db_pool, invoice_id),
                ).await?;
                let billed_lines: Vec<BilledLine> = line_rows.iter()
                    .map(|r| BilledLine { item_id: r.item_id, quantity: r.quantity }).collect();
                self.sink.publish(BillingEvent::PurchaseInvoicePosted(PurchaseInvoicePosted {
                    invoice_id, company_id: env.company_id, journal_id: ack.journal_id, post_id: ack.post_id,
                    source_po_id: hdr.source_po_id, billed_lines, grand_total: hdr.grand_total,
                }));
                Ok(PostOutcome { invoice_id, post_id: ack.post_id, journal_id: ack.journal_id, idempotent_reuse: ack.idempotent_reuse })
            }
            Err(rej) => {
                let _ = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchases.mark_posting_failed(&self.db_pool, invoice_id),
                ).await;
                Err(BillingError::GlRejected { code: rej.code, message: rej.message })
            }
        }
    }

    // ---- Payment schedule ---------------------------------------------------

    /// Attach installment due dates to an invoice (AR or A/P). Lean — settlement is payments' job.
    pub async fn add_payment_schedule(&self, invoice_ref: Uuid, kind: &str, company_id: Uuid, installments: &[(chrono::NaiveDate, Decimal)]) -> Result<(), BillingError> {
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): company is an explicit argument.
        company_scope::bind_company_on(&mut tx, company_id).await?;
        for (i, (due, amt)) in installments.iter().enumerate() {
            self.schedules.insert_schedule(&mut tx, &NewPaymentScheduleRow {
                id: Uuid::new_v4(),
                invoice_ref,
                kind,
                company_id,
                installment_no: (i + 1) as i32,
                due_date: *due,
                amount: money(*amt),
            }).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Apply a settlement against an invoice (driven by payment's `PaymentSettled` via an ACL) — the
    /// cash-loop seam target. **Billing's invariant (CLAMP, council 2026-07-05):** it knocks off only
    /// what is owed — `applied = min(requested, outstanding)` — and returns `applied`. It never
    /// rejects an over-settlement: the cash physically arrived, so the caller books the unapplied
    /// remainder (`requested − applied`) as an on-account party credit (already sitting on the A/R
    /// control from the settlement post). This keeps the GL A/R and the billing subledger in
    /// agreement even when two payments race the same invoice. Draws outstanding down, advances
    /// payment schedules fill-in-order, flips status → `partially_paid` / `paid`. One transaction,
    /// `FOR UPDATE` on the invoice + its schedules.
    pub async fn apply_settlement(&self, invoice_ref: Uuid, kind: &str, amount: Decimal) -> Result<Decimal, BillingError> {
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): this seam carries no company argument — it is driven by payment's
        // `PaymentSettled` via an ACL. The CALLER must establish the scope
        // (`with_company_scope(Some(event.company_id))`); we bind whatever it set onto our tx. With no
        // ambient scope the drawdown fails closed (InvoiceNotFound) rather than touching another
        // company's invoice.
        company_scope::bind_current_company(&mut tx).await?;
        let applied = self.apply_settlement_in_tx(&mut tx, invoice_ref, kind, amount).await?;
        tx.commit().await?;
        Ok(applied)
    }

    /// **Go-live exactly-once settlement consumer.** Apply every allocation of one `PaymentSettled`
    /// under a single inbox dedup on the bus `event_id`, all in one transaction — so an at-least-once
    /// redelivery of the event is a no-op (the drawdown already committed with the dedup mark). This is
    /// the redelivery-safe entry the payment council parked; the relay calls it with the event's id.
    /// Returns the total amount applied (0 on a redelivery). Requires `backbone_outbox::outbox::migrate`
    /// to have created `billing.inbox_consumed`.
    pub async fn apply_settlements_once(
        &self,
        event_id: Uuid,
        consumer: &str,
        allocations: &[(Uuid, String, Decimal)],
    ) -> Result<Decimal, BillingError> {
        let mut tx = self.db_pool.begin().await?;
        // RLS scope: as `apply_settlement` — the relay/ACL must wrap this in the event's company scope.
        company_scope::bind_current_company(&mut tx).await?;
        let first = backbone_outbox::inbox::once(&mut *tx, "billing", consumer, event_id)
            .await
            .map_err(|e| BillingError::Db(sqlx::Error::Protocol(e.to_string())))?;
        if !first {
            tx.commit().await?; // already consumed — exactly-once no-op
            return Ok(Decimal::ZERO);
        }
        let mut total = Decimal::ZERO;
        for (invoice_ref, kind, amount) in allocations {
            total += self.apply_settlement_in_tx(&mut tx, *invoice_ref, kind, *amount).await?;
        }
        tx.commit().await?;
        Ok(total)
    }

    /// The drawdown core, on a caller-supplied transaction (shared by `apply_settlement` and the
    /// exactly-once `apply_settlements_once`). Clamps to what is owed; the remainder is on-account.
    async fn apply_settlement_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        invoice_ref: Uuid,
        kind: &str,
        amount: Decimal,
    ) -> Result<Decimal, BillingError> {
        let ikind = invoice_kind(kind)?;
        if amount < Decimal::ZERO { return Err(BillingError::NegativeAmount); }
        let amount = money(amount);
        if amount.is_zero() { return Ok(Decimal::ZERO); }

        // Lock the invoice FIRST, then draw the schedules down — this order serializes concurrent
        // settlements of the same invoice.
        let row = self.settlement.lock_outstanding(&mut **tx, ikind, invoice_ref).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_ref))?;
        let outstanding = row.outstanding_amount;
        // Clamp to what is owed; the remainder is the caller's on-account credit.
        let applied = if amount < outstanding { amount } else { outstanding };
        if applied.is_zero() {
            return Ok(Decimal::ZERO);
        }
        let new_out = outstanding - applied;
        let new_status = if new_out.is_zero() { "paid" } else { "partially_paid" };
        self.settlement.update_settlement(&mut **tx, ikind, invoice_ref, new_out, new_status).await?;

        // Draw down payment schedules fill-in-order (earliest installment first) by the applied amount.
        let scheds = self.schedules.lock_schedules_fill_order(&mut **tx, invoice_ref, kind).await?;
        let mut remaining = applied;
        for s in &scheds {
            if remaining.is_zero() { break; }
            let capacity = s.amount - s.paid_amount;
            if capacity <= Decimal::ZERO { continue; }
            let take = if capacity < remaining { capacity } else { remaining };
            let new_paid = s.paid_amount + take;
            let sstatus = if new_paid >= s.amount { "paid" } else { "partially_paid" };
            self.schedules.update_paid(&mut **tx, s.id, new_paid, sstatus).await?;
            remaining -= take;
        }
        Ok(applied)
    }

    /// Reverse a settlement against an invoice (driven by payment's `PaymentCancelled` via an ACL) —
    /// the paired mirror of `apply_settlement` (council 2026-07-05). **Restores** `outstanding_amount`
    /// by `restored = min(amount, grand_total − outstanding)` (never above the invoice total — the
    /// bound that makes a repeat/redelivered reverse safe), rewinds payment schedules last-installment
    /// first (the inverse of fill-in-order), and flips status → `partially_paid` / `submitted`
    /// (re-owed). Without this, a refunded/bounced payment would leave the invoice permanently `paid`.
    /// Returns the amount actually restored. One transaction, `FOR UPDATE`.
    pub async fn reverse_settlement(&self, invoice_ref: Uuid, kind: &str, amount: Decimal) -> Result<Decimal, BillingError> {
        let ikind = invoice_kind(kind)?;
        if amount < Decimal::ZERO { return Err(BillingError::NegativeAmount); }
        let amount = money(amount);
        if amount.is_zero() { return Ok(Decimal::ZERO); }

        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): as `apply_settlement` — driven by payment's `PaymentCancelled` via an
        // ACL and carries no company argument, so the CALLER must establish the scope
        // (`with_company_scope(Some(event.company_id))`). Fails closed without one.
        company_scope::bind_current_company(&mut tx).await?;
        let row = self.settlement.lock_outstanding(&mut tx, ikind, invoice_ref).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_ref))?;
        let outstanding = row.outstanding_amount;
        let grand = row.grand_total;
        // Restore at most what was actually settled — never push outstanding above the invoice total.
        let headroom = grand - outstanding;
        let restored = if amount < headroom { amount } else { headroom };
        if restored.is_zero() {
            tx.commit().await?;
            return Ok(Decimal::ZERO);
        }
        let new_out = outstanding + restored;
        let new_status = if new_out >= grand { "submitted" } else { "partially_paid" };
        self.settlement.update_settlement(&mut tx, ikind, invoice_ref, new_out, new_status).await?;

        // Rewind schedules last-installment first (inverse of the fill-in-order drawdown).
        let scheds = self.schedules.lock_schedules_rewind_order(&mut tx, invoice_ref, kind).await?;
        let mut remaining = restored;
        for s in &scheds {
            if remaining.is_zero() { break; }
            if s.paid_amount <= Decimal::ZERO { continue; }
            let take = if s.paid_amount < remaining { s.paid_amount } else { remaining };
            let new_paid = s.paid_amount - take;
            let sstatus = if new_paid.is_zero() { "unpaid" } else if new_paid >= s.amount { "paid" } else { "partially_paid" };
            self.schedules.update_paid(&mut tx, s.id, new_paid, sstatus).await?;
            remaining -= take;
        }
        tx.commit().await?;
        Ok(restored)
    }

    /// Credit-note a posted Sales Invoice — REVERSE the revenue recognition (council 2026-07-05, the
    /// POS returns path). Posts the sign-flipped AR journal (`Dr Revenue · Cr A/R [customer]`,
    /// `posting_type="reversal"`, linked via `reverses_post_id`), sets `outstanding_amount=0`, flips the
    /// invoice → `cancelled`, and emits `InvoiceCancelled`. Distinct from `reverse_settlement` (which
    /// only restores the settlement's outstanding) — this undoes the revenue itself, so a full return
    /// (credit note + payment refund) nets revenue, cash, and A/R to zero. Idempotent: a re-credit
    /// short-circuits on `status='cancelled'`.
    pub async fn reverse_sales_invoice(&self, invoice_id: Uuid, sink: &dyn GlPostSink) -> Result<PostOutcome, BillingError> {
        // RLS scope (ADR-0008), ID-only: fenced by the request/inherited scope.
        let orig_post: Option<Uuid> = self.sales.fetch_accounting_post_id(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        // The forward revenue post, sign-flipped, is the credit note.
        let fwd = self.build_ar_post(invoice_id).await?;
        let lines = fwd.lines.iter().map(|l| GlPostLine {
            account_id: l.account_id, debit: l.credit, credit: l.debit,
            party_type: l.party_type.clone(), party_id: l.party_id,
            description: l.description.as_ref().map(|d| format!("Credit note: {d}")),
        }).collect();
        let env = AccountingPostEnvelope {
            idempotency_key: format!("reversal:{invoice_id}"), posting_type: "reversal".into(),
            reverses_post_id: orig_post, lines,
            description: fwd.description.map(|d| format!("Credit note: {d}")),
            ..fwd
        };
        if !env.is_balanced() { return Err(BillingError::UnbalancedPost); }
        match sink.post(&env).await {
            Ok(ack) => {
                let affected = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.sales.mark_cancelled(&self.db_pool, invoice_id),
                ).await?;
                if affected == 1 {
                    self.sink.publish(BillingEvent::InvoiceCancelled(InvoiceCancelled { invoice_id, kind: "sales".into() }));
                }
                Ok(PostOutcome { invoice_id, post_id: ack.post_id, journal_id: ack.journal_id, idempotent_reuse: ack.idempotent_reuse })
            }
            Err(rej) => Err(BillingError::GlRejected { code: rej.code, message: rej.message }),
        }
    }

    // ---- shared -------------------------------------------------------------

    async fn short_circuit_sales_posted(&self, invoice_id: Uuid) -> Result<Option<PostOutcome>, BillingError> {
        let row = self.sales.fetch_posting_state(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        Ok(posted_outcome(invoice_id, row))
    }

    async fn short_circuit_purchase_posted(&self, invoice_id: Uuid) -> Result<Option<PostOutcome>, BillingError> {
        let row = self.purchases.fetch_posting_state(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        Ok(posted_outcome(invoice_id, row))
    }
}

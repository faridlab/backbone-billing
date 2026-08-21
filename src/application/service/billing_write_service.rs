//! Validated write path + AR/AP posting engine for billing (hand-authored, user-owned).
//!
//! Region-neutral: invoices carry NO tax columns; tax lines live in the removable `InvoiceTaxLine`
//! overlay. Two population paths: caller-supplied lines (billing computes nothing itself), or —
//! when an invoice line carries a `tax_template_id` and the service is wired with
//! `with_tax_engine` — the document's tax is computed by `backbone-tax`'s document engine
//! (rounding policy, repartition routing, cash-basis deferral) and the caller's tax lines are
//! ignored. Under `round_globally` the engine's redistributed per-line nets OVERWRITE the
//! caller's own per-line money() totals (see `price_document`) or the journal mis-balances.
//! On post, billing assembles the net lines + overlay tax into ONE balanced `AccountingPost`:
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

use crate::infrastructure::persistence::{
    InvoiceSettlementRepository, InvoiceTaxLineRepository, NewInvoiceTaxLineRow,
    PaymentScheduleRepository, PaymentTermRepository, PostingStateRow,
    PurchaseInvoiceLineRepository, PurchaseInvoiceRepository, SalesInvoiceLineRepository,
    SalesInvoiceRepository,
};

use super::billing_events::{BillingEventSink, LoggingSink};

pub(super) fn money(v: Decimal) -> Decimal {
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
    /// Tax template (backbone-tax). When ANY line carries one, the create path computes
    /// the whole document's tax through the engine (caller-supplied tax lines are ignored
    /// for that document) and fails closed with `tax_engine_unwired` if no engine is wired.
    pub tax_template_id: Option<Uuid>,
}

/// A tax line for the overlay. `basis`: "output" | "input" | "withholding". The routing
/// fields are `None`/zero for caller-supplied lines; engine-computed lines carry the
/// template + repartition split that produced them, and `real_account_id` is `Some` iff
/// the amount is cash-basis deferred (`exigibility` "on_payment") — posting goes to the
/// transition `account_id`, the flip to the real account is the reconciliation seam's job.
#[derive(Debug, Clone)]
pub struct NewTaxLine {
    pub account_id: Uuid,
    pub basis: String,
    pub description: Option<String>,
    pub rate: Decimal,
    pub tax_amount: Decimal,
    /// The base the tax was computed on (0 for caller-supplied lines — billing does not
    /// compute it; the engine path sets the source line's post-policy net).
    pub taxable_base: Decimal,
    pub tax_template_id: Option<Uuid>,
    pub repartition_line_id: Option<Uuid>,
    pub real_account_id: Option<Uuid>,
    /// "on_invoice" (the overlay default) | "on_payment".
    pub exigibility: Option<String>,
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
    /// Payment term applied; when set, the post verb derives due date + installments + the
    /// early-pay-discount block (a manual `due_date` alongside a term is refused).
    pub payment_term_id: Option<Uuid>,
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
    /// Payment term applied; when set, the post verb derives due date + installments + the
    /// early-pay-discount block (a manual `due_date` alongside a term is refused).
    pub payment_term_id: Option<Uuid>,
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
    GlRejected {
        code: String,
        message: String,
    },
    /// The reconciliation-graph sink refused the settlement's edge (guard violation, unposted
    /// journal, or a clamp disagreement between the subledger and the ledger). Settlements are
    /// fail-closed on this: the drawdown rolls back with the refused edge.
    ReconcileRefused {
        code: String,
        message: String,
    },
    /// A line carried a tax template but no tax engine is wired (`with_tax_engine` not called).
    /// Fail closed: never silently fall back to un-taxed totals on a template-driven document.
    TaxEngineUnwired,
    /// The tax engine refused or failed the document computation (bad template, no effective
    /// rate, invalid repartition, missing company scope…).
    TaxCompute {
        code: String,
        message: String,
    },
    /// A payment term was not found (unknown id, retired, or another company's — the split-fence
    /// read makes cross-tenant terms indistinguishable from missing ones).
    TermNotFound(Uuid),
    /// A payment term or its invoice application is shape-invalid (validation codes:
    /// `term_name_required`, `term_lines_required`, `discount_tax_basis_unsupported`,
    /// `discount_percent_out_of_range`, `discount_days_out_of_range`, `discount_account_required`,
    /// `term_multiple_balance`, `term_balance_not_last`, `term_percent_out_of_range`,
    /// `term_fixed_out_of_range`, `term_value_invalid`, `term_days_negative`,
    /// `term_day_of_month_required`, `term_delay_invalid`, `term_percent_exceeds_total`,
    /// `term_exceeds_total`, `term_does_not_cover_total`, `term_inactive`,
    /// `due_date_conflicts_with_term`, `schedule_conflicts_with_term`).
    TermInvalid {
        code: String,
        message: String,
    },
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
            BillingError::ReconcileRefused { code, .. } => code.clone(),
            BillingError::TaxEngineUnwired => "tax_engine_unwired".into(),
            BillingError::TaxCompute { code, .. } => code.clone(),
            BillingError::TermNotFound(_) => "term_not_found".into(),
            BillingError::TermInvalid { code, .. } => code.clone(),
            BillingError::Db(_) => "internal_error".into(),
        }
    }
    pub fn http_status(&self) -> u16 {
        match self {
            BillingError::InvoiceNotFound(_) | BillingError::TermNotFound(_) => 404,
            BillingError::Db(_) => 500,
            _ => 422,
        }
    }
}
impl std::fmt::Display for BillingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BillingError::GlRejected { code, message } => write!(f, "{code}: {message}"),
            BillingError::ReconcileRefused { code, message } => write!(f, "{code}: {message}"),
            BillingError::TaxCompute { code, message } => write!(f, "{code}: {message}"),
            BillingError::TermInvalid { code, message } => write!(f, "{code}: {message}"),
            other => write!(f, "{}", other.code()),
        }
    }
}
impl std::error::Error for BillingError {}
impl From<sqlx::Error> for BillingError {
    fn from(e: sqlx::Error) -> Self {
        BillingError::Db(e)
    }
}
pub(super) fn is_dup(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .map(|d| d.is_unique_violation())
        .unwrap_or(false)
}

pub(super) struct PricedLine {
    pub(super) item_id: Uuid,
    pub(super) account_id: Uuid,
    pub(super) description: Option<String>,
    pub(super) quantity: Decimal,
    pub(super) unit_price: Decimal,
    pub(super) net_amount: Decimal,
}

/// Price lines (`net = money(qty·price)`), reject empty/negative, and total the supplied tax lines
/// by basis. Returns (priced, net_total, output, input, withholding).
pub(super) fn price(
    lines: &[NewInvoiceLine],
    tax: &[NewTaxLine],
) -> Result<(Vec<PricedLine>, Decimal, Decimal, Decimal, Decimal), BillingError> {
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
            item_id: l.item_id,
            account_id: l.account_id,
            description: l.description.clone(),
            quantity: l.quantity,
            unit_price: l.unit_price,
            net_amount: net,
        });
    }
    let mut output = Decimal::ZERO;
    let mut input = Decimal::ZERO;
    let mut wht = Decimal::ZERO;
    for t in tax {
        if t.tax_amount < Decimal::ZERO {
            return Err(BillingError::NegativeAmount);
        }
        match t.basis.as_str() {
            "output" => output += t.tax_amount,
            "input" => input += t.tax_amount,
            "withholding" => wht += t.tax_amount,
            _ => {}
        }
    }
    Ok((
        priced,
        money(net_total),
        money(output),
        money(input),
        money(wht),
    ))
}

/// The fully-priced document both create verbs consume: validated lines with FINAL nets,
/// the overlay tax lines to persist, and the basis totals the header columns carry.
pub(super) struct PricedDocument {
    pub(super) priced: Vec<PricedLine>,
    pub(super) net_total: Decimal,
    pub(super) tax_lines: Vec<NewTaxLine>,
    pub(super) output: Decimal,
    pub(super) input: Decimal,
    pub(super) withholding: Decimal,
}

/// Sum a tax-line set by basis ("output" | "input" | "withholding"), each rounded once.
pub(super) fn sum_by_basis(tax: &[NewTaxLine]) -> (Decimal, Decimal, Decimal) {
    let mut output = Decimal::ZERO;
    let mut input = Decimal::ZERO;
    let mut wht = Decimal::ZERO;
    for t in tax {
        match t.basis.as_str() {
            "output" => output += t.tax_amount,
            "input" => input += t.tax_amount,
            "withholding" => wht += t.tax_amount,
            _ => {}
        }
    }
    (money(output), money(input), money(wht))
}

/// Reconcile a `PostOutcome` from an invoice's persisted posting state — the shared tail of the
/// posted short-circuit, for both invoice kinds.
pub(super) fn posted_outcome(invoice_id: Uuid, row: PostingStateRow) -> Option<PostOutcome> {
    if row.posting_state == "posted" {
        if let (Some(j), Some(p)) = (row.journal_id, row.accounting_post_id) {
            return Some(PostOutcome {
                invoice_id,
                post_id: p,
                journal_id: j,
                idempotent_reuse: true,
            });
        }
    }
    None
}

#[derive(Clone)]
pub struct BillingWriteService {
    pub(super) db_pool: PgPool,
    pub(super) sink: Arc<dyn BillingEventSink>,
    pub(super) sales: Arc<SalesInvoiceRepository>,
    pub(super) sales_lines: Arc<SalesInvoiceLineRepository>,
    pub(super) purchases: Arc<PurchaseInvoiceRepository>,
    pub(super) purchase_lines: Arc<PurchaseInvoiceLineRepository>,
    pub(super) tax_lines: Arc<InvoiceTaxLineRepository>,
    pub(super) schedules: Arc<PaymentScheduleRepository>,
    pub(super) settlement: Arc<InvoiceSettlementRepository>,
    /// Payment-terms master (header + lines). Owns the split-fence reads the post hook and the
    /// discount resolver need.
    pub(super) terms: Arc<PaymentTermRepository>,
    /// When set, `post_*_invoice` / `reverse_sales_invoice` stage the seam event into
    /// `<schema>.outbox_events` **inside the state-transition transaction** (crash-safe emission —
    /// the go-live durable bus, mirroring `backbone-payment`'s outbox fence). When `None`, only the
    /// legacy in-proc sink fires. Requires `backbone_outbox::outbox::migrate` to have created the table.
    pub(super) outbox_schema: Option<String>,
    /// Document-grade tax engine (backbone-tax). When `None`, any invoice line carrying a
    /// `tax_template_id` fails closed (`tax_engine_unwired`) — a template-driven document must
    /// never silently fall back to un-taxed totals.
    pub(super) tax_engine: Option<Arc<backbone_tax::TaxEngine>>,
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
            terms: Arc::new(PaymentTermRepository::new(db_pool.clone())),
            db_pool,
            sink,
            outbox_schema: None,
            tax_engine: None,
        }
    }

    /// Enable crash-safe seam-event emission via the durable outbox in `schema` (e.g. `"billing"`),
    /// staged inside the posted/cancelled transition transaction — mirrors
    /// `backbone-payment::PaymentWriteService::with_outbox_schema`. The relay later drains the outbox
    /// to the real bus; consumers dedup via `backbone_outbox::inbox::once`. Requires
    /// `backbone_outbox::outbox::migrate(&pool, schema)` to have created `<schema>.outbox_events`.
    pub fn with_outbox_schema(mut self, schema: impl Into<String>) -> Self {
        self.outbox_schema = Some(schema.into());
        self
    }

    /// Wire the document-grade tax engine (backbone-tax) used when an invoice line carries a
    /// `tax_template_id`. Without it, template-driven creates fail closed (`tax_engine_unwired`);
    /// documents whose lines carry no template never touch the engine.
    pub fn with_tax_engine(mut self, engine: Arc<backbone_tax::TaxEngine>) -> Self {
        self.tax_engine = Some(engine);
        self
    }

    pub(super) async fn insert_tax_lines(
        &self,
        tx: &mut sqlx::PgConnection,
        invoice_id: Uuid,
        company_id: Uuid,
        kind: &str,
        tax: &[NewTaxLine],
    ) -> Result<(), BillingError> {
        for t in tax {
            self.tax_lines
                .insert_tax_line(
                    &mut *tx,
                    &NewInvoiceTaxLineRow {
                        id: Uuid::new_v4(),
                        invoice_ref: invoice_id,
                        kind,
                        company_id,
                        account_id: t.account_id,
                        basis: &t.basis,
                        description: t.description.as_deref(),
                        rate: t.rate,
                        tax_amount: money(t.tax_amount),
                        taxable_base: money(t.taxable_base),
                        tax_template_id: t.tax_template_id,
                        repartition_line_id: t.repartition_line_id,
                        real_account_id: t.real_account_id,
                        exigibility: t.exigibility.as_deref(),
                    },
                )
                .await?;
        }
        Ok(())
    }

    /// Price a document for creation — the shared front of both create verbs.
    ///
    /// When ANY line carries a `tax_template_id`, the document is template-driven: its whole tax
    /// overlay comes from `backbone-tax`'s document engine (rounding policy, repartition routing,
    /// cash-basis deferral) and the caller's supplied tax lines are IGNORED for that document —
    /// two computation sources on one document would double-count. Fails closed with
    /// `tax_engine_unwired` when no engine is wired. Un-Templated documents keep the supplied
    /// lines verbatim (the pre-engine behavior).
    ///
    /// **Net overwrite (load-bearing):** under `round_globally` the engine redistributes the
    /// per-line nets so they sum to the rounded document total; those nets OVERWRITE the
    /// per-line `money()` totals here. The overlay lines and the revenue legs must journal
    /// against exactly these nets or the document mis-balances (e.g. nets 17.80+17.79 with tax
    /// 3.74+3.73 ⇒ A/R 43.06 exactly — the per-line money() totals 17.80+17.80 would not).
    pub(super) async fn price_document(
        &self,
        lines: &[NewInvoiceLine],
        supplied: &[NewTaxLine],
        company_id: Uuid,
        on_date: chrono::NaiveDate,
        kind: &str,
    ) -> Result<PricedDocument, BillingError> {
        // price() validates (non-empty, non-negative) and computes the per-line money() nets —
        // the fallback totals for un-templated documents and the pre-overwrite value otherwise.
        let (mut priced, _net_total, _, _, _) = price(lines, supplied)?;
        if !lines.iter().any(|l| l.tax_template_id.is_some()) {
            let (output, input, withholding) = sum_by_basis(supplied);
            let net_total = money(priced.iter().map(|p| p.net_amount).sum());
            return Ok(PricedDocument {
                priced,
                net_total,
                tax_lines: supplied.to_vec(),
                output,
                input,
                withholding,
            });
        }

        let engine = self
            .tax_engine
            .clone()
            .ok_or(BillingError::TaxEngineUnwired)?;
        let req_lines: Vec<backbone_tax::DocumentTaxRequestLine> = lines
            .iter()
            .filter_map(|l| {
                l.tax_template_id
                    .map(|template_id| backbone_tax::DocumentTaxRequestLine {
                        template_id,
                        quantity: l.quantity,
                        unit_price: l.unit_price,
                    })
            })
            .collect();
        let req = backbone_tax::DocumentTaxRequest {
            company_id,
            // Both sales and purchase invoices route through the invoice repartition family;
            // refunds are credit notes (wholesale sign flip at reversal), not this path.
            document_type: backbone_tax::DocumentType::Invoice,
            on_date,
            lines: req_lines,
        };
        let result =
            company_scope::with_company_scope(Some(company_id), engine.calculate_document(&req))
                .await
                .map_err(|e: backbone_tax::TaxError| BillingError::TaxCompute {
                    code: e.code().to_string(),
                    message: e.to_string(),
                })?;

        // Overwrite the templated lines' nets with the engine's post-policy nets
        // (`net_amounts` is indexed over the templated subset, in input order).
        let mut k = 0usize;
        for (i, l) in lines.iter().enumerate() {
            if l.tax_template_id.is_some() {
                priced[i].net_amount = result.net_amounts[k];
                k += 1;
            }
        }
        let net_total = money(priced.iter().map(|p| p.net_amount).sum());

        let tax_lines: Vec<NewTaxLine> = result.lines.iter().map(|tl| {
            // Withholding components arrive signed negative (a deduction); the overlay stores
            // face amounts and `price`-style basis sums handle the deduction at totaling time.
            let basis = if tl.is_withholding { "withholding" }
                else if kind == "sales" { "output" } else { "input" };
            Ok(NewTaxLine {
                account_id: tl.account_id.ok_or_else(|| BillingError::TaxCompute {
                    code: "tax_line_unrouted".into(),
                    message: format!(
                        "template {} produced a tax split with no routing account (repartition tax lines need an account)",
                        tl.template_id,
                    ),
                })?,
                basis: basis.into(),
                description: tl.description.clone(),
                rate: tl.rate,
                tax_amount: tl.tax_amount.abs(),
                taxable_base: result.net_amounts.get(tl.source_index).copied().unwrap_or(Decimal::ZERO),
                tax_template_id: Some(tl.template_id),
                repartition_line_id: tl.repartition_line_id,
                real_account_id: tl.real_account_id,
                exigibility: Some(if tl.real_account_id.is_some() { "on_payment" } else { "on_invoice" }.into()),
            })
        }).collect::<Result<Vec<_>, BillingError>>()?;
        let (output, input, withholding) = sum_by_basis(&tax_lines);
        Ok(PricedDocument {
            priced,
            net_total,
            tax_lines,
            output,
            input,
            withholding,
        })
    }

    /// Stage a seam event into the durable outbox on the SAME transaction as the state transition —
    /// the crash-safe fence (mirrors `backbone-payment::stage_settled`). The event is serialized into
    /// the outbox payload so a relay can deliver it to any consumer (buying, tax, selling); billing's
    /// own in-proc sink still fires after commit for same-process consumers/tests. The caller has
    /// already bound the company scope onto `conn` (RLS), so this just executes on it.
    pub(super) async fn stage_outbox_event<E: serde::Serialize>(
        &self,
        conn: &mut sqlx::PgConnection,
        schema: &str,
        event_type: &str,
        aggregate_type: &str,
        aggregate_id: Uuid,
        company_id: Uuid,
        event: &E,
    ) -> Result<(), BillingError> {
        let payload = serde_json::to_value(event).map_err(|e| {
            BillingError::Db(sqlx::Error::Protocol(format!("outbox serialize: {e}")))
        })?;
        // OutboxRecord::new requires the owning tenant (ADR-0011 — the outbox_events table is fenced
        // by company_id). The caller passes the event's company explicitly.
        let rec = backbone_outbox::OutboxRecord::new(
            event_type,
            aggregate_type,
            aggregate_id.to_string(),
            company_id,
            payload,
            chrono::Utc::now(),
        );
        backbone_outbox::outbox::stage(&mut *conn, schema, &rec)
            .await
            .map_err(|e| BillingError::Db(sqlx::Error::Protocol(e.to_string())))?;
        Ok(())
    }
}

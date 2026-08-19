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

use crate::infrastructure::persistence::{
    InvoiceSettlementRepository, InvoiceTaxLineRepository, NewInvoiceTaxLineRow,
    PaymentScheduleRepository, PostingStateRow, PurchaseInvoiceLineRepository,
    PurchaseInvoiceRepository, SalesInvoiceLineRepository, SalesInvoiceRepository,
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
    /// The reconciliation-graph sink refused the settlement's edge (guard violation, unposted
    /// journal, or a clamp disagreement between the subledger and the ledger). Settlements are
    /// fail-closed on this: the drawdown rolls back with the refused edge.
    ReconcileRefused { code: String, message: String },
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
            BillingError::ReconcileRefused { code, message } => write!(f, "{code}: {message}"),
            other => write!(f, "{}", other.code()),
        }
    }
}
impl std::error::Error for BillingError {}
impl From<sqlx::Error> for BillingError {
    fn from(e: sqlx::Error) -> Self { BillingError::Db(e) }
}
pub(super) fn is_dup(e: &sqlx::Error) -> bool {
    e.as_database_error().map(|d| d.is_unique_violation()).unwrap_or(false)
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
pub(super) fn price(lines: &[NewInvoiceLine], tax: &[NewTaxLine]) -> Result<(Vec<PricedLine>, Decimal, Decimal, Decimal, Decimal), BillingError> {
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
pub(super) fn posted_outcome(invoice_id: Uuid, row: PostingStateRow) -> Option<PostOutcome> {
    if row.posting_state == "posted" {
        if let (Some(j), Some(p)) = (row.journal_id, row.accounting_post_id) {
            return Some(PostOutcome { invoice_id, post_id: p, journal_id: j, idempotent_reuse: true });
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
    /// When set, `post_*_invoice` / `reverse_sales_invoice` stage the seam event into
    /// `<schema>.outbox_events` **inside the state-transition transaction** (crash-safe emission —
    /// the go-live durable bus, mirroring `backbone-payment`'s outbox fence). When `None`, only the
    /// legacy in-proc sink fires. Requires `backbone_outbox::outbox::migrate` to have created the table.
    pub(super) outbox_schema: Option<String>,
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
            outbox_schema: None,
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

    pub(super) async fn insert_tax_lines(&self, tx: &mut sqlx::PgConnection, invoice_id: Uuid, company_id: Uuid, kind: &str, tax: &[NewTaxLine]) -> Result<(), BillingError> {
        for t in tax {
            self.tax_lines.insert_tax_line(&mut *tx, &NewInvoiceTaxLineRow {
                id: Uuid::new_v4(),
                invoice_ref: invoice_id,
                kind,
                company_id,
                account_id: t.account_id,
                basis: &t.basis,
                description: t.description.as_deref(),
                rate: t.rate,
                tax_amount: money(t.tax_amount),
            }).await?;
        }
        Ok(())
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
        let payload = serde_json::to_value(event)
            .map_err(|e| BillingError::Db(sqlx::Error::Protocol(format!("outbox serialize: {e}"))))?;
        // OutboxRecord::new requires the owning tenant (ADR-0011 — the outbox_events table is fenced
        // by company_id). The caller passes the event's company explicitly.
        let rec = backbone_outbox::OutboxRecord::new(
            event_type, aggregate_type, aggregate_id.to_string(), company_id, payload, chrono::Utc::now(),
        );
        backbone_outbox::outbox::stage(&mut *conn, schema, &rec)
            .await
            .map_err(|e| BillingError::Db(sqlx::Error::Protocol(e.to_string())))?;
        Ok(())
    }
}

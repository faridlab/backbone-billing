//! Billing domain events (hand-authored, user-owned) — the public extension surface.
//!
//! `SalesInvoicePosted` lets consumers (e.g. the tax faktur-pajak overlay, loyalty) react;
//! `PurchaseInvoicePosted` carries the billed lines so an ACL routes them to
//! `backbone-buying::mark_billed` — retiring buying's simulated billing leg.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A sales invoice's revenue was posted to the GL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SalesInvoicePosted {
    pub invoice_id: Uuid,
    pub company_id: Uuid,
    pub journal_id: Uuid,
    pub post_id: Uuid,
    pub source_so_id: Option<Uuid>,
    /// The billed lines (item + qty) — what a downstream SO's `billed_qty` advances by (the
    /// order-to-cash mirror of `PurchaseInvoicePosted.billed_lines`).
    pub billed_lines: Vec<BilledLine>,
    pub grand_total: Decimal,
    /// Tax-overlay fields for the tax faktur-pajak consumer
    /// (`backbone-tax::EFakturService::record_tax_transaction`): `taxable_base` = invoice net,
    /// `output_total` = PPN output (the sales `tax_total`). The ACL maps these into `PostedForTax`.
    pub posting_date: NaiveDate,
    pub taxable_base: Decimal,
    pub output_total: Decimal,
}

/// One billed line (item + quantity) — what a downstream PO's billed_qty advances by.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BilledLine {
    pub item_id: Uuid,
    pub quantity: Decimal,
}

/// A purchase invoice's A/P was posted. Carries `source_po_id` + billed lines so a composition
/// layer advances the source PO's billed_qty (`buying::mark_billed`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PurchaseInvoicePosted {
    pub invoice_id: Uuid,
    pub company_id: Uuid,
    pub journal_id: Uuid,
    pub post_id: Uuid,
    pub source_po_id: Option<Uuid>,
    pub billed_lines: Vec<BilledLine>,
    pub grand_total: Decimal,
    /// Tax-overlay fields for the tax consumer: `taxable_base` = invoice net, `input_total` =
    /// PPN input (the purchase `tax_total`), `withholding_total` = PPh withheld.
    pub posting_date: NaiveDate,
    pub taxable_base: Decimal,
    pub input_total: Decimal,
    pub withholding_total: Decimal,
}

/// An invoice was cancelled (a reversal post is emitted separately). `company_id` lets a
/// consumer bound its RLS scope from the payload alone — the other posted events carry it for
/// the same reason.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvoiceCancelled {
    pub invoice_id: Uuid,
    pub company_id: Uuid,
    pub kind: String, // "sales" | "purchase"
}

/// The billing domain-event union.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BillingEvent {
    SalesInvoicePosted(SalesInvoicePosted),
    PurchaseInvoicePosted(PurchaseInvoicePosted),
    InvoiceCancelled(InvoiceCancelled),
}

/// Sink for billing domain events. Fire-and-forget; a real adapter wires a bus, tests record.
pub trait BillingEventSink: Send + Sync {
    fn publish(&self, event: BillingEvent);
}

/// Default sink — emits structured tracing events.
pub struct LoggingSink;

impl BillingEventSink for LoggingSink {
    fn publish(&self, event: BillingEvent) {
        tracing::info!(target: "billing.events", ?event, "billing domain event");
    }
}

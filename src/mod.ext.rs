// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

pub mod request_pool;
// The AR/AP seam contract — re-exported at the crate root so sibling modules depend on
// `backbone_billing::{PurchaseInvoicePosted, SalesInvoicePosted, InvoiceCancelled, ...}` rather
// than reaching into `application::service::billing_events` (council 2026-07-26, rec #1).
//
// NOTE: `billing::exports::BillingEvent` is a generated CRUD-lifecycle ghost (18 variants) that the
// write path NEVER emits; it cannot be removed without a framework codegen change (the `export`
// generator hardcodes it at metaphor-plugin-schema/.../generators/export.rs). The REAL domain-event
// union is `DomainBillingEvent` below — prefer it and the individual structs over the ghost.
pub use application::service::billing_events::BillingEvent as DomainBillingEvent;
pub use application::service::billing_events::{
    BilledLine, BillingEventSink, InvoiceCancelled, LoggingSink, PurchaseInvoicePosted,
    SalesInvoicePosted,
};
pub use application::service::SettlementOutcome;

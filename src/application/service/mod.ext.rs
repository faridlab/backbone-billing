// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

pub mod billing_events;
pub mod billing_gl;
pub mod billing_write_service;
// Focused siblings of `billing_write_service` — one `BillingWriteService` inherent impl, split by
// concern (A/R ledger, A/P ledger, the cash loop). Each is declared `user_owned`.
pub mod billing_purchase;
pub mod billing_sales;
pub mod billing_settlement;
pub mod settlement_consumer;
// Payment-terms master: pure schedule derivation + the invoice-post materialization hook.
pub mod term_schedule;
pub use billing_events::{
    BilledLine, BillingEvent, BillingEventSink, InvoiceCancelled, LoggingSink,
    PurchaseInvoicePosted, SalesInvoicePosted,
};
pub use billing_gl::{
    AccountingPostEnvelope, GlPostAck, GlPostLine, GlPostRejected, GlPostSink, ReconcileEdgeAck,
    ReconcileLine, ReconcileOrigin, ReconcilePairRequest, ReconcileRejected, ReconcileSink,
    UnreconcilePairRequest,
};
pub use billing_settlement::{EarlyPayDiscount, SettlementOutcome};
pub use billing_write_service::{
    BillingError, BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewSalesInvoice,
    NewTaxLine, PostOutcome,
};
pub use settlement_consumer::{
    PaymentCancelledDto, PaymentCancelledHandler, PaymentSettledDto, PaymentSettledHandler,
    SettledInvoiceDto,
};
pub use term_schedule::{derive_schedule, line_due_date, validate_term, NewTermLine};

// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

// The settlement drawdown's SQL — cross-table (sales OR purchase, by a runtime invoice kind), so it
// belongs to neither invoice repository. Declared `user_owned` in metaphor.codegen.yaml.
mod invoice_settlement_repository;
// The payment-terms master's SQL (header + lines + the invoice materialization writes) lives in
// `payment_term_repository` — same path the generator would use, declared `user_owned` so the
// generated section's `mod` declaration above resolves to the hand-written file.
// The hand-written billing SQL's parameter/projection types. Their repositories are all declared
// `user_owned` in metaphor.codegen.yaml.
// `parse_invoice_kind` resolves the settlement seam's wire string into the schema's own
// `domain::entity::InvoiceKind` — no second copy of that enum is defined here.
pub use invoice_settlement_repository::{
    parse_invoice_kind, InvoiceSettlementRepository, OutstandingRow, OverdueInvoiceRow,
};
pub use invoice_tax_line_repository::{NewInvoiceTaxLineRow, TaxAmountRow};
pub use payment_schedule_repository::{NewPaymentScheduleRow, ScheduleDrawdownRow};
pub use payment_term_repository::{
    InvoiceEpdRow, NewPaymentTermLineRow, NewPaymentTermRow, TermContextRow, TermHeaderRow,
    TermLineRow,
};
pub use purchase_invoice_line_repository::{ExpenseAmountRow, NewPurchaseInvoiceLineRow};
pub use purchase_invoice_repository::{ApHeaderRow, NewPurchaseInvoiceRow, PurchaseSeamHeaderRow};
pub use sales_invoice_line_repository::{BilledLineRow, NewSalesInvoiceLineRow, RevenueAmountRow};
pub use sales_invoice_repository::{
    ArHeaderRow, NewSalesInvoiceRow, PostingStateRow, SalesSeamHeaderRow,
};

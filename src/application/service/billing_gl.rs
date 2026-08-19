//! Outbound GL-posting + reconciliation ports (hand-authored, user-owned) — re-exports of the
//! shared contracts.
//!
//! The GL-posting wire types (`AccountingPostEnvelope`, `GlPostLine`, `GlPostAck`, `GlPostRejected`)
//! and the `GlPostSink` port live in the shared `backbone-gl-posting` crate (backbone-framework
//! v2.7.5) — the single source for all producers (phase 2). The reconciliation wire types
//! (`ReconcileLine`, `ReconcilePairRequest`, `ReconcileSink`, …) joined it on v2.7.9: the
//! settlement seam creates (and unlinks) reconciliation-graph edges between the invoice's
//! receivable/payable line and the payment's line, on the caller's transaction. This file
//! re-exports both under billing's existing paths so billing's write services, tests, and
//! `application::service::*` resolve unchanged. Billing is the AR/AP emitter (Sales Invoice
//! `Dr A/R · Cr Revenue · Cr PPN`; Purchase Invoice `Dr Expense · Dr PPN Input · Cr A/P · Cr PPh`);
//! the ACL maps the envelope into accounting's `PostingRequest`. Zero normal Cargo edge into
//! backbone-accounting.

pub use backbone_gl_posting::{
    AccountingPostEnvelope, GlPostAck, GlPostLine, GlPostRejected, GlPostSink, ReconcileEdgeAck,
    ReconcileLine, ReconcileOrigin, ReconcilePairRequest, ReconcileRejected, ReconcileSink,
    UnreconcilePairRequest,
};

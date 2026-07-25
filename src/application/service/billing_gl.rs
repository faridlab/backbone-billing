//! Outbound GL-posting port (hand-authored, user-owned) — re-export of the shared contract.
//!
//! The GL-posting wire types (`AccountingPostEnvelope`, `GlPostLine`, `GlPostAck`, `GlPostRejected`)
//! and the `GlPostSink` port now live in the shared `backbone-gl-posting` crate (backbone-framework
//! v2.7.5) — the single source for all producers (phase 2). This file re-exports them under billing's
//! existing paths so billing's write services, tests, and `application::service::*` resolve unchanged.
//! Billing is the AR/AP emitter (Sales Invoice `Dr A/R · Cr Revenue · Cr PPN`; Purchase Invoice
//! `Dr Expense · Dr PPN Input · Cr A/P · Cr PPh`); the ACL maps the envelope into accounting's
//! `PostingRequest`. Zero normal Cargo edge into backbone-accounting.

pub use backbone_gl_posting::{
    AccountingPostEnvelope, GlPostAck, GlPostLine, GlPostRejected, GlPostSink,
};

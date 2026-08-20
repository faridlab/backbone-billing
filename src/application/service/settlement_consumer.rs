//! Settlement event consumers — the relay-facing entry into the settlement seam.
//!
//! Hand-authored, user-owned. `backbone-payment` publishes `PaymentSettled` / `PaymentCancelled`
//! (its settlement/reversal events) onto the bus; the composing host's relay hands each to the
//! matching handler here. The event JSON is the contract — these DTOs are locally-shaped mirrors
//! of payment's payloads, so billing keeps ZERO normal Cargo edge into backbone-payment (the same
//! posture as the GL-posting seam; wire drift is caught by the seam tests, not by a compiler).
//!
//! Both handlers are exactly-once per bus `event_id`: they ride the inbox-deduped
//! `apply_settlements_once` / `reverse_settlements_once`, so an at-least-once redelivery is a
//! committed no-op. The host supplies the `ReconcileSink` (implemented over accounting's
//! reconciliation write service) — every applied settlement lands its graph edge in the same
//! transaction as the subledger drawdown.

use rust_decimal::Decimal;
use uuid::Uuid;

use super::billing_gl::ReconcileSink;
use super::billing_settlement::SettlementOutcome;
use super::billing_write_service::{BillingError, BillingWriteService};

/// One allocation of a payment event: which invoice, in which billing table, for how much.
/// Mirror of payment's `SettledInvoice` (the wire field names are the contract).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SettledInvoiceDto {
    pub invoice_ref: Uuid,
    /// "sales" | "purchase" — which billing invoice table `invoice_ref` points at.
    pub invoice_kind: String,
    pub amount: Decimal,
}

/// Mirror of payment's `PaymentSettled` (the wire field names are the contract). Only the fields
/// the settlement seam consumes are modeled — the host relay deserializes the bus envelope's
/// payload into this and passes the envelope's `event_id` alongside.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PaymentSettledDto {
    pub payment_id: Uuid,
    pub company_id: Uuid,
    pub payment_type: String,
    pub allocations: Vec<SettledInvoiceDto>,
    pub paid_amount: Decimal,
}

/// Mirror of payment's `PaymentCancelled` — the reversal event carrying the allocations undone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PaymentCancelledDto {
    pub payment_id: Uuid,
    pub company_id: Uuid,
    pub payment_type: String,
    pub allocations: Vec<SettledInvoiceDto>,
    pub paid_amount: Decimal,
}

/// The `PaymentSettled` consumer: apply every allocation exactly-once under the bus `event_id`,
/// creating the reconciliation edges in the same transaction as the drawdowns.
pub struct PaymentSettledHandler {
    billing: std::sync::Arc<BillingWriteService>,
    reconcile: std::sync::Arc<dyn ReconcileSink>,
    /// Inbox consumer name — one per relay route; separates dedup streams if a host ever runs more
    /// than one consumer over the same events.
    consumer: String,
}

impl PaymentSettledHandler {
    pub fn new(
        billing: std::sync::Arc<BillingWriteService>,
        reconcile: std::sync::Arc<dyn ReconcileSink>,
        consumer: impl Into<String>,
    ) -> Self {
        Self {
            billing,
            reconcile,
            consumer: consumer.into(),
        }
    }

    pub async fn handle(
        &self,
        event_id: Uuid,
        event: &PaymentSettledDto,
    ) -> Result<SettlementOutcome, BillingError> {
        let allocations: Vec<(Uuid, String, Decimal)> = event
            .allocations
            .iter()
            .map(|a| (a.invoice_ref, a.invoice_kind.clone(), a.amount))
            .collect();
        self.billing
            .apply_settlements_once(
                event_id,
                &self.consumer,
                event.company_id,
                event.payment_id,
                &allocations,
                self.reconcile.as_ref(),
            )
            .await
    }
}

/// The `PaymentCancelled` consumer: unlink every allocation's graph edge (side-effecting —
/// generated exchange moves are reversed) and restore the outstandings, exactly-once under the
/// bus `event_id`.
pub struct PaymentCancelledHandler {
    billing: std::sync::Arc<BillingWriteService>,
    reconcile: std::sync::Arc<dyn ReconcileSink>,
    consumer: String,
}

impl PaymentCancelledHandler {
    pub fn new(
        billing: std::sync::Arc<BillingWriteService>,
        reconcile: std::sync::Arc<dyn ReconcileSink>,
        consumer: impl Into<String>,
    ) -> Self {
        Self {
            billing,
            reconcile,
            consumer: consumer.into(),
        }
    }

    /// Returns the total amount restored (0 on a redelivery).
    pub async fn handle(
        &self,
        event_id: Uuid,
        event: &PaymentCancelledDto,
    ) -> Result<Decimal, BillingError> {
        let allocations: Vec<(Uuid, String, Decimal)> = event
            .allocations
            .iter()
            .map(|a| (a.invoice_ref, a.invoice_kind.clone(), a.amount))
            .collect();
        self.billing
            .reverse_settlements_once(
                event_id,
                &self.consumer,
                event.company_id,
                event.payment_id,
                &allocations,
                self.reconcile.as_ref(),
            )
            .await
    }
}

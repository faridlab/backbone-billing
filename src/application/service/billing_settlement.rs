//! The settlement seam + payment schedules — a focused sibling of `billing_write_service`.
//!
//! Hand-authored, user-owned. Split out of `billing_write_service.rs` because this is the cash loop,
//! not the ledger: it is driven by payment's `PaymentSettled` / `PaymentCancelled` via an ACL and
//! applies to EITHER invoice kind. `BillingWriteService`'s struct, DTOs and errors stay in
//! `billing_write_service.rs`; Rust lets one inherent impl be split across modules of a crate.
//!
//! Holds no SQL — the drawdown's statements live in `InvoiceSettlementRepository` (cross-table, by a
//! runtime kind) and `PaymentScheduleRepository`. What stays here is what MUST: the unit of work, the
//! lock ORDER (invoice `FOR UPDATE` first, then the schedules — this serializes concurrent
//! settlements of one invoice), and the clamp arithmetic.

use backbone_orm::company_scope;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entity::InvoiceKind;
use crate::infrastructure::persistence::{parse_invoice_kind, NewPaymentScheduleRow};

use super::billing_write_service::{money, BillingError, BillingWriteService};

/// Resolve the settlement seam's wire `kind` into the closed enum the repository dispatches on.
/// Unknown kinds keep their original domain error.
fn invoice_kind(kind: &str) -> Result<InvoiceKind, BillingError> {
    parse_invoice_kind(kind).ok_or_else(|| BillingError::NotDraft(format!("unknown invoice kind {kind}")))
}

impl BillingWriteService {
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
}

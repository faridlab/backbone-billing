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
//!
//! Since the reconciliation graph landed, every settlement ALSO writes its ledger counterpart: the
//! same transaction creates (or unlinks) the edge between the invoice's receivable/payable line and
//! the payment's line on the same control account, through the shared [`ReconcileSink`] port —
//! billing's `outstanding_amount` stays a cache, probed equal to `grand_total − Σ edge amounts`.

use backbone_orm::company_scope;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::entity::InvoiceKind;
use crate::infrastructure::persistence::{parse_invoice_kind, NewPaymentScheduleRow};

use super::billing_gl::{
    ReconcileLine, ReconcileOrigin, ReconcilePairRequest, ReconcileSink, UnreconcilePairRequest,
};
use super::billing_write_service::{money, BillingError, BillingWriteService};

/// The result of applying a settlement to an invoice (council 2026-07-26, rec #3).
///
/// `applied` is what was knocked off the invoice's outstanding (clamped to what was owed).
/// `remainder` is the unapplied portion (`requested − applied`) — overpaid cash the CALLER must
/// book as an on-account party credit (billing keeps no customer-credit concept). Surfacing it in
/// the return type makes the caller's booking obligation a compile-time fact, not a docstring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettlementOutcome {
    pub applied: Decimal,
    pub remainder: Decimal,
}

/// An invoice's materialized early-pay-discount decision, resolved at settlement time.
///
/// Carries the PERCENT and the invoice's outstanding BASIS (not a precomputed amount): the payment
/// applies the discount to what it actually ALLOCATES to this invoice, clamped to the basis — so
/// a partial payment inside the window earns the discount on the paid part only, and an
/// over-allocating payment cannot farm discount on money the invoice never asked for. An amount
/// fixed at resolve time would be wrong the moment the allocation differs from the outstanding;
/// the basis alone travels because the outstanding moves with each knock-off between resolves.
#[derive(Debug, Clone)]
pub struct EarlyPayDiscount {
    pub invoice_ref: Uuid,
    pub invoice_kind: String,
    pub percent: Decimal,
    pub outstanding_basis: Decimal,
    pub account_id: Uuid,
}

/// Resolve the settlement seam's wire `kind` into the closed enum the repository dispatches on.
/// Unknown kinds keep their original domain error.
fn invoice_kind(kind: &str) -> Result<InvoiceKind, BillingError> {
    parse_invoice_kind(kind)
        .ok_or_else(|| BillingError::NotDraft(format!("unknown invoice kind {kind}")))
}

/// The two reconciliation-graph locator sides of one invoice↔payment settlement, on the invoice's
/// control account. Sales: the invoice's A/R debit (`order`) meets the payment's A/R credit
/// (`payment`); purchase mirrors (payment debits A/P, invoice credits it via `expense`). Debit is
/// whichever side owes, credit whichever pays — the graph's direction guard demands it.
fn edge_sides(
    kind: InvoiceKind,
    invoice_ref: Uuid,
    control_account: Uuid,
    payment_id: Uuid,
) -> (ReconcileLine, ReconcileLine) {
    match kind {
        InvoiceKind::Sales => (
            ReconcileLine::new("order", invoice_ref, control_account),
            ReconcileLine::new("payment", payment_id, control_account),
        ),
        InvoiceKind::Purchase => (
            ReconcileLine::new("payment", payment_id, control_account),
            ReconcileLine::new("expense", invoice_ref, control_account),
        ),
    }
}

impl BillingWriteService {
    // ---- Early-pay discount resolution ---------------------------------------

    /// Resolve an invoice's early-pay-discount decision for a settlement happening on `on_date`.
    ///
    /// Applicable iff the invoice was posted with a materialized discount block whose deadline
    /// covers `on_date` and whose outstanding is still positive. Returns the percent + the expense
    /// account; the CALLER computes the amount on what it allocates (see [`EarlyPayDiscount`]).
    /// Serves either invoice kind; unknown kind keeps its original domain error.
    pub async fn resolve_early_pay_discount(
        &self,
        company_id: Uuid,
        invoice_ref: Uuid,
        kind: &str,
        on_date: chrono::NaiveDate,
    ) -> Result<Option<EarlyPayDiscount>, BillingError> {
        let ikind = invoice_kind(kind)?;
        let table = match ikind {
            InvoiceKind::Sales => "sales_invoices",
            InvoiceKind::Purchase => "purchase_invoices",
        };
        // Fence-correct read: bind the company on a dedicated transaction (a raw pool query
        // carries no `app.company_id`, and the strict invoice fence would hide the row entirely).
        let mut tx = self.db_pool.begin().await.map_err(BillingError::Db)?;
        company_scope::bind_company_on(&mut tx, company_id)
            .await
            .map_err(BillingError::Db)?;
        let fetched = self
            .terms
            .fetch_invoice_epd(&mut *tx, table, invoice_ref)
            .await
            .map_err(BillingError::Db)?;
        tx.commit().await.map_err(BillingError::Db)?;
        let Some(epd) = fetched else {
            return Ok(None); // unknown invoice → no discount; the settlement itself will 404
        };
        if epd.posting_state != "posted" {
            return Ok(None); // drafts carry no live discount decision
        }
        if epd.early_pay_discount_percent <= Decimal::ZERO
            || epd.outstanding_amount <= Decimal::ZERO
        {
            return Ok(None);
        }
        let Some(deadline) = epd.early_pay_discount_deadline else {
            return Ok(None);
        };
        if on_date > deadline {
            return Ok(None); // outside the window — full amount only
        }
        let Some(account_id) = epd.early_pay_discount_account_id else {
            return Ok(None); // materialization guarantees one; refuse silently rather than guess
        };
        Ok(Some(EarlyPayDiscount {
            invoice_ref,
            invoice_kind: kind.to_string(),
            percent: epd.early_pay_discount_percent,
            outstanding_basis: epd.outstanding_amount,
            account_id,
        }))
    }

    // ---- Payment schedule ---------------------------------------------------

    /// Attach installment due dates to an invoice (AR or A/P). Lean — settlement is payments' job.
    /// Refused when the invoice carries a payment term: the term's derived installments and a
    /// manual schedule cannot coexist (the post hook enforces the same rule from the other side).
    pub async fn add_payment_schedule(
        &self,
        invoice_ref: Uuid,
        kind: &str,
        company_id: Uuid,
        installments: &[(chrono::NaiveDate, Decimal)],
    ) -> Result<(), BillingError> {
        let ikind = invoice_kind(kind)?;
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): company is an explicit argument.
        company_scope::bind_company_on(&mut tx, company_id).await?;
        let table = match ikind {
            InvoiceKind::Sales => "sales_invoices",
            InvoiceKind::Purchase => "purchase_invoices",
        };
        if let Some(ctx) = self
            .terms
            .fetch_term_context(&mut *tx, table, invoice_ref)
            .await?
        {
            if ctx.payment_term_id.is_some() {
                return Err(BillingError::TermInvalid {
                    code: "schedule_conflicts_with_term".into(),
                    message: "an invoice with a payment term derives its installments; drop the term first".into(),
                });
            }
        }
        for (i, (due, amt)) in installments.iter().enumerate() {
            self.schedules
                .insert_schedule(
                    &mut tx,
                    &NewPaymentScheduleRow {
                        id: Uuid::new_v4(),
                        invoice_ref,
                        kind,
                        company_id,
                        installment_no: (i + 1) as i32,
                        due_date: *due,
                        amount: money(*amt),
                    },
                )
                .await?;
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
    ///
    /// The same transaction creates the reconciliation edge (debit = invoice control line, credit =
    /// payment control line, amount = the clamped `applied`) through `reconcile` — so the subledger
    /// drawdown and the graph edge commit or roll back together. **Fail-closed:** if the sink refuses
    /// the pair (unposted journal, guard violation) or clamps to a different amount than billing did,
    /// the whole settlement rolls back rather than leaving the cache and the graph disagreeing.
    pub async fn apply_settlement(
        &self,
        company_id: Uuid,
        invoice_ref: Uuid,
        kind: &str,
        amount: Decimal,
        payment_id: Uuid,
        reconcile: &dyn ReconcileSink,
    ) -> Result<SettlementOutcome, BillingError> {
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): the tenant is now an EXPLICIT argument (council 2026-07-26, rec #2) —
        // previously this seam relied on an ambient `with_company_scope` the caller was trusted to set,
        // which left a "forget the wrapper and you get InvoiceNotFound" trap. `bind_company_on` sets the
        // same tx-local `app.company_id` the repo's RLS policy reads.
        company_scope::bind_company_on(&mut tx, company_id).await?;
        let applied = self
            .apply_settlement_in_tx(
                &mut tx,
                invoice_ref,
                kind,
                amount,
                company_id,
                payment_id,
                reconcile,
            )
            .await?;
        tx.commit().await?;
        Ok(SettlementOutcome {
            applied,
            remainder: money(amount) - applied,
        })
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
        company_id: Uuid,
        payment_id: Uuid,
        allocations: &[(Uuid, String, Decimal)],
        reconcile: &dyn ReconcileSink,
    ) -> Result<SettlementOutcome, BillingError> {
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): explicit `company_id` (council 2026-07-26, rec #2) — the relay/ACL
        // passes the event's company; previously this relied on an ambient scope.
        company_scope::bind_company_on(&mut tx, company_id).await?;
        let first = backbone_outbox::inbox::once(&mut *tx, "billing", consumer, event_id)
            .await
            .map_err(|e| BillingError::Db(sqlx::Error::Protocol(e.to_string())))?;
        if !first {
            tx.commit().await?; // already consumed — exactly-once no-op
            return Ok(SettlementOutcome {
                applied: Decimal::ZERO,
                remainder: Decimal::ZERO,
            });
        }
        let mut applied_total = Decimal::ZERO;
        let mut requested_total = Decimal::ZERO;
        for (invoice_ref, kind, amount) in allocations {
            requested_total += money(*amount);
            applied_total += self
                .apply_settlement_in_tx(
                    &mut tx,
                    *invoice_ref,
                    kind,
                    *amount,
                    company_id,
                    payment_id,
                    reconcile,
                )
                .await?;
        }
        tx.commit().await?;
        Ok(SettlementOutcome {
            applied: applied_total,
            remainder: requested_total - applied_total,
        })
    }

    /// The drawdown core, on a caller-supplied transaction (shared by `apply_settlement` and the
    /// exactly-once `apply_settlements_once`). Clamps to what is owed; the remainder is on-account.
    /// Creates the reconciliation edge with the clamped amount — the graph IS the ledger-side record
    /// of this settlement, so a refusal or clamp mismatch aborts the whole unit of work.
    async fn apply_settlement_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        invoice_ref: Uuid,
        kind: &str,
        amount: Decimal,
        company_id: Uuid,
        payment_id: Uuid,
        reconcile: &dyn ReconcileSink,
    ) -> Result<Decimal, BillingError> {
        let ikind = invoice_kind(kind)?;
        if amount < Decimal::ZERO {
            return Err(BillingError::NegativeAmount);
        }
        let amount = money(amount);
        if amount.is_zero() {
            return Ok(Decimal::ZERO);
        }

        // Lock the invoice FIRST, then draw the schedules down — this order serializes concurrent
        // settlements of the same invoice.
        let row = self
            .settlement
            .lock_outstanding(&mut **tx, ikind, invoice_ref)
            .await?
            .ok_or(BillingError::InvoiceNotFound(invoice_ref))?;
        let outstanding = row.outstanding_amount;
        // Clamp to what is owed; the remainder is the caller's on-account credit.
        let applied = if amount < outstanding {
            amount
        } else {
            outstanding
        };
        if applied.is_zero() {
            return Ok(Decimal::ZERO);
        }
        let new_out = outstanding - applied;
        let new_status = if new_out.is_zero() {
            "paid"
        } else {
            "partially_paid"
        };
        self.settlement
            .update_settlement(&mut **tx, ikind, invoice_ref, new_out, new_status)
            .await?;

        // Draw down payment schedules fill-in-order (earliest installment first) by the applied amount.
        let scheds = self
            .schedules
            .lock_schedules_fill_order(&mut **tx, invoice_ref, kind)
            .await?;
        let mut remaining = applied;
        for s in &scheds {
            if remaining.is_zero() {
                break;
            }
            let capacity = s.amount - s.paid_amount;
            if capacity <= Decimal::ZERO {
                continue;
            }
            let take = if capacity < remaining {
                capacity
            } else {
                remaining
            };
            let new_paid = s.paid_amount + take;
            let sstatus = if new_paid >= s.amount {
                "paid"
            } else {
                "partially_paid"
            };
            self.schedules
                .update_paid(&mut **tx, s.id, new_paid, sstatus)
                .await?;
            remaining -= take;
        }

        // The ledger-side settlement record: one partial edge between the invoice's control line and
        // the payment's, for exactly what was knocked off. Same transaction — edge and drawdown
        // commit together, so `outstanding == grand_total − Σ edges` holds by construction.
        let (debit, credit) = edge_sides(ikind, invoice_ref, row.control_account_id, payment_id);
        let ack = reconcile
            .reconcile_pair_on(
                &mut **tx,
                &ReconcilePairRequest {
                    company_id,
                    debit,
                    credit,
                    amount: applied,
                    origin: ReconcileOrigin::Settlement,
                },
            )
            .await
            .map_err(|r| BillingError::ReconcileRefused {
                code: r.code,
                message: r.message,
            })?;
        if ack.applied != applied {
            // The graph clamped to a different amount than billing's own clamp — the subledger and
            // the ledger disagree about what is absorbable. Refuse rather than drift.
            return Err(BillingError::ReconcileRefused {
                code: "reconcile_clamp_mismatch".into(),
                message: format!(
                    "billing clamped {applied} but the graph applied {}",
                    ack.applied
                ),
            });
        }
        Ok(applied)
    }

    /// Reverse a settlement against an invoice (driven by payment's `PaymentCancelled` via an ACL) —
    /// the paired mirror of `apply_settlement` (council 2026-07-05). **Restores** `outstanding_amount`
    /// by `restored = min(amount, grand_total − outstanding)` (never above the invoice total — the
    /// bound that makes a repeat/redelivered reverse safe), rewinds payment schedules last-installment
    /// first (the inverse of fill-in-order), and flips status → `partially_paid` / `submitted`
    /// (re-owed). Without this, a refunded/bounced payment would leave the invoice permanently `paid`.
    ///
    /// The graph edge is unlinked FIRST (side-effecting: any exchange-difference move the edge
    /// generated is reversed by the sink), then the outstanding is restored — both in the one
    /// transaction. Returns the amount actually restored.
    pub async fn reverse_settlement(
        &self,
        company_id: Uuid,
        invoice_ref: Uuid,
        kind: &str,
        amount: Decimal,
        payment_id: Uuid,
        reconcile: &dyn ReconcileSink,
    ) -> Result<Decimal, BillingError> {
        let mut tx = self.db_pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        let restored = self
            .reverse_settlement_in_tx(
                &mut tx,
                invoice_ref,
                kind,
                amount,
                company_id,
                payment_id,
                reconcile,
            )
            .await?;
        tx.commit().await?;
        Ok(restored)
    }

    /// **Go-live exactly-once settlement-reversal consumer** — the mirror of
    /// [`Self::apply_settlements_once`] for `PaymentCancelled`: one inbox dedup on the bus
    /// `event_id`, then every allocation's edge unlinked and outstanding restored in one
    /// transaction. An at-least-once redelivery is a no-op. Returns the total restored (0 on a
    /// redelivery). Requires `billing.inbox_consumed` (same migration as the apply path).
    pub async fn reverse_settlements_once(
        &self,
        event_id: Uuid,
        consumer: &str,
        company_id: Uuid,
        payment_id: Uuid,
        allocations: &[(Uuid, String, Decimal)],
        reconcile: &dyn ReconcileSink,
    ) -> Result<Decimal, BillingError> {
        let mut tx = self.db_pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        let first = backbone_outbox::inbox::once(&mut *tx, "billing", consumer, event_id)
            .await
            .map_err(|e| BillingError::Db(sqlx::Error::Protocol(e.to_string())))?;
        if !first {
            tx.commit().await?; // already consumed — exactly-once no-op
            return Ok(Decimal::ZERO);
        }
        let mut restored_total = Decimal::ZERO;
        for (invoice_ref, kind, amount) in allocations {
            restored_total += self
                .reverse_settlement_in_tx(
                    &mut tx,
                    *invoice_ref,
                    kind,
                    *amount,
                    company_id,
                    payment_id,
                    reconcile,
                )
                .await?;
        }
        tx.commit().await?;
        Ok(restored_total)
    }

    /// The restore core, on a caller-supplied transaction. Unlinks the graph pair BEFORE restoring
    /// the outstanding (the unlink is side-effecting — it may post reversal moves — and must not
    /// outlive an aborted restore).
    async fn reverse_settlement_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        invoice_ref: Uuid,
        kind: &str,
        amount: Decimal,
        company_id: Uuid,
        payment_id: Uuid,
        reconcile: &dyn ReconcileSink,
    ) -> Result<Decimal, BillingError> {
        let ikind = invoice_kind(kind)?;
        if amount < Decimal::ZERO {
            return Err(BillingError::NegativeAmount);
        }
        let amount = money(amount);
        if amount.is_zero() {
            return Ok(Decimal::ZERO);
        }

        let row = self
            .settlement
            .lock_outstanding(&mut **tx, ikind, invoice_ref)
            .await?
            .ok_or(BillingError::InvoiceNotFound(invoice_ref))?;
        let outstanding = row.outstanding_amount;
        let grand = row.grand_total;
        // Restore at most what was actually settled — never push outstanding above the invoice total.
        let headroom = grand - outstanding;
        let restored = if amount < headroom { amount } else { headroom };
        if restored.is_zero() {
            return Ok(Decimal::ZERO);
        }

        // Unlink the settlement's graph edge first: reverses any move the edge generated (exchange
        // difference), repairs the matching group, removes the partial — all on this transaction.
        let (debit, credit) = edge_sides(ikind, invoice_ref, row.control_account_id, payment_id);
        reconcile
            .unreconcile_pair_on(
                &mut **tx,
                &UnreconcilePairRequest {
                    company_id,
                    debit,
                    credit,
                },
            )
            .await
            .map_err(|r| BillingError::ReconcileRefused {
                code: r.code,
                message: r.message,
            })?;

        let new_out = outstanding + restored;
        let new_status = if new_out >= grand {
            "submitted"
        } else {
            "partially_paid"
        };
        self.settlement
            .update_settlement(&mut **tx, ikind, invoice_ref, new_out, new_status)
            .await?;

        // Rewind schedules last-installment first (inverse of the fill-in-order drawdown).
        let scheds = self
            .schedules
            .lock_schedules_rewind_order(&mut **tx, invoice_ref, kind)
            .await?;
        let mut remaining = restored;
        for s in &scheds {
            if remaining.is_zero() {
                break;
            }
            if s.paid_amount <= Decimal::ZERO {
                continue;
            }
            let take = if s.paid_amount < remaining {
                s.paid_amount
            } else {
                remaining
            };
            let new_paid = s.paid_amount - take;
            let sstatus = if new_paid.is_zero() {
                "unpaid"
            } else if new_paid >= s.amount {
                "paid"
            } else {
                "partially_paid"
            };
            self.schedules
                .update_paid(&mut **tx, s.id, new_paid, sstatus)
                .await?;
            remaining -= take;
        }
        Ok(restored)
    }

    /// The derived overdue read: open, GL-posted invoices whose due date has passed (`today`
    /// injected by the caller — deterministic tests). Both kinds, earliest due first.
    pub async fn list_overdue_invoices(
        &self,
        company_id: Uuid,
        today: chrono::NaiveDate,
    ) -> Result<Vec<crate::infrastructure::persistence::OverdueInvoiceRow>, BillingError> {
        let mut tx = self.db_pool.begin().await.map_err(BillingError::Db)?;
        company_scope::bind_company_on(&mut tx, company_id)
            .await
            .map_err(BillingError::Db)?;
        let rows = self
            .settlement
            .list_overdue_invoices(&mut *tx, company_id, today)
            .await
            .map_err(BillingError::Db)?;
        tx.commit().await.map_err(BillingError::Db)?;
        Ok(rows)
    }
}

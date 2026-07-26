//! The Sales Invoice (A/R) write path — a focused sibling of `billing_write_service`.
//!
//! Hand-authored, user-owned. Split out of `billing_write_service.rs` so each side of the ledger
//! reads on its own: this file owns draft creation, the revenue post builder, the post + reconcile,
//! and the credit note. `BillingWriteService`'s struct, DTOs, errors and pricing stay in
//! `billing_write_service.rs`; Rust lets one inherent impl be split across modules of a crate.
//!
//! Holds no SQL — every statement lives in `SalesInvoiceRepository` / `SalesInvoiceLineRepository` /
//! `InvoiceTaxLineRepository` (the module's 4-layer rule). This file orchestrates: it owns the unit
//! of work, decides the company scope (ADR-0008), and publishes the seam events.

use backbone_orm::company_scope;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::infrastructure::persistence::{NewSalesInvoiceLineRow, NewSalesInvoiceRow};

use super::billing_events::{BilledLine, BillingEvent, InvoiceCancelled, SalesInvoicePosted};
use super::billing_gl::{AccountingPostEnvelope, GlPostLine, GlPostSink};
use super::billing_write_service::{
    is_dup, posted_outcome, price, BillingError, BillingWriteService, NewSalesInvoice, PostOutcome,
};

impl BillingWriteService {
    // ---- Sales Invoice (AR) -------------------------------------------------

    pub async fn create_sales_invoice(&self, inv: NewSalesInvoice) -> Result<Uuid, BillingError> {
        let (priced, net_total, output, _in, _wht) = price(&inv.lines, &inv.tax_lines)?;
        let grand = net_total + output;
        let id = Uuid::new_v4();
        let currency = inv.currency.unwrap_or_else(|| "IDR".into());
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): company on the DTO.
        company_scope::bind_company_on(&mut tx, inv.company_id).await?;
        let r = self.sales.insert_draft(&mut tx, &NewSalesInvoiceRow {
            id,
            invoice_number: &inv.invoice_number,
            company_id: inv.company_id,
            branch_id: inv.branch_id,
            customer_id: inv.customer_id,
            source_so_id: inv.source_so_id,
            posting_date: inv.posting_date,
            due_date: inv.due_date,
            currency: &currency,
            net_total,
            tax_total: output,
            grand_total: grand,
            receivable_account_id: inv.receivable_account_id,
        }).await;
        if let Err(e) = r {
            return Err(if is_dup(&e) { BillingError::DuplicateNumber(inv.invoice_number) } else { e.into() });
        }
        for p in &priced {
            self.sales_lines.insert_line(&mut tx, &NewSalesInvoiceLineRow {
                id: Uuid::new_v4(),
                invoice_id: id,
                company_id: inv.company_id,
                item_id: p.item_id,
                account_id: p.account_id,
                description: p.description.as_deref(),
                quantity: p.quantity,
                unit_price: p.unit_price,
                net_amount: p.net_amount,
            }).await?;
        }
        self.insert_tax_lines(&mut tx, id, inv.company_id, "sales", &inv.tax_lines).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Build the balanced revenue post: `Dr A/R (grand) [customer] · Cr Revenue (net per account) ·
    /// Cr PPN Output (per overlay output line)`.
    pub async fn build_ar_post(&self, invoice_id: Uuid) -> Result<AccountingPostEnvelope, BillingError> {
        // RLS scope (ADR-0008), ID-only: fenced by the request/inherited scope.
        let inv = self.sales.fetch_ar_header(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        let currency = inv.currency;
        if currency != "IDR" { return Err(BillingError::UnsupportedCurrency(currency)); }

        // Cr Revenue per income account.
        let rev_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.sales_lines.fetch_revenue_amounts(&self.db_pool, invoice_id),
        ).await?;
        let mut revenue: BTreeMap<Uuid, Decimal> = BTreeMap::new();
        for r in &rev_rows {
            *revenue.entry(r.revenue_account_id).or_insert(Decimal::ZERO) += r.net_amount;
        }
        // Cr PPN Output per overlay output line.
        let tax_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.tax_lines.fetch_amounts_by_basis(&self.db_pool, invoice_id, "sales", "output"),
        ).await?;

        let mut lines = vec![
            GlPostLine::debit(inv.receivable_account_id, inv.grand_total)
                .with_party("customer", inv.customer_id)
                .with_description(format!("A/R {}", inv.invoice_number)),
        ];
        for (acct, amt) in &revenue { lines.push(GlPostLine::credit(*acct, *amt).with_description("Revenue")); }
        for t in &tax_rows { lines.push(GlPostLine::credit(t.account_id, t.tax_amount).with_description("PPN Output")); }

        let env = AccountingPostEnvelope {
            idempotency_key: invoice_id.to_string(), company_id: inv.company_id, branch_id: inv.branch_id,
            source_type: "order".into(), source_id: invoice_id, source_reference: Some(inv.invoice_number),
            posting_date: inv.posting_date, currency, posting_type: "original".into(), reverses_post_id: None,
            description: Some("Sales invoice".into()), lines,
        };
        if !env.is_balanced() { return Err(BillingError::UnbalancedPost); }
        Ok(env)
    }

    pub async fn post_sales_invoice(&self, invoice_id: Uuid, sink: &dyn GlPostSink) -> Result<PostOutcome, BillingError> {
        if let Some(o) = self.short_circuit_sales_posted(invoice_id).await? { return Ok(o); }
        let env = self.build_ar_post(invoice_id).await?;
        match sink.post(&env).await {
            Ok(ack) => {
                // The pending→posted transition AND the durable outbox stage commit in ONE tx, so a
                // crash after the transition can never lose the `SalesInvoicePosted` event (go-live
                // durable bus — mirrors backbone-payment::post_payment). Only the winner of a
                // concurrent double-post (rows_affected == 1) stages + publishes; the loser reconciles
                // from the persisted row without re-emitting.
                let mut tx = self.db_pool.begin().await?;
                company_scope::bind_company_on(&mut tx, env.company_id).await?;
                let affected = self.sales
                    .mark_posted_on(&mut *tx, invoice_id, ack.journal_id, ack.post_id).await?;
                if affected == 0 {
                    tx.rollback().await?;
                    return self.short_circuit_sales_posted(invoice_id).await?
                        .ok_or(BillingError::InvoiceNotFound(invoice_id));
                }
                let hdr = self.sales.fetch_seam_header_on(&mut *tx, invoice_id).await?;
                // Billed lines (item + qty) for the selling seam — advance the source SO's billed_qty.
                let line_rows = self.sales_lines.fetch_billed_lines_on(&mut *tx, invoice_id).await?;
                let billed_lines: Vec<BilledLine> = line_rows.iter()
                    .map(|r| BilledLine { item_id: r.item_id, quantity: r.quantity }).collect();
                let event = SalesInvoicePosted {
                    invoice_id, company_id: env.company_id, journal_id: ack.journal_id, post_id: ack.post_id,
                    source_so_id: hdr.source_so_id, billed_lines, grand_total: hdr.grand_total,
                };
                if let Some(schema) = self.outbox_schema.clone() {
                    self.stage_outbox_event(
                        &mut *tx, &schema, "SalesInvoicePosted", "SalesInvoice", invoice_id, &event,
                    ).await?;
                }
                tx.commit().await?;
                self.sink.publish(BillingEvent::SalesInvoicePosted(event));
                Ok(PostOutcome { invoice_id, post_id: ack.post_id, journal_id: ack.journal_id, idempotent_reuse: ack.idempotent_reuse })
            }
            Err(rej) => {
                let _ = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.sales.mark_posting_failed(&self.db_pool, invoice_id),
                ).await;
                Err(BillingError::GlRejected { code: rej.code, message: rej.message })
            }
        }
    }


    /// Credit-note a posted Sales Invoice — REVERSE the revenue recognition (council 2026-07-05, the
    /// POS returns path). Posts the sign-flipped AR journal (`Dr Revenue · Cr A/R [customer]`,
    /// `posting_type="reversal"`, linked via `reverses_post_id`), sets `outstanding_amount=0`, flips the
    /// invoice → `cancelled`, and emits `InvoiceCancelled`. Distinct from `reverse_settlement` (which
    /// only restores the settlement's outstanding) — this undoes the revenue itself, so a full return
    /// (credit note + payment refund) nets revenue, cash, and A/R to zero. Idempotent: a re-credit
    /// short-circuits on `status='cancelled'`.
    pub async fn reverse_sales_invoice(&self, invoice_id: Uuid, sink: &dyn GlPostSink) -> Result<PostOutcome, BillingError> {
        // RLS scope (ADR-0008), ID-only: fenced by the request/inherited scope.
        let orig_post: Option<Uuid> = self.sales.fetch_accounting_post_id(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        // The forward revenue post, sign-flipped, is the credit note.
        let fwd = self.build_ar_post(invoice_id).await?;
        let lines = fwd.lines.iter().map(|l| GlPostLine {
            account_id: l.account_id, debit: l.credit, credit: l.debit,
            party_type: l.party_type.clone(), party_id: l.party_id,
            description: l.description.as_ref().map(|d| format!("Credit note: {d}")),
        }).collect();
        let env = AccountingPostEnvelope {
            idempotency_key: format!("reversal:{invoice_id}"), posting_type: "reversal".into(),
            reverses_post_id: orig_post, lines,
            description: fwd.description.map(|d| format!("Credit note: {d}")),
            ..fwd
        };
        if !env.is_balanced() { return Err(BillingError::UnbalancedPost); }
        match sink.post(&env).await {
            Ok(ack) => {
                // The cancelled transition AND the durable outbox stage commit in ONE tx (mirrors the
                // post path + backbone-payment). Only the call that flips → cancelled (affected == 1)
                // stages + publishes; an idempotent re-credit commits without re-emitting.
                let mut tx = self.db_pool.begin().await?;
                company_scope::bind_company_on(&mut tx, env.company_id).await?;
                let affected = self.sales.mark_cancelled_on(&mut *tx, invoice_id).await?;
                if affected == 1 {
                    let event = InvoiceCancelled { invoice_id, kind: "sales".into() };
                    if let Some(schema) = self.outbox_schema.clone() {
                        self.stage_outbox_event(
                            &mut *tx, &schema, "InvoiceCancelled", "SalesInvoice", invoice_id, &event,
                        ).await?;
                    }
                    tx.commit().await?;
                    self.sink.publish(BillingEvent::InvoiceCancelled(event));
                } else {
                    tx.commit().await?;
                }
                Ok(PostOutcome { invoice_id, post_id: ack.post_id, journal_id: ack.journal_id, idempotent_reuse: ack.idempotent_reuse })
            }
            Err(rej) => Err(BillingError::GlRejected { code: rej.code, message: rej.message }),
        }
    }


    // ---- shared -------------------------------------------------------------

    pub(super) async fn short_circuit_sales_posted(&self, invoice_id: Uuid) -> Result<Option<PostOutcome>, BillingError> {
        let row = self.sales.fetch_posting_state(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        Ok(posted_outcome(invoice_id, row))
    }
}

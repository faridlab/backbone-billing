//! The Purchase Invoice (A/P) write path — a focused sibling of `billing_write_service`.
//!
//! Hand-authored, user-owned. Split out of `billing_write_service.rs` so each side of the ledger
//! reads on its own: this file owns draft creation, the payable post builder, and the post +
//! reconcile. `BillingWriteService`'s struct, DTOs, errors and pricing stay in
//! `billing_write_service.rs`; Rust lets one inherent impl be split across modules of a crate.
//!
//! Holds no SQL — every statement lives in `PurchaseInvoiceRepository` /
//! `PurchaseInvoiceLineRepository` / `InvoiceTaxLineRepository` (the module's 4-layer rule). This
//! file orchestrates: it owns the unit of work, decides the company scope (ADR-0008), and publishes
//! the seam events.

use backbone_orm::company_scope;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::infrastructure::persistence::{NewPurchaseInvoiceLineRow, NewPurchaseInvoiceRow};

use super::billing_events::{BilledLine, BillingEvent, PurchaseInvoicePosted};
use super::billing_gl::{AccountingPostEnvelope, GlPostLine, GlPostSink};
use super::billing_write_service::{
    is_dup, posted_outcome, price, BillingError, BillingWriteService, NewPurchaseInvoice, PostOutcome,
};

impl BillingWriteService {
    // ---- Purchase Invoice (A/P) ---------------------------------------------

    pub async fn create_purchase_invoice(&self, inv: NewPurchaseInvoice) -> Result<Uuid, BillingError> {
        let (priced, net_total, _out, input, wht) = price(&inv.lines, &inv.tax_lines)?;
        let grand = net_total + input - wht; // A/P owed
        // Withholding is a deduction from the payable, never more than the base: a negative A/P would
        // persist a negative outstanding + emit a negative-A/P seam event (fails open). Refuse it.
        if grand < Decimal::ZERO { return Err(BillingError::NegativeAmount); }
        let id = Uuid::new_v4();
        let currency = inv.currency.unwrap_or_else(|| "IDR".into());
        let mut tx = self.db_pool.begin().await?;
        // RLS scope (ADR-0008): company on the DTO.
        company_scope::bind_company_on(&mut tx, inv.company_id).await?;
        let r = self.purchases.insert_draft(&mut tx, &NewPurchaseInvoiceRow {
            id,
            invoice_number: &inv.invoice_number,
            company_id: inv.company_id,
            branch_id: inv.branch_id,
            supplier_id: inv.supplier_id,
            source_po_id: inv.source_po_id,
            posting_date: inv.posting_date,
            due_date: inv.due_date,
            currency: &currency,
            net_total,
            tax_total: input,
            withholding_total: wht,
            grand_total: grand,
            payable_account_id: inv.payable_account_id,
        }).await;
        if let Err(e) = r {
            return Err(if is_dup(&e) { BillingError::DuplicateNumber(inv.invoice_number) } else { e.into() });
        }
        for p in &priced {
            self.purchase_lines.insert_line(&mut tx, &NewPurchaseInvoiceLineRow {
                id: Uuid::new_v4(),
                invoice_id: id,
                item_id: p.item_id,
                account_id: p.account_id,
                description: p.description.as_deref(),
                quantity: p.quantity,
                unit_price: p.unit_price,
                net_amount: p.net_amount,
            }).await?;
        }
        self.insert_tax_lines(&mut tx, id, "purchase", &inv.tax_lines).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Build the balanced A/P post: `Dr Expense (net per account) · Dr PPN Input (per input line) ·
    /// Cr A/P (grand) [supplier] · Cr PPh Payable (per withholding line)`.
    pub async fn build_ap_post(&self, invoice_id: Uuid) -> Result<AccountingPostEnvelope, BillingError> {
        // RLS scope (ADR-0008), ID-only: fenced by the request/inherited scope.
        let inv = self.purchases.fetch_ap_header(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        let currency = inv.currency;
        if currency != "IDR" { return Err(BillingError::UnsupportedCurrency(currency)); }

        let exp_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.purchase_lines.fetch_expense_amounts(&self.db_pool, invoice_id),
        ).await?;
        let mut expense: BTreeMap<Uuid, Decimal> = BTreeMap::new();
        for r in &exp_rows {
            *expense.entry(r.expense_account_id).or_insert(Decimal::ZERO) += r.net_amount;
        }
        let input_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.tax_lines.fetch_amounts_by_basis(&self.db_pool, invoice_id, "purchase", "input"),
        ).await?;
        let wht_rows = company_scope::with_company_scope(
            Some(inv.company_id),
            self.tax_lines.fetch_amounts_by_basis(&self.db_pool, invoice_id, "purchase", "withholding"),
        ).await?;

        let mut lines: Vec<GlPostLine> = Vec::new();
        for (acct, amt) in &expense { lines.push(GlPostLine::debit(*acct, *amt).with_description("Expense/GR-IR")); }
        for t in &input_rows { lines.push(GlPostLine::debit(t.account_id, t.tax_amount).with_description("PPN Input")); }
        lines.push(GlPostLine::credit(inv.payable_account_id, inv.grand_total)
            .with_party("supplier", inv.supplier_id)
            .with_description(format!("A/P {}", inv.invoice_number)));
        for t in &wht_rows { lines.push(GlPostLine::credit(t.account_id, t.tax_amount).with_description("PPh Payable")); }

        let env = AccountingPostEnvelope {
            idempotency_key: invoice_id.to_string(), company_id: inv.company_id, branch_id: inv.branch_id,
            source_type: "expense".into(), source_id: invoice_id, source_reference: Some(inv.invoice_number),
            posting_date: inv.posting_date, currency, posting_type: "original".into(), reverses_post_id: None,
            description: Some("Purchase invoice".into()), lines,
        };
        if !env.is_balanced() { return Err(BillingError::UnbalancedPost); }
        Ok(env)
    }

    pub async fn post_purchase_invoice(&self, invoice_id: Uuid, sink: &dyn GlPostSink) -> Result<PostOutcome, BillingError> {
        if let Some(o) = self.short_circuit_purchase_posted(invoice_id).await? { return Ok(o); }
        let env = self.build_ap_post(invoice_id).await?;
        match sink.post(&env).await {
            Ok(ack) => {
                // Gate the reconcile + seam event on THIS invocation performing the pending→posted
                // transition — the A/P seam routes `PurchaseInvoicePosted.billed_lines` into
                // buying::mark_billed, so a double-emit would double-advance the PO's billed_qty and
                // corrupt the 3-way match. Only the winner of the UPDATE race publishes.
                let affected = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchases.mark_posted(&self.db_pool, invoice_id, ack.journal_id, ack.post_id),
                ).await?;
                if affected == 0 {
                    return self.short_circuit_purchase_posted(invoice_id).await?
                        .ok_or(BillingError::InvoiceNotFound(invoice_id));
                }
                // Billed lines (item + qty) for the buying seam.
                let hdr = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchases.fetch_seam_header(&self.db_pool, invoice_id),
                ).await?;
                let line_rows = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchase_lines.fetch_billed_lines(&self.db_pool, invoice_id),
                ).await?;
                let billed_lines: Vec<BilledLine> = line_rows.iter()
                    .map(|r| BilledLine { item_id: r.item_id, quantity: r.quantity }).collect();
                self.sink.publish(BillingEvent::PurchaseInvoicePosted(PurchaseInvoicePosted {
                    invoice_id, company_id: env.company_id, journal_id: ack.journal_id, post_id: ack.post_id,
                    source_po_id: hdr.source_po_id, billed_lines, grand_total: hdr.grand_total,
                }));
                Ok(PostOutcome { invoice_id, post_id: ack.post_id, journal_id: ack.journal_id, idempotent_reuse: ack.idempotent_reuse })
            }
            Err(rej) => {
                let _ = company_scope::with_company_scope(
                    Some(env.company_id),
                    self.purchases.mark_posting_failed(&self.db_pool, invoice_id),
                ).await;
                Err(BillingError::GlRejected { code: rej.code, message: rej.message })
            }
        }
    }

    // ---- shared -------------------------------------------------------------

    pub(super) async fn short_circuit_purchase_posted(&self, invoice_id: Uuid) -> Result<Option<PostOutcome>, BillingError> {
        let row = self.purchases.fetch_posting_state(&self.db_pool, invoice_id).await?
            .ok_or(BillingError::InvoiceNotFound(invoice_id))?;
        Ok(posted_outcome(invoice_id, row))
    }
}

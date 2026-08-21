//! Settlement repository — the hand-written SQL for drawing an invoice's `outstanding_amount` down
//! (and back up), across BOTH invoice tables.
//!
//! Hand-authored and **user-owned**: this exact path is declared under `user_owned` in
//! `metaphor.codegen.yaml`. It is not schema-derived and has no entity of its own.
//!
//! **Why this file exists (the one place the per-entity repository shape did not fit).** The
//! settlement seam is driven by payment's `PaymentSettled` / `PaymentCancelled`, which name their
//! target by an `invoice_kind` string decided at RUNTIME — the same statement has to run against
//! `billing.sales_invoices` or `billing.purchase_invoices`. That makes it neither `SalesInvoice`'s
//! SQL nor `PurchaseInvoice`'s, so it lives here rather than being duplicated into both. The table
//! name is chosen ONLY from the closed [`InvoiceKind`] enum — never interpolated from a caller
//! string — so the `format!` below cannot carry untrusted input.

use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::entity::InvoiceKind;

/// Parse the settlement seam's wire `kind` into the schema's own [`InvoiceKind`]. `None` = unknown
/// kind; the caller turns that into its own domain error (the repository does not own domain
/// vocabulary).
///
/// Deliberately NOT `InvoiceKind::from_str`: that one lowercases first, so it would also accept
/// "Sales"/"PURCHASE". The settlement seam has always matched the wire value exactly, and widening
/// what a payment event may say is not this refactor's call to make.
pub fn parse_invoice_kind(kind: &str) -> Option<InvoiceKind> {
    match kind {
        "sales" => Some(InvoiceKind::Sales),
        "purchase" => Some(InvoiceKind::Purchase),
        _ => None,
    }
}

/// The table a given kind settles against. A closed match over the schema enum, so the table name
/// interpolated into the statements below can only ever be one of two compile-time literals.
fn settlement_table(kind: InvoiceKind) -> &'static str {
    match kind {
        InvoiceKind::Sales => "billing.sales_invoices",
        InvoiceKind::Purchase => "billing.purchase_invoices",
    }
}

/// The column holding the invoice's control account (A/R for sales, A/P for purchase). The
/// reconciliation edge between the invoice's line and the payment's line sits on this account —
/// the settlement seam needs it to build the locator.
fn control_account_column(kind: InvoiceKind) -> &'static str {
    match kind {
        InvoiceKind::Sales => "receivable_account_id",
        InvoiceKind::Purchase => "payable_account_id",
    }
}

/// A locked invoice's settlement state. `grand_total` is only read by the reversal path (it is the
/// ceiling the restore clamps against); `control_account_id` feeds the reconciliation locator.
pub struct OutstandingRow {
    pub outstanding_amount: Decimal,
    pub grand_total: Decimal,
    pub control_account_id: Uuid,
}

/// The settlement drawdown's SQL. Stateless: every method takes the CALLER'S transaction, because
/// the whole seam is one unit of work — the invoice lock, the schedule locks, and the writes must
/// all commit or roll back together.
pub struct InvoiceSettlementRepository;

impl Default for InvoiceSettlementRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoiceSettlementRepository {
    pub fn new() -> Self {
        Self
    }

    /// Lock the live invoice `FOR UPDATE` and read what it still owes. `Ok(None)` = no such live
    /// invoice IN THE CALLER'S SCOPE — which is also how this fails closed when no company scope was
    /// established (the settlement entrypoints bind the caller's ambient scope deliberately: they are
    /// event-driven and carry no company argument, so the ACL must wrap them).
    ///
    /// **This lock is the serialization point for the whole seam and must be taken FIRST** — before
    /// the schedules — so two payments racing the same invoice queue here rather than both reading a
    /// stale outstanding.
    pub async fn lock_outstanding(
        &self,
        conn: &mut sqlx::PgConnection,
        kind: InvoiceKind,
        invoice_ref: Uuid,
    ) -> Result<Option<OutstandingRow>, sqlx::Error> {
        let sql = format!(
            "SELECT outstanding_amount, grand_total, {} AS control_account_id FROM {} WHERE id=$1 AND (metadata->>'deleted_at') IS NULL FOR UPDATE",
            control_account_column(kind),
            settlement_table(kind),
        );
        let row = sqlx::query(&sql)
            .bind(invoice_ref)
            .fetch_optional(conn)
            .await?;
        Ok(row.map(|r| OutstandingRow {
            outstanding_amount: r.get("outstanding_amount"),
            grand_total: r.get("grand_total"),
            control_account_id: r.get("control_account_id"),
        }))
    }

    /// Write back the drawn-down (or restored) outstanding + the recomputed invoice status
    /// ("paid" | "partially_paid" | "submitted", cast at the DB).
    ///
    /// Must ride the same transaction that holds the row's `FOR UPDATE` from
    /// [`Self::lock_outstanding`] — don't re-bind the company here, the caller's bind covers it.
    pub async fn update_settlement(
        &self,
        conn: &mut sqlx::PgConnection,
        kind: InvoiceKind,
        invoice_ref: Uuid,
        outstanding_amount: Decimal,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        let sql = format!(
            "UPDATE {} SET outstanding_amount=$2, status=$3::invoice_status WHERE id=$1",
            settlement_table(kind),
        );
        sqlx::query(&sql)
            .bind(invoice_ref)
            .bind(outstanding_amount)
            .bind(status)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// The derived overdue read: open, GL-posted invoices whose due date has passed — both kinds,
    /// one ordered list. **Derived on purpose**: there is no stored `overdue` flag that a later
    /// settlement could leave stale; the filter recomputes from live state on every call.
    /// Served on the caller's company-scoped connection.
    pub async fn list_overdue_invoices(
        &self,
        conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
        company_id: Uuid,
        today: chrono::NaiveDate,
    ) -> Result<Vec<OverdueInvoiceRow>, sqlx::Error> {
        // Both tables carry the same overdue predicate; `kind` labels which side each row came
        // from. `posting_state='posted'` excludes drafts; `status` excludes settled/voided ones.
        let rows = sqlx::query(
            r#"SELECT id, kind, invoice_number, due_date, outstanding_amount, grand_total
               FROM (
                   SELECT id, 'sales'::text AS kind, invoice_number, due_date,
                          outstanding_amount, grand_total
                   FROM billing.sales_invoices
                   WHERE company_id = $1
                     AND posting_state = 'posted'
                     AND status::text NOT IN ('paid', 'cancelled')
                     AND due_date IS NOT NULL AND due_date < $2
                     AND (metadata->>'deleted_at') IS NULL
                   UNION ALL
                   SELECT id, 'purchase'::text, invoice_number, due_date,
                          outstanding_amount, grand_total
                   FROM billing.purchase_invoices
                   WHERE company_id = $1
                     AND posting_state = 'posted'
                     AND status::text NOT IN ('paid', 'cancelled')
                     AND due_date IS NOT NULL AND due_date < $2
                     AND (metadata->>'deleted_at') IS NULL
               ) overdue
               ORDER BY due_date, kind, invoice_number"#,
        )
        .bind(company_id)
        .bind(today)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .iter()
            .map(|r| OverdueInvoiceRow {
                id: r.get("id"),
                kind: r.get("kind"),
                invoice_number: r.get("invoice_number"),
                due_date: r.get("due_date"),
                outstanding_amount: r.get("outstanding_amount"),
                grand_total: r.get("grand_total"),
            })
            .collect())
    }
}

/// One row of the derived overdue read.
pub struct OverdueInvoiceRow {
    pub id: Uuid,
    /// "sales" | "purchase"
    pub kind: String,
    pub invoice_number: String,
    pub due_date: chrono::NaiveDate,
    pub outstanding_amount: Decimal,
    pub grand_total: Decimal,
}

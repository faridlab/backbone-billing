//! Repository for PaymentTerm + PaymentTermLine entities.
//!
//! Hand-authored, user-owned (declared in `metaphor.codegen.yaml`). Holds the payment-terms SQL
//! per the module's 4-layer rule: services orchestrate, repositories hold SQL. Reads must ride a
//! company-scoped connection — `payment_terms` carries a SPLIT fence (read admits the company's own
//! rows AND global NULL-company templates; write stays own-only), so a scoped session sees both
//! while an unscoped one sees nothing (fail-closed, ADR-0008/0014).
//!
//! Thin newtype over `backbone_orm::GenericCrudRepository<PaymentTerm, backbone_orm::SoftDelete>`;
//! standard CRUD via `Deref`.

use anyhow::Result;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::PaymentTerm;

/// Table name for PaymentTerm entities
pub const TABLE_NAME: &str = "billing.payment_terms";

pub struct PaymentTermRepository(
    backbone_orm::GenericCrudRepository<PaymentTerm, backbone_orm::SoftDelete>,
);

impl std::ops::Deref for PaymentTermRepository {
    type Target = backbone_orm::GenericCrudRepository<PaymentTerm, backbone_orm::SoftDelete>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PaymentTermRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self(backbone_orm::GenericCrudRepository::new(pool, TABLE_NAME))
    }
}

/// One slice of a term's installment shape, as the schedule derivation consumes it.
/// Enum columns arrive as `::text` and the derivation matches on the raw strings —
/// the DB row already passed the enum cast on insert.
pub struct TermLineRow {
    pub id: Uuid,
    /// "balance" | "percent" | "fixed"
    pub value: String,
    pub value_amount: Decimal,
    pub nb_days: i32,
    pub day_of_month: Option<i32>,
    /// "days" | "day_following_month" | "day_current_month"
    pub delay_type: String,
    /// "invoice_date" | "end_of_invoice_month"
    pub anchor: String,
    pub sequence: i32,
}

/// A term header's decision inputs: what the invoice-post hook materializes.
pub struct TermHeaderRow {
    pub id: Uuid,
    /// NULL = global template
    pub company_id: Option<Uuid>,
    pub name: String,
    pub status: String,
    pub early_discount: bool,
    pub discount_percent: Decimal,
    pub discount_days: i32,
    pub discount_account_id: Option<Uuid>,
}

/// The materialized early-pay-discount decision inputs read back off an invoice at settlement time.
pub struct InvoiceEpdRow {
    pub early_pay_discount_percent: Decimal,
    pub early_pay_discount_deadline: Option<chrono::NaiveDate>,
    pub early_pay_discount_account_id: Option<Uuid>,
    pub outstanding_amount: Decimal,
    /// `posting_state::text` — the resolver only serves posted invoices.
    pub posting_state: String,
}

/// The exact rows a term insert writes.
pub struct NewPaymentTermRow<'a> {
    pub id: Uuid,
    pub company_id: Option<Uuid>,
    pub name: &'a str,
    pub note: Option<&'a str>,
    pub sequence: i32,
    pub early_discount: bool,
    pub discount_percent: Decimal,
    pub discount_days: i32,
    pub discount_account_id: Option<Uuid>,
    pub discount_tax_basis: &'a str,
}

pub struct NewPaymentTermLineRow<'a> {
    pub id: Uuid,
    pub term_id: Uuid,
    pub company_id: Option<Uuid>,
    pub value: &'a str,
    pub value_amount: Decimal,
    pub nb_days: i32,
    pub day_of_month: Option<i32>,
    pub delay_type: &'a str,
    pub anchor: &'a str,
    pub sequence: i32,
}

impl PaymentTermRepository {
    /// Insert one term header on the caller's transaction (the caller has bound the company scope;
    /// a global row is born only on an owner/bypass connection, never through this path with a
    /// tenant scope — the split fence's WITH CHECK enforces it).
    pub async fn insert_term(
        &self,
        conn: &mut sqlx::PgConnection,
        t: &NewPaymentTermRow<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO billing.payment_terms
                (id, company_id, name, note, sequence, status, early_discount,
                 discount_percent, discount_days, discount_account_id, discount_tax_basis)
               VALUES ($1,$2,$3,$4,$5,'active'::payment_term_status,$6,$7,$8,$9,$10::discount_tax_basis)"#,
        )
        .bind(t.id).bind(t.company_id).bind(t.name).bind(t.note).bind(t.sequence)
        .bind(t.early_discount).bind(t.discount_percent).bind(t.discount_days)
        .bind(t.discount_account_id).bind(t.discount_tax_basis)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Insert one term line on the caller's transaction. The line's company_id is denormalized from
    /// the header so it passes the (same-shape) child fence on its own.
    pub async fn insert_term_line(
        &self,
        conn: &mut sqlx::PgConnection,
        l: &NewPaymentTermLineRow<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO billing.payment_term_lines
                (id, term_id, company_id, value, value_amount, nb_days, day_of_month,
                 delay_type, anchor, sequence)
               VALUES ($1,$2,$3,$4::payment_term_line_value,$5,$6,$7,
                       $8::payment_term_delay_type,$9::payment_term_anchor,$10)"#,
        )
        .bind(l.id)
        .bind(l.term_id)
        .bind(l.company_id)
        .bind(l.value)
        .bind(l.value_amount)
        .bind(l.nb_days)
        .bind(l.day_of_month)
        .bind(l.delay_type)
        .bind(l.anchor)
        .bind(l.sequence)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Read one term header + its lines on a caller-supplied, company-bound connection. The split
    /// fence admits a global term here (and hides another company's).
    pub async fn fetch_term(
        &self,
        conn: &mut sqlx::PgConnection,
        term_id: Uuid,
    ) -> Result<Option<(TermHeaderRow, Vec<TermLineRow>)>, sqlx::Error> {
        let head = sqlx::query(
            r#"SELECT id, company_id, name, status::text AS status, early_discount,
                      discount_percent, discount_days, discount_account_id
               FROM billing.payment_terms
               WHERE id=$1 AND (metadata->>'deleted_at') IS NULL"#,
        )
        .bind(term_id)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(h) = head else { return Ok(None) };
        let header = TermHeaderRow {
            id: h.get("id"),
            company_id: h.get("company_id"),
            name: h.get("name"),
            status: h.get("status"),
            early_discount: h.get("early_discount"),
            discount_percent: h.get("discount_percent"),
            discount_days: h.get("discount_days"),
            discount_account_id: h.get("discount_account_id"),
        };
        let line_rows = self.fetch_term_lines(term_id).fetch_all(&mut *conn).await?;
        let lines = line_rows
            .iter()
            .map(|r| TermLineRow {
                id: r.get("id"),
                value: r.get("value"),
                value_amount: r.get("value_amount"),
                nb_days: r.get("nb_days"),
                day_of_month: r.get("day_of_month"),
                delay_type: r.get("delay_type"),
                anchor: r.get("anchor"),
                sequence: r.get("sequence"),
            })
            .collect();
        Ok(Some((header, lines)))
    }

    /// The lines query, unassembled (so `fetch_term` can reuse the executor).
    fn fetch_term_lines(
        &self,
        term_id: Uuid,
    ) -> sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(
            r#"SELECT id, value::text AS value, value_amount, nb_days, day_of_month,
                      delay_type::text AS delay_type, anchor::text AS anchor, sequence
               FROM billing.payment_term_lines
               WHERE term_id=$1 AND (metadata->>'deleted_at') IS NULL
               ORDER BY sequence, id"#,
        )
        .bind(term_id)
    }

    /// List the term headers visible to a scoped connection (own + global templates), newest last.
    pub async fn list_terms(
        &self,
        conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
        company_id: Uuid,
    ) -> Result<Vec<TermHeaderRow>, sqlx::Error> {
        // Explicit company predicate (not just the fence): the fence admits NULL-company rows on
        // read; this states the intent — "the terms this company may use".
        let rows = sqlx::query(
            r#"SELECT id, company_id, name, status::text AS status, early_discount,
                      discount_percent, discount_days, discount_account_id
               FROM billing.payment_terms
               WHERE (company_id IS NULL OR company_id=$1)
                 AND status='active'::payment_term_status
                 AND (metadata->>'deleted_at') IS NULL
               ORDER BY company_id NULLS FIRST, sequence, name"#,
        )
        .bind(company_id)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .iter()
            .map(|h| TermHeaderRow {
                id: h.get("id"),
                company_id: h.get("company_id"),
                name: h.get("name"),
                status: h.get("status"),
                early_discount: h.get("early_discount"),
                discount_percent: h.get("discount_percent"),
                discount_days: h.get("discount_days"),
                discount_account_id: h.get("discount_account_id"),
            })
            .collect())
    }

    /// Flip a term's status (`active` | `inactive`). Soft-retire: historical invoices keep their
    /// materialized schedule; the term just disappears from new-invoice selection. Rides the
    /// caller's company-bound transaction: the fence's WITH CHECK keeps the UPDATE own-only, so a
    /// global template or another company's term matches zero rows (caller reads `TermNotFound`).
    pub async fn set_status(
        &self,
        conn: &mut sqlx::PgConnection,
        term_id: Uuid,
        status: &str,
    ) -> Result<u64, sqlx::Error> {
        let r = sqlx::query(
            "UPDATE billing.payment_terms SET status=$2::payment_term_status \
             WHERE id=$1 AND (metadata->>'deleted_at') IS NULL",
        )
        .bind(term_id)
        .bind(status)
        .execute(conn)
        .await?;
        Ok(r.rows_affected())
    }

    /// The invoice-post hook's materialization write: stamp the derived due date and the EPD block
    /// onto one invoice header. Rides the caller's post transaction.
    pub async fn materialize_term_on_invoice(
        &self,
        conn: &mut sqlx::PgConnection,
        table: &str,
        invoice_id: Uuid,
        term_id: Uuid,
        due_date: chrono::NaiveDate,
        epd_percent: Decimal,
        epd_deadline: Option<chrono::NaiveDate>,
        epd_account: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        // `table` is a compile-time constant from this module's two call sites — never user input.
        let sql = format!(
            "UPDATE billing.{table} \
             SET due_date=$2, payment_term_id=$3, early_pay_discount_percent=$4, \
                 early_pay_discount_deadline=$5, early_pay_discount_account_id=$6 \
             WHERE id=$1"
        );
        sqlx::query(&sql)
            .bind(invoice_id)
            .bind(due_date)
            .bind(term_id)
            .bind(epd_percent)
            .bind(epd_deadline)
            .bind(epd_account)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Read an invoice's materialized EPD decision inputs (either kind, runtime table dispatch).
    /// Served on the caller's scoped connection.
    pub async fn fetch_invoice_epd(
        &self,
        conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
        table: &str,
        invoice_ref: Uuid,
    ) -> Result<Option<InvoiceEpdRow>, sqlx::Error> {
        let sql = format!(
            "SELECT early_pay_discount_percent, early_pay_discount_deadline, \
                    early_pay_discount_account_id, outstanding_amount, posting_state::text AS ps \
             FROM billing.{table} WHERE id=$1 AND (metadata->>'deleted_at') IS NULL"
        );
        let row = sqlx::query(&sql)
            .bind(invoice_ref)
            .fetch_optional(conn)
            .await?;
        Ok(row.map(|r| InvoiceEpdRow {
            early_pay_discount_percent: r.get("early_pay_discount_percent"),
            early_pay_discount_deadline: r.get("early_pay_discount_deadline"),
            early_pay_discount_account_id: r.get("early_pay_discount_account_id"),
            outstanding_amount: r.get("outstanding_amount"),
            posting_state: r.get("ps"),
        }))
    }

    /// The post hook's invoice inputs: which term (if any) the invoice carries plus the dates and
    /// totals the derivation needs. Rides the caller's post transaction.
    pub async fn fetch_term_context(
        &self,
        conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
        table: &str,
        invoice_id: Uuid,
    ) -> Result<Option<TermContextRow>, sqlx::Error> {
        let sql = format!(
            "SELECT payment_term_id, posting_date, grand_total, due_date \
             FROM billing.{table} WHERE id=$1 AND (metadata->>'deleted_at') IS NULL"
        );
        let row = sqlx::query(&sql)
            .bind(invoice_id)
            .fetch_optional(conn)
            .await?;
        Ok(row.map(|r| TermContextRow {
            payment_term_id: r.get("payment_term_id"),
            posting_date: r.get("posting_date"),
            grand_total: r.get("grand_total"),
            manual_due_date: r.get("due_date"),
        }))
    }
}

/// What the post hook needs off the invoice header before deriving a term's schedule.
pub struct TermContextRow {
    pub payment_term_id: Option<Uuid>,
    pub posting_date: chrono::NaiveDate,
    pub grand_total: Decimal,
    /// The create-time manual due date — must be `None` when a term is applied.
    pub manual_due_date: Option<chrono::NaiveDate>,
}

backbone_core::impl_crud_repository!(PaymentTermRepository, PaymentTerm, soft_delete);

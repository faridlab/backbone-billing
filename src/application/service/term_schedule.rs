//! Payment-term derivation — how a term's lines become an invoice's due dates.
//!
//! Hand-authored, user-owned. Two halves:
//! 1. **Pure derivation** (`derive_schedule`) — term lines + posting date + grand total → the
//!    installment slices `(due_date, amount)`. No I/O, so the golden tests pin it exactly
//!    (anchor/delay matrix + smooth-delta conservation: Σ slices == grand_total to the cent).
//! 2. **The invoice-post hook** (`apply_term_on_post`) — rides the post transaction: refuses
//!    term/manual-date conflicts, materializes the derived `due_date` + the early-pay-discount
//!    block onto the header (history never rewrites), and seeds the payment-schedule rows when the
//!    term splits the balance across installments.
//!
//! Split out of `billing_write_service.rs` like its siblings; `BillingWriteService`'s struct and
//! errors stay there.

use backbone_orm::company_scope;
use chrono::Datelike;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::infrastructure::persistence::{
    NewPaymentTermLineRow, NewPaymentTermRow, PaymentTermRepository, TermHeaderRow, TermLineRow,
};

use super::billing_write_service::{money, BillingError, BillingWriteService};

/// Days-in-month for a (year, month) — used to clamp `day_of_month` targets (Feb 31 → Feb 28/29),
/// the same clamp calendar libraries apply.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if chrono::NaiveDate::from_ymd_opt(year, 2, 29).is_some() {
                29
            } else {
                28
            }
        }
        _ => 30, // unreachable for real months
    }
}

/// Resolve one line's due date from the invoice's posting date.
///
/// - `days`: anchor + `nb_days` calendar days.
/// - `day_current_month`: the `day_of_month` of the anchor month, `nb_days` months on.
/// - `day_following_month`: the `day_of_month` of the month AFTER the anchor month, `nb_days`
///   months on (so `nb_days` doubles as the month offset for the day-shaped delays).
/// Both day shapes clamp to the target month's last day (Jan 31 + 1 month → Feb 28/29).
pub fn line_due_date(
    line: &TermLineRow,
    posting_date: chrono::NaiveDate,
) -> Option<chrono::NaiveDate> {
    // Anchor: the invoice date, or the end of its month.
    let anchor = if line.anchor == "end_of_invoice_month" {
        let dim = days_in_month(posting_date.year(), posting_date.month()) as i32;
        chrono::NaiveDate::from_ymd_opt(posting_date.year(), posting_date.month(), dim as u32)?
    } else {
        posting_date
    };
    match line.delay_type.as_str() {
        "days" => anchor.checked_add_signed(chrono::Duration::days(line.nb_days as i64)),
        "day_current_month" => {
            let dom = line.day_of_month? as u32;
            let shifted =
                anchor.checked_add_months(chrono::Months::new(line.nb_days.max(0) as u32))?;
            let dim = days_in_month(shifted.year(), shifted.month());
            chrono::NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), dom.min(dim))
        }
        "day_following_month" => {
            let dom = line.day_of_month? as u32;
            let shifted =
                anchor.checked_add_months(chrono::Months::new(line.nb_days.max(0) as u32 + 1))?;
            let dim = days_in_month(shifted.year(), shifted.month());
            chrono::NaiveDate::from_ymd_opt(shifted.year(), shifted.month(), dom.min(dim))
        }
        _ => None,
    }
}

/// One term slice as the create API receives it (mirrors `TermLineRow`, minus storage).
#[derive(Debug, Clone)]
pub struct NewTermLine {
    pub value: String,
    pub value_amount: Decimal,
    pub nb_days: i32,
    pub day_of_month: Option<i32>,
    pub delay_type: String,
    pub anchor: String,
    pub sequence: i32,
}

/// Validate a term's shape at create: non-empty name, ≥1 line, ≤1 `balance` slice and it must be
/// the last, percents within (0, 100] summing ≤ 100, day-shaped delays carrying `day_of_month`,
/// and the early-pay-discount block complete when enabled (a discount without an expense account
/// has nowhere to land — we keep no property-fallback like Odoo's ir_property).
/// `reduced_price` is refused outright: applying a discount to the pre-tax base requires
/// recomputing the tax overlay at settlement, which is its own pass.
pub fn validate_term(
    name: &str,
    lines: &[NewTermLine],
    early_discount: bool,
    discount_percent: Decimal,
    discount_days: i32,
    discount_account_id: Option<Uuid>,
    discount_tax_basis: &str,
) -> Result<(), BillingError> {
    if name.trim().is_empty() {
        return Err(term_err(
            "term_name_required",
            "a payment term needs a name",
        ));
    }
    if lines.is_empty() {
        return Err(term_err(
            "term_lines_required",
            "a payment term needs at least one line",
        ));
    }
    if discount_tax_basis == "reduced_price" {
        return Err(term_err(
            "discount_tax_basis_unsupported",
            "discount on the pre-tax base needs the settlement-time tax recompute; use 'included'",
        ));
    }
    if early_discount {
        if discount_percent <= Decimal::ZERO || discount_percent > Decimal::from(100u32) {
            return Err(term_err(
                "discount_percent_out_of_range",
                "early-pay discount percent must be in (0, 100]",
            ));
        }
        if discount_days <= 0 {
            return Err(term_err(
                "discount_days_out_of_range",
                "an early-pay discount needs a positive discount window",
            ));
        }
        if discount_account_id.is_none() {
            return Err(term_err(
                "discount_account_required",
                "an early-pay discount needs an explicit expense account",
            ));
        }
    }
    let mut percent_sum = Decimal::ZERO;
    let mut saw_balance = false;
    for (i, l) in lines.iter().enumerate() {
        let is_last = i == lines.len() - 1;
        match l.value.as_str() {
            "balance" => {
                if saw_balance {
                    return Err(term_err(
                        "term_multiple_balance",
                        "only one balance slice per term",
                    ));
                }
                if !is_last {
                    return Err(term_err(
                        "term_balance_not_last",
                        "the balance slice must be the term's last line",
                    ));
                }
                saw_balance = true;
            }
            "percent" => {
                if l.value_amount <= Decimal::ZERO || l.value_amount > Decimal::from(100u32) {
                    return Err(term_err(
                        "term_percent_out_of_range",
                        "percent slices must be in (0, 100]",
                    ));
                }
                percent_sum += l.value_amount;
            }
            "fixed" => {
                if l.value_amount <= Decimal::ZERO {
                    return Err(term_err(
                        "term_fixed_out_of_range",
                        "fixed slices must be positive",
                    ));
                }
            }
            other => {
                return Err(term_err(
                    "term_value_invalid",
                    format!("unknown term slice value '{other}'"),
                ))
            }
        }
        if l.nb_days < 0 {
            return Err(term_err(
                "term_days_negative",
                "slice day offsets must be ≥ 0",
            ));
        }
        if matches!(
            l.delay_type.as_str(),
            "day_current_month" | "day_following_month"
        ) {
            match l.day_of_month {
                Some(d) if (1..=31).contains(&d) => {}
                _ => {
                    return Err(term_err(
                        "term_day_of_month_required",
                        "day-shaped delays need a day_of_month in 1..=31",
                    ))
                }
            }
        }
        if line_due_date(
            &TermLineRow {
                id: Uuid::nil(),
                value: l.value.clone(),
                value_amount: l.value_amount,
                nb_days: l.nb_days,
                day_of_month: l.day_of_month,
                delay_type: l.delay_type.clone(),
                anchor: l.anchor.clone(),
                sequence: l.sequence,
            },
            chrono::NaiveDate::from_ymd_opt(2026, 1, 31).expect("valid date"),
        )
        .is_none()
        {
            return Err(term_err(
                "term_delay_invalid",
                "the delay/anchor combination resolves no date",
            ));
        }
    }
    if percent_sum > Decimal::from(100u32) {
        return Err(term_err(
            "term_percent_exceeds_total",
            "percent slices must sum to at most 100",
        ));
    }
    Ok(())
}

fn term_err(code: &str, message: impl Into<String>) -> BillingError {
    BillingError::TermInvalid {
        code: code.into(),
        message: message.into(),
    }
}

/// Derive the installment slices for one invoice: term lines + posting date + grand total →
/// `(due_date, amount)` pairs in slice order, Σ amounts == `grand_total` exactly.
///
/// Amount derivation per slice: `fixed` takes its amount; `percent` takes
/// `money(grand × pct / 100)` (smooth-delta: the LAST slice absorbs the rounding remainder, so
/// conservation is exact — the same rule the tax repartition uses); `balance` takes whatever is
/// left. A term without a `balance` slice must cover the total exactly or the derivation refuses
/// (`term_does_not_cover_total`) — the caller never silently loses or invents a remainder.
pub fn derive_schedule(
    lines: &[TermLineRow],
    posting_date: chrono::NaiveDate,
    grand_total: Decimal,
) -> Result<Vec<(chrono::NaiveDate, Decimal)>, BillingError> {
    if grand_total < Decimal::ZERO {
        return Err(BillingError::NegativeAmount);
    }
    let mut slices: Vec<(chrono::NaiveDate, Decimal)> = Vec::with_capacity(lines.len());
    let mut fixed_and_percent = Decimal::ZERO;
    for l in lines {
        let due = line_due_date(l, posting_date).ok_or_else(|| {
            term_err(
                "term_delay_invalid",
                "the delay/anchor combination resolves no date",
            )
        })?;
        let amount = match l.value.as_str() {
            "fixed" => money(l.value_amount),
            "percent" => money(grand_total * l.value_amount / Decimal::from(100u32)),
            // balance placeholder: real amount assigned after the loop
            "balance" => Decimal::ZERO,
            other => {
                return Err(term_err(
                    "term_value_invalid",
                    format!("unknown term slice value '{other}'"),
                ))
            }
        };
        if l.value != "balance" {
            fixed_and_percent += amount;
        }
        slices.push((due, amount));
    }
    // Same-date slices collapse (two 30-day slices of one term are one installment of the sum).
    slices.sort_by(|a, b| a.0.cmp(&b.0));
    let mut collapsed: Vec<(chrono::NaiveDate, Decimal)> = Vec::with_capacity(slices.len());
    for (due, amt) in slices {
        if let Some(last) = collapsed.last_mut() {
            if last.0 == due {
                last.1 += amt;
                continue;
            }
        }
        collapsed.push((due, amt));
    }
    // Assign the balance + absorb the rounding delta into the LAST slice (smooth-delta tail).
    let balance_left = grand_total - fixed_and_percent;
    if balance_left < Decimal::ZERO {
        return Err(term_err(
            "term_exceeds_total",
            "fixed slices alone exceed the invoice total",
        ));
    }
    let has_balance = lines.iter().any(|l| l.value == "balance");
    if !has_balance && !balance_left.is_zero() {
        return Err(term_err(
            "term_does_not_cover_total",
            "the term has no balance slice and does not cover the invoice total",
        ));
    }
    if let Some(last) = collapsed.last_mut() {
        last.1 += balance_left;
    } else {
        collapsed.push((posting_date, grand_total));
    }
    // A zero-value last slice still carries the due date when it is the only one (balance-only term).
    Ok(collapsed)
}

/// Begin a transaction with the company fence bound — the only correct way to reach fenced tables
/// from the pool. A bare pool execute never carries `app.company_id` (the task-local scope alone
/// does not fence raw queries), so every pool-based read or write here goes through this.
async fn scoped_tx(
    pool: &sqlx::PgPool,
    company_id: Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, BillingError> {
    let mut tx = pool.begin().await.map_err(BillingError::Db)?;
    company_scope::bind_company_on(&mut tx, company_id)
        .await
        .map_err(BillingError::Db)?;
    Ok(tx)
}

impl BillingWriteService {
    /// Create a payment term (header + lines) in one transaction. Tenant terms carry the caller's
    /// company; global templates are seeded owner-side (the split fence refuses a tenant forging
    /// `company_id = NULL`). Shape validation runs BEFORE any insert.
    pub async fn create_payment_term(
        &self,
        company_id: Uuid,
        name: &str,
        note: Option<&str>,
        sequence: i32,
        lines: &[NewTermLine],
        early_discount: bool,
        discount_percent: Decimal,
        discount_days: i32,
        discount_account_id: Option<Uuid>,
        discount_tax_basis: &str,
    ) -> Result<Uuid, BillingError> {
        validate_term(
            name,
            lines,
            early_discount,
            discount_percent,
            discount_days,
            discount_account_id,
            discount_tax_basis,
        )?;
        let id = Uuid::new_v4();
        let mut tx = self.db_pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        self.terms
            .insert_term(
                &mut *tx,
                &NewPaymentTermRow {
                    id,
                    company_id: Some(company_id),
                    name,
                    note,
                    sequence,
                    early_discount,
                    discount_percent,
                    discount_days,
                    discount_account_id,
                    discount_tax_basis,
                },
            )
            .await?;
        for l in lines {
            self.terms
                .insert_term_line(
                    &mut *tx,
                    &NewPaymentTermLineRow {
                        id: Uuid::new_v4(),
                        term_id: id,
                        company_id: Some(company_id),
                        value: &l.value,
                        value_amount: l.value_amount,
                        nb_days: l.nb_days,
                        day_of_month: l.day_of_month,
                        delay_type: &l.delay_type,
                        anchor: &l.anchor,
                        sequence: l.sequence,
                    },
                )
                .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    /// List the terms a company may pick from: its own plus the global templates, active only,
    /// globals first. Served on a company-scoped session so the split fence admits both sources
    /// while an unscoped session sees neither.
    pub async fn list_payment_terms(
        &self,
        company_id: Uuid,
    ) -> Result<Vec<TermHeaderRow>, BillingError> {
        let mut tx = scoped_tx(&self.db_pool, company_id).await?;
        let rows = self.terms.list_terms(&mut *tx, company_id).await?;
        tx.commit().await.map_err(BillingError::Db)?;
        Ok(rows)
    }

    /// Soft-retire (or re-activate) a term. Historical invoices keep their materialized schedule;
    /// the term just leaves new-invoice selection. Own-company rows only — a global template or
    /// another company's term is invisible to the scoped UPDATE and reads as `TermNotFound`.
    pub async fn set_payment_term_status(
        &self,
        company_id: Uuid,
        term_id: Uuid,
        status: &str,
    ) -> Result<u64, BillingError> {
        let mut tx = scoped_tx(&self.db_pool, company_id).await?;
        let affected = self.terms.set_status(&mut *tx, term_id, status).await?;
        tx.commit().await.map_err(BillingError::Db)?;
        Ok(affected)
    }

    /// Preview a term's due dates for a hypothetical invoice (pure — nothing persists). The
    /// read-only seam the pickers use; the same derivation the post hook materializes.
    pub async fn preview_payment_term(
        &self,
        company_id: Uuid,
        term_id: Uuid,
        posting_date: chrono::NaiveDate,
        grand_total: Decimal,
    ) -> Result<Vec<(chrono::NaiveDate, Decimal)>, BillingError> {
        let mut tx = scoped_tx(&self.db_pool, company_id).await?;
        let fetched = self.terms.fetch_term(&mut *tx, term_id).await?;
        tx.commit().await.map_err(BillingError::Db)?;
        let Some((header, lines)) = fetched else {
            return Err(BillingError::TermNotFound(term_id));
        };
        if header.status != "active" {
            return Err(term_err(
                "term_inactive",
                "the term is retired; pick an active one",
            ));
        }
        derive_schedule(&lines, posting_date, grand_total)
    }

    /// The invoice-post hook: materialize one invoice's term. Rides the caller's post transaction
    /// (company already bound). Refuses the two conflicts — a manual `due_date` competing with the
    /// term's derivation, and pre-existing schedule rows — then stamps the derived `due_date` +
    /// EPD block and seeds the installment rows when the term splits the balance.
    ///
    /// A single-slice term writes ONLY the due date (one installment would duplicate the header);
    /// a multi-slice term writes the rows and the header's `due_date` = the LAST slice's date
    /// (the whole-balance deadline, the Odoo shape).
    pub(super) async fn apply_term_on_post(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        table: &str,
        invoice_id: Uuid,
        company_id: Uuid,
        term_id: Uuid,
        posting_date: chrono::NaiveDate,
        grand_total: Decimal,
        manual_due_date: Option<chrono::NaiveDate>,
        has_manual_schedules: bool,
    ) -> Result<(), BillingError> {
        let Some((header, lines)) = self.terms.fetch_term(&mut **tx, term_id).await? else {
            return Err(BillingError::TermNotFound(term_id));
        };
        // The split fence read admits global templates; an own-company term passes trivially. A
        // term belonging to ANOTHER company is invisible here → TermNotFound, never cross-tenant.
        if header.company_id.is_some() && header.company_id != Some(company_id) {
            return Err(BillingError::TermNotFound(term_id));
        }
        if manual_due_date.is_some() {
            return Err(term_err(
                "due_date_conflicts_with_term",
                "an invoice with a payment term derives its due date; drop the manual due_date",
            ));
        }
        if has_manual_schedules {
            return Err(term_err(
                "schedule_conflicts_with_term",
                "an invoice with a payment term derives its installments; drop the manual schedule",
            ));
        }
        let schedule = derive_schedule(&lines, posting_date, grand_total)?;
        let due_date = schedule.last().map(|s| s.0).unwrap_or(posting_date);
        let (epd_percent, epd_deadline, epd_account) = if header.early_discount {
            (
                header.discount_percent,
                Some(posting_date + chrono::Duration::days(header.discount_days as i64)),
                header.discount_account_id,
            )
        } else {
            (Decimal::ZERO, None, None)
        };
        self.terms
            .materialize_term_on_invoice(
                &mut **tx,
                table,
                invoice_id,
                term_id,
                due_date,
                epd_percent,
                epd_deadline,
                epd_account,
            )
            .await?;
        if schedule.len() > 1 {
            for (i, (due, amt)) in schedule.iter().enumerate() {
                self.schedules
                    .insert_schedule(
                        &mut **tx,
                        &crate::infrastructure::persistence::NewPaymentScheduleRow {
                            id: Uuid::new_v4(),
                            invoice_ref: invoice_id,
                            kind: if table == "sales_invoices" {
                                "sales"
                            } else {
                                "purchase"
                            },
                            company_id,
                            installment_no: (i + 1) as i32,
                            due_date: *due,
                            amount: *amt,
                        },
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

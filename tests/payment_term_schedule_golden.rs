//! Payment-term derivation goldens: the anchor/delay matrix, smooth-delta conservation, and the
//! invoice-post materialization (due date + EPD block + seeded installments).
//! Pure cases need no DB; the post-hook case requires DATABASE_URL (:5433/backbone_billing).

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope, GlPostAck, GlPostRejected, GlPostSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingWriteService, NewInvoiceLine, NewSalesInvoice,
};
use backbone_billing::application::service::term_schedule::{
    derive_schedule, line_due_date, NewTermLine,
};
use backbone_billing::infrastructure::persistence::TermLineRow;

fn d(s: &str) -> Decimal {
    Decimal::from_str_exact(s).unwrap()
}
fn day(y: i32, m: u32, dd: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(y, m, dd).unwrap()
}
fn uq(p: &str) -> String {
    format!("{p}-{}", &Uuid::new_v4().simple().to_string()[..8])
}
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/backbone_billing".to_string()
    });
    PgPool::connect(&url).await.expect("connect DB")
}

fn line(
    value: &str,
    value_amount: Decimal,
    nb_days: i32,
    delay_type: &str,
    anchor: &str,
) -> TermLineRow {
    TermLineRow {
        id: Uuid::nil(),
        value: value.into(),
        value_amount,
        nb_days,
        day_of_month: None,
        delay_type: delay_type.into(),
        anchor: anchor.into(),
        sequence: 10,
    }
}

// ---- pure derivation ---------------------------------------------------------

// GD-1: a single balance line is the whole total on the derived date (no installment rows ever).
#[test]
fn single_balance_line_sets_one_date() {
    let sched = derive_schedule(
        &[line("balance", Decimal::ZERO, 30, "days", "invoice_date")],
        day(2026, 7, 5),
        d("1000"),
    )
    .unwrap();
    assert_eq!(sched, vec![(day(2026, 8, 4), d("1000.00"))]);
}

// GD-2: anchor end-of-invoice-month + day_following_month — the B2B "10th of next month" shape,
// including the Jan-31 → Feb-28 month-end clamp.
#[test]
fn end_of_month_anchor_and_day_clamp() {
    let l = line(
        "balance",
        Decimal::ZERO,
        0,
        "day_following_month",
        "end_of_invoice_month",
    );
    let mut row = l;
    row.day_of_month = Some(31);
    // Invoice Jan 31 2026 → anchor Jan 31 → following month = Feb → clamp to Feb 28.
    assert_eq!(
        line_due_date(&row, day(2026, 1, 31)),
        Some(day(2026, 2, 28))
    );
    // Invoice Feb 15 → anchor Feb 28 → following month = Mar 31 (no clamp needed).
    assert_eq!(
        line_due_date(&row, day(2026, 2, 15)),
        Some(day(2026, 3, 31))
    );

    let mut cur = line(
        "balance",
        Decimal::ZERO,
        1,
        "day_current_month",
        "end_of_invoice_month",
    );
    cur.day_of_month = Some(10);
    // day_current_month: day 10 of the anchor month + 1 month.
    assert_eq!(
        line_due_date(&cur, day(2026, 1, 31)),
        Some(day(2026, 2, 10))
    );
}

// GD-3: smooth-delta conservation — three percent slices on an awkward total still sum EXACTLY to
// the grand total; the last slice absorbs the rounding remainder.
#[test]
fn percent_slices_conserve_exactly() {
    let grand = d("100000.01");
    let sched = derive_schedule(
        &[
            line("percent", d("33.3333"), 15, "days", "invoice_date"),
            line("percent", d("33.3333"), 30, "days", "invoice_date"),
            line("balance", Decimal::ZERO, 45, "days", "invoice_date"),
        ],
        day(2026, 7, 5),
        grand,
    )
    .unwrap();
    assert_eq!(sched.len(), 3);
    let sum: Decimal = sched.iter().map(|s| s.1).sum();
    assert_eq!(sum, grand);
    // The tail absorbed the delta (33.3333% of 100000.01 is not a clean cent).
    assert_eq!(sched[2].0, day(2026, 8, 19));
}

// GD-4: same-date slices collapse into one installment of the sum.
#[test]
fn same_date_slices_collapse() {
    let sched = derive_schedule(
        &[
            line("fixed", d("400"), 30, "days", "invoice_date"),
            line("percent", d("30"), 30, "days", "invoice_date"),
            line("balance", Decimal::ZERO, 30, "days", "invoice_date"),
        ],
        day(2026, 7, 5),
        d("1000"),
    )
    .unwrap();
    assert_eq!(sched.len(), 1);
    assert_eq!(sched[0], (day(2026, 8, 4), d("1000.00")));
}

// GD-5: fixed slices beyond the total refuse; no-balance terms that under-cover refuse.
#[test]
fn over_and_under_coverage_refuse() {
    let over = derive_schedule(
        &[
            line("fixed", d("600"), 10, "days", "invoice_date"),
            line("fixed", d("600"), 20, "days", "invoice_date"),
        ],
        day(2026, 7, 5),
        d("1000"),
    )
    .unwrap_err();
    assert_eq!(over.code(), "term_exceeds_total");

    let under = derive_schedule(
        &[line("fixed", d("900"), 10, "days", "invoice_date")],
        day(2026, 7, 5),
        d("1000"),
    )
    .unwrap_err();
    assert_eq!(under.code(), "term_does_not_cover_total");
}

// ---- post-hook materialization (DB) ------------------------------------------

struct OkGl {
    journal: Uuid,
    post: Uuid,
}
#[async_trait::async_trait]
impl GlPostSink for OkGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        Ok(GlPostAck {
            post_id: self.post,
            journal_id: self.journal,
            idempotent_reuse: false,
        })
    }
}

// GD-6: posting an invoice whose term splits the balance seeds the installments, stamps the
// header's due date with the LAST slice, and materializes the early-pay-discount block
// (percent + deadline = posting_date + discount_days + account) — all inside the post commit.
#[tokio::test]
async fn post_materializes_term_schedule_and_epd() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let epd_account = Uuid::new_v4();

    let term = w
        .create_payment_term(
            company,
            &uq("TERM"),
            None,
            10,
            &[
                NewTermLine {
                    value: "percent".into(),
                    value_amount: d("30"),
                    nb_days: 30,
                    day_of_month: None,
                    delay_type: "days".into(),
                    anchor: "invoice_date".into(),
                    sequence: 10,
                },
                NewTermLine {
                    value: "balance".into(),
                    value_amount: Decimal::ZERO,
                    nb_days: 60,
                    day_of_month: None,
                    delay_type: "days".into(),
                    anchor: "invoice_date".into(),
                    sequence: 20,
                },
            ],
            true,   // early_discount
            d("2"), // 2%
            10,     // window: 10 days
            Some(epd_account),
            "included",
        )
        .await
        .unwrap();

    let posting = day(2026, 7, 5);
    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let inv = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: posting,
            due_date: None, // the term derives it
            payment_term_id: Some(term),
            currency: None,
            receivable_account_id: ar,
            lines: vec![NewInvoiceLine {
                item_id: item,
                account_id: rev,
                description: None,
                quantity: d("1"),
                unit_price: d("1000000"),
                tax_template_id: None,
            }],
            tax_lines: vec![],
        })
        .await
        .unwrap();

    w.post_sales_invoice(
        inv,
        &OkGl {
            journal: Uuid::new_v4(),
            post: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    let hdr: (
        Option<chrono::NaiveDate>,
        Option<Uuid>,
        Decimal,
        Option<chrono::NaiveDate>,
        Option<Uuid>,
    ) = sqlx::query_as(
        "SELECT due_date, payment_term_id, early_pay_discount_percent, \
                    early_pay_discount_deadline, early_pay_discount_account_id \
             FROM billing.sales_invoices WHERE id=$1",
    )
    .bind(inv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(hdr.0, Some(day(2026, 9, 3))); // last slice: Jul 5 + 60d
    assert_eq!(hdr.1, Some(term));
    assert_eq!(hdr.2, d("2.0000"));
    assert_eq!(hdr.3, Some(day(2026, 7, 15))); // posting + 10d
    assert_eq!(hdr.4, Some(epd_account));

    let rows: Vec<(i32, chrono::NaiveDate, Decimal, String)> = sqlx::query_as(
        "SELECT installment_no, due_date, amount, status::text FROM billing.payment_schedules \
         WHERE invoice_ref=$1 ORDER BY installment_no",
    )
    .bind(inv)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        (1, day(2026, 8, 4), d("300000.00"), "unpaid".to_string())
    );
    assert_eq!(
        rows[1],
        (2, day(2026, 9, 3), d("700000.00"), "unpaid".to_string())
    );
}

// GD-7: a single-slice term writes ONLY the due date — no schedule row duplicating the header.
#[tokio::test]
async fn single_slice_term_writes_due_date_only() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let term = w
        .create_payment_term(
            company,
            &uq("TERM"),
            None,
            10,
            &[NewTermLine {
                value: "balance".into(),
                value_amount: Decimal::ZERO,
                nb_days: 30,
                day_of_month: None,
                delay_type: "days".into(),
                anchor: "invoice_date".into(),
                sequence: 10,
            }],
            false,
            Decimal::ZERO,
            0,
            None,
            "included",
        )
        .await
        .unwrap();

    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let inv = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(2026, 7, 5),
            due_date: None,
            payment_term_id: Some(term),
            currency: None,
            receivable_account_id: ar,
            lines: vec![NewInvoiceLine {
                item_id: item,
                account_id: rev,
                description: None,
                quantity: d("1"),
                unit_price: d("500"),
                tax_template_id: None,
            }],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    w.post_sales_invoice(
        inv,
        &OkGl {
            journal: Uuid::new_v4(),
            post: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    let (due,): (Option<chrono::NaiveDate>,) =
        sqlx::query_as("SELECT due_date FROM billing.sales_invoices WHERE id=$1")
            .bind(inv)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(due, Some(day(2026, 8, 4)));
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM billing.payment_schedules WHERE invoice_ref=$1")
            .bind(inv)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 0);
}

// GD-8: the two conflicts refuse — a manual due_date competing with the term's derivation (at
// create), and a manual schedule competing with the term's installments (after post).
#[tokio::test]
async fn manual_due_date_and_schedule_conflicts_refuse() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let term = w
        .create_payment_term(
            company,
            &uq("TERM"),
            None,
            10,
            &[NewTermLine {
                value: "balance".into(),
                value_amount: Decimal::ZERO,
                nb_days: 30,
                day_of_month: None,
                delay_type: "days".into(),
                anchor: "invoice_date".into(),
                sequence: 10,
            }],
            false,
            Decimal::ZERO,
            0,
            None,
            "included",
        )
        .await
        .unwrap();

    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let line = NewInvoiceLine {
        item_id: item,
        account_id: rev,
        description: None,
        quantity: d("1"),
        unit_price: d("500"),
        tax_template_id: None,
    };
    let refuse = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(2026, 7, 5),
            due_date: Some(day(2026, 8, 1)), // competes with the term
            payment_term_id: Some(term),
            currency: None,
            receivable_account_id: ar,
            lines: vec![line.clone()],
            tax_lines: vec![],
        })
        .await
        .unwrap_err();
    assert_eq!(refuse.code(), "due_date_conflicts_with_term");

    // Term + manual schedule: create WITHOUT a due date, post, then attach a schedule → refuse.
    let inv = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(2026, 7, 5),
            due_date: None,
            payment_term_id: Some(term),
            currency: None,
            receivable_account_id: Uuid::new_v4(),
            lines: vec![line],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    w.post_sales_invoice(
        inv,
        &OkGl {
            journal: Uuid::new_v4(),
            post: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();
    let refuse = w
        .add_payment_schedule(inv, "sales", company, &[(day(2026, 8, 1), d("500"))])
        .await
        .unwrap_err();
    assert_eq!(refuse.code(), "schedule_conflicts_with_term");
}

//! The early-pay-discount resolver: the settlement seam payment calls. Window boundary, posting
//! state, outstanding, account presence, tenant fence, and both invoice kinds.
//! Requires DATABASE_URL (:5433/backbone_billing).

use std::sync::Arc;

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope, GlPostAck, GlPostRejected, GlPostSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewSalesInvoice,
};
use backbone_billing::application::service::term_schedule::NewTermLine;

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

/// A term with a 2% / 10-day discount window + the invoice posted under it.
async fn posted_with_epd(w: &BillingWriteService, company: Uuid, kind: &str) -> Uuid {
    let epd_account = Uuid::new_v4();
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
            true,
            d("2"),
            10,
            Some(epd_account),
            "included",
        )
        .await
        .unwrap();
    let line = NewInvoiceLine {
        item_id: Uuid::new_v4(),
        account_id: Uuid::new_v4(),
        description: None,
        quantity: d("1"),
        unit_price: d("1000000"),
        tax_template_id: None,
    };
    let inv = if kind == "sales" {
        let id = w
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
            id,
            &OkGl {
                journal: Uuid::new_v4(),
                post: Uuid::new_v4(),
            },
        )
        .await
        .unwrap();
        id
    } else {
        let id = w
            .create_purchase_invoice(NewPurchaseInvoice {
                invoice_number: uq("PI"),
                company_id: company,
                branch_id: None,
                supplier_id: Uuid::new_v4(),
                source_po_id: None,
                posting_date: day(2026, 7, 5),
                due_date: None,
                payment_term_id: Some(term),
                currency: None,
                payable_account_id: Uuid::new_v4(),
                lines: vec![line],
                tax_lines: vec![],
            })
            .await
            .unwrap();
        w.post_purchase_invoice(
            id,
            &OkGl {
                journal: Uuid::new_v4(),
                post: Uuid::new_v4(),
            },
        )
        .await
        .unwrap();
        id
    };
    inv
}

// EPD-1: the window is inclusive of the deadline day; one day past → no discount. The decision
// carries PERCENT + account (payment computes the amount on what it allocates).
#[tokio::test]
async fn window_is_deadline_inclusive() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let inv = posted_with_epd(&w, company, "sales").await;

    let on_deadline = w
        .resolve_early_pay_discount(company, inv, "sales", day(2026, 7, 15))
        .await
        .unwrap()
        .expect("discount applies on the deadline day");
    assert_eq!(on_deadline.percent, d("2.0000"));
    assert_eq!(on_deadline.invoice_ref, inv);
    assert_ne!(
        on_deadline.account_id,
        Uuid::nil(),
        "carries the expense account"
    );

    let past = w
        .resolve_early_pay_discount(company, inv, "sales", day(2026, 7, 16))
        .await
        .unwrap();
    assert!(past.is_none(), "one day past the window — full amount only");
}

// EPD-2: purchase-side resolution mirrors sales (A/P discounts are supplier discounts).
#[tokio::test]
async fn purchase_kind_resolves() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let inv = posted_with_epd(&w, company, "purchase").await;
    let r = w
        .resolve_early_pay_discount(company, inv, "purchase", day(2026, 7, 10))
        .await
        .unwrap()
        .expect("purchase discount applies");
    assert_eq!(r.invoice_kind, "purchase");
}

// EPD-3: unknown invoice, draft invoice, fully-settled invoice, wrong tenant → all resolve None
// (never an error: "no discount" is a legitimate answer; the settlement itself 404s if the
// invoice truly does not exist).
#[tokio::test]
async fn non_applicable_states_resolve_none() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let other = Uuid::new_v4();
    let inv = posted_with_epd(&w, company, "sales").await;

    // unknown invoice
    assert!(w
        .resolve_early_pay_discount(company, Uuid::new_v4(), "sales", day(2026, 7, 10))
        .await
        .unwrap()
        .is_none());
    // fully settled inside the window
    sqlx::query(
        "UPDATE billing.sales_invoices SET outstanding_amount=0, status='paid' WHERE id=$1",
    )
    .bind(inv)
    .execute(&pool)
    .await
    .unwrap();
    assert!(w
        .resolve_early_pay_discount(company, inv, "sales", day(2026, 7, 10))
        .await
        .unwrap()
        .is_none());
}

// EPD-4: the tenant leg — a restricted (non-BYPASSRLS) service must resolve the owning company's
// discount and resolve NOTHING for a different company. (The default test pool is superuser,
// which bypasses RLS by construction, so this leg needs the real posture.)
#[tokio::test]
async fn tenant_fence_resolves_none_for_other_company() {
    let admin = pool().await;
    let w = BillingWriteService::new(admin.clone());
    let company = Uuid::new_v4();
    let inv = posted_with_epd(&w, company, "sales").await;

    sqlx::query("SELECT pg_advisory_lock(hashtext('billing_fence_probe'))")
        .execute(&admin)
        .await
        .unwrap();
    let _ = sqlx::query(
        "CREATE ROLE billing_fence_probe LOGIN PASSWORD 'probe' \
           NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE",
    )
    .execute(&admin)
    .await;
    for grant in [
        "GRANT CONNECT ON DATABASE backbone_billing TO billing_fence_probe",
        "GRANT USAGE ON SCHEMA billing TO billing_fence_probe",
        "GRANT SELECT ON TABLE billing.sales_invoices TO billing_fence_probe",
        "GRANT SELECT, INSERT, UPDATE ON TABLE billing.payment_terms TO billing_fence_probe",
        "GRANT SELECT, INSERT, UPDATE ON TABLE billing.payment_term_lines TO billing_fence_probe",
    ] {
        sqlx::query(grant).execute(&admin).await.unwrap();
    }
    sqlx::query("SELECT pg_advisory_unlock(hashtext('billing_fence_probe'))")
        .execute(&admin)
        .await
        .unwrap();

    let probe =
        PgPool::connect("postgresql://billing_fence_probe:probe@localhost:5433/backbone_billing")
            .await
            .expect("connect as restricted probe");
    let w_probe = BillingWriteService::new(probe);
    let when = day(2026, 7, 10);
    assert!(
        w_probe
            .resolve_early_pay_discount(company, inv, "sales", when)
            .await
            .unwrap()
            .is_some(),
        "owning company resolves under the restricted role"
    );
    assert!(
        w_probe
            .resolve_early_pay_discount(Uuid::new_v4(), inv, "sales", when)
            .await
            .unwrap()
            .is_none(),
        "another company resolves nothing — the invoice fence hides the row"
    );
}

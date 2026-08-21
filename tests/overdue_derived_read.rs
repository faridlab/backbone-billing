//! The derived overdue read: open, GL-posted invoices past due — both kinds, exclusions, ordering.
//! No stored `overdue` flag exists anywhere; every case here pins that the filter recomputes from
//! live state (settle an invoice → it leaves the list with no flag to update).
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
fn gl() -> OkGl {
    OkGl {
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    }
}

async fn posted_sales(
    w: &BillingWriteService,
    company: Uuid,
    posting: chrono::NaiveDate,
    due: chrono::NaiveDate,
) -> Uuid {
    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let inv = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: posting,
            due_date: Some(due),
            payment_term_id: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![NewInvoiceLine {
                item_id: item,
                account_id: rev,
                description: None,
                quantity: d("1"),
                unit_price: d("1000"),
                tax_template_id: None,
            }],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    w.post_sales_invoice(inv, &gl()).await.unwrap();
    inv
}

async fn posted_purchase(
    w: &BillingWriteService,
    company: Uuid,
    posting: chrono::NaiveDate,
    due: chrono::NaiveDate,
) -> Uuid {
    let (item, exp, ap) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let inv = w
        .create_purchase_invoice(NewPurchaseInvoice {
            invoice_number: uq("PI"),
            company_id: company,
            branch_id: None,
            supplier_id: Uuid::new_v4(),
            source_po_id: None,
            posting_date: posting,
            due_date: Some(due),
            payment_term_id: None,
            currency: None,
            payable_account_id: ap,
            lines: vec![NewInvoiceLine {
                item_id: item,
                account_id: exp,
                description: None,
                quantity: d("1"),
                unit_price: d("2000"),
                tax_template_id: None,
            }],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    w.post_purchase_invoice(inv, &gl()).await.unwrap();
    inv
}

// OD-1: both kinds appear, earliest due first; settled and future-due and draft invoices do not.
#[tokio::test]
async fn overdue_lists_open_posted_both_kinds() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let today = day(2026, 8, 15);

    let od_sales = posted_sales(&w, company, day(2026, 6, 1), day(2026, 6, 30)).await;
    let od_purchase = posted_purchase(&w, company, day(2026, 6, 1), day(2026, 7, 15)).await;
    let future = posted_sales(&w, company, day(2026, 8, 1), day(2026, 9, 30)).await;
    let _draft = {
        // never posted → stays draft, GL-invisible even though past due
        let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        w.create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(2026, 6, 1),
            due_date: Some(day(2026, 6, 15)),
            payment_term_id: None,
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
        .unwrap()
    };
    let settled = posted_sales(&w, company, day(2026, 6, 1), day(2026, 6, 30)).await;
    // Pay one off entirely (direct state write: this is a derived-read test, not a settlement test).
    sqlx::query(
        "UPDATE billing.sales_invoices SET outstanding_amount=0, status='paid' WHERE id=$1",
    )
    .bind(settled)
    .execute(&pool)
    .await
    .unwrap();

    let rows = w.list_overdue_invoices(company, today).await.unwrap();
    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    assert_eq!(
        ids,
        vec![od_sales, od_purchase],
        "earliest due first, both kinds"
    );
    assert!(!ids.contains(&future) && !ids.contains(&settled));

    let by_kind: Vec<(&Uuid, &str, i64, Decimal)> = rows
        .iter()
        .map(|r| {
            (
                &r.id,
                r.kind.as_str(),
                (today - r.due_date).num_days(),
                r.outstanding_amount,
            )
        })
        .collect();
    assert_eq!(by_kind[0], (&od_sales, "sales", 46, d("1000.00")));
    assert_eq!(by_kind[1], (&od_purchase, "purchase", 31, d("2000.00")));
}

// OD-2: the tenant fence — another company's overdue invoice is invisible.
#[tokio::test]
async fn overdue_is_company_scoped() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    posted_sales(&w, b, day(2026, 6, 1), day(2026, 6, 30)).await;

    let rows = w.list_overdue_invoices(a, day(2026, 8, 15)).await.unwrap();
    assert!(
        rows.is_empty(),
        "company A must not see B's overdue invoices"
    );
}

//! Payment schedules + the billing event surface (the public extension points). Billing-only.
//! Requires DATABASE_URL (:5433/backbone_billing).

use std::sync::{Arc, Mutex};

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use backbone_billing::application::service::billing_events::{BillingEvent, BillingEventSink};
use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope, GlPostAck, GlPostRejected, GlPostSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingWriteService, NewInvoiceLine, NewSalesInvoice,
};

fn d(s: &str) -> Decimal {
    Decimal::from_str_exact(s).unwrap()
}
fn day() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap()
}
fn due(n: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 8, n).unwrap()
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

#[derive(Default, Clone)]
struct Recorder {
    events: Arc<Mutex<Vec<BillingEvent>>>,
}
impl BillingEventSink for Recorder {
    fn publish(&self, e: BillingEvent) {
        self.events.lock().unwrap().push(e);
    }
}
#[derive(Clone)]
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

async fn sales(w: &BillingWriteService, company: Uuid, so: Option<Uuid>) -> Uuid {
    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    w.create_sales_invoice(NewSalesInvoice {
        invoice_number: uq("SI"),
        company_id: company,
        branch_id: None,
        customer_id: Uuid::new_v4(),
        source_so_id: so,
        posting_date: day(),
        due_date: None,
        payment_term_id: None,
        currency: None,
        receivable_account_id: ar,
        lines: vec![NewInvoiceLine {
            item_id: item,
            account_id: rev,
            description: None,
            quantity: d("1"),
            unit_price: d("300000"),
            tax_template_id: None,
        }],
        tax_lines: vec![],
    })
    .await
    .unwrap()
}

// SE-1: three installments attach to an invoice, numbered 1..3, all unpaid, summing the grand.
#[tokio::test]
async fn payment_schedule_installments() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let inv = sales(&w, company, None).await;
    w.add_payment_schedule(
        inv,
        "sales",
        company,
        &[
            (due(1), d("100000")),
            (due(15), d("100000")),
            (due(28), d("100000")),
        ],
    )
    .await
    .unwrap();
    let rows: Vec<(i32, Decimal, String)> = sqlx::query_as(
        "SELECT installment_no, amount, status::text FROM billing.payment_schedules WHERE invoice_ref=$1 ORDER BY installment_no")
        .bind(inv).fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert!(rows.iter().all(|r| r.2 == "unpaid"));
    assert_eq!(rows.iter().map(|r| r.1).sum::<Decimal>(), d("300000.00"));
}

// SE-2: posting a sales invoice emits SalesInvoicePosted carrying the source SO id + grand total —
// the public hook a faktur-pajak / loyalty overlay subscribes to.
#[tokio::test]
async fn sales_invoice_posted_event_is_emitted() {
    let pool = pool().await;
    let rec = Recorder::default();
    let w = BillingWriteService::with_sink(pool.clone(), Arc::new(rec.clone()));
    let company = Uuid::new_v4();
    let so = Uuid::new_v4();
    let inv = sales(&w, company, Some(so)).await;
    w.post_sales_invoice(
        inv,
        &OkGl {
            journal: Uuid::new_v4(),
            post: Uuid::new_v4(),
        },
    )
    .await
    .unwrap();

    let evts = rec.events.lock().unwrap().clone();
    let posted = evts
        .iter()
        .find_map(|e| match e {
            BillingEvent::SalesInvoicePosted(p) if p.invoice_id == inv => Some(p.clone()),
            _ => None,
        })
        .expect("SalesInvoicePosted emitted");
    assert_eq!(posted.source_so_id, Some(so));
    assert_eq!(posted.grand_total, d("300000.00"));
    assert_eq!(posted.company_id, company);
}

//! Integrity probes for billing — the invariants that must hold against a REAL Postgres, beyond the
//! golden math. Requires DATABASE_URL (:5433/backbone_billing).

use std::sync::{Arc, Mutex};

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use backbone_billing::application::service::billing_events::{BillingEvent, BillingEventSink};
use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope, GlPostAck, GlPostRejected, GlPostSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingError, BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewSalesInvoice, NewTaxLine,
};

fn d(s: &str) -> Decimal { Decimal::from_str_exact(s).unwrap() }
fn day() -> chrono::NaiveDate { chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap() }
fn uq(p: &str) -> String { format!("{p}-{}", &Uuid::new_v4().simple().to_string()[..8]) }
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/backbone_billing".to_string());
    PgPool::connect(&url).await.expect("connect DB")
}

/// A sink that ALWAYS rejects — proves a rejected post never marks the invoice posted, and the
/// failure is recoverable (posting_state=failed, retryable).
struct RejectingGl;
#[async_trait::async_trait]
impl GlPostSink for RejectingGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        Err(GlPostRejected { code: "period_closed".into(), message: "accounting period is closed".into() })
    }
}
/// A sink that records + acks — for the retry-after-failure probe.
#[derive(Default, Clone)]
struct OkGl { hits: Arc<Mutex<usize>>, journal: Uuid, post: Uuid }
#[async_trait::async_trait]
impl GlPostSink for OkGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        *self.hits.lock().unwrap() += 1;
        Ok(GlPostAck { post_id: self.post, journal_id: self.journal, idempotent_reuse: false })
    }
}

fn line(item: Uuid, acct: Uuid, qty: &str, price: &str) -> NewInvoiceLine {
    NewInvoiceLine { item_id: item, account_id: acct, description: None, quantity: d(qty), unit_price: d(price) }
}

async fn draft_sales(w: &BillingWriteService, company: Uuid, currency: Option<String>, tax: Vec<NewTaxLine>) -> Uuid {
    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    w.create_sales_invoice(NewSalesInvoice {
        invoice_number: uq("SI"), company_id: company, branch_id: None, customer_id: Uuid::new_v4(),
        source_so_id: None, posting_date: day(), due_date: None, currency, receivable_account_id: ar,
        lines: vec![line(item, rev, "1", "100000")], tax_lines: tax,
    }).await.unwrap()
}

// IP-1: a rejected GL post leaves the invoice NOT posted and RECOVERABLE — posting_state=failed,
// status still draft, no journal id. A later successful post then completes it.
#[tokio::test]
async fn rejected_post_is_recoverable() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let id = draft_sales(&w, Uuid::new_v4(), None, vec![]).await;

    let e = w.post_sales_invoice(id, &RejectingGl).await.unwrap_err();
    assert!(matches!(e, BillingError::GlRejected { .. }));
    let (ps, st, jid): (String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT posting_state::text, status::text, journal_id FROM billing.sales_invoices WHERE id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(ps, "failed");
    assert_eq!(st, "draft", "a failed post must not submit the invoice");
    assert!(jid.is_none());

    // Recovery: a working sink posts it cleanly.
    let ok = OkGl { hits: Arc::new(Mutex::new(0)), journal: Uuid::new_v4(), post: Uuid::new_v4() };
    w.post_sales_invoice(id, &ok).await.unwrap();
    let (ps2, st2): (String, String) = sqlx::query_as("SELECT posting_state::text, status::text FROM billing.sales_invoices WHERE id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(ps2, "posted");
    assert_eq!(st2, "submitted");
    assert_eq!(*ok.hits.lock().unwrap(), 1);
}

// IP-2: a non-IDR invoice is refused at post time (currency is not yet supported end-to-end); the
// document persists but no unbalanced/mis-valued post reaches the ledger.
#[tokio::test]
async fn non_idr_currency_refused_at_post() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let id = draft_sales(&w, Uuid::new_v4(), Some("USD".into()), vec![]).await;
    let ok = OkGl { hits: Arc::new(Mutex::new(0)), journal: Uuid::new_v4(), post: Uuid::new_v4() };
    let e = w.post_sales_invoice(id, &ok).await.unwrap_err();
    assert!(matches!(e, BillingError::UnsupportedCurrency(c) if c == "USD"));
    assert_eq!(*ok.hits.lock().unwrap(), 0, "the sink is never reached for an unsupported currency");
}

// IP-3: build_ar_post is self-balancing regardless of a supplied tax line's own account — the A/R
// debit always equals net + Σoutput. (A tampered tax_amount that broke balance would be caught by
// is_balanced → UnbalancedPost rather than posting a broken journal.)
#[tokio::test]
async fn ar_post_is_balanced_with_tax() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let ppn = Uuid::new_v4();
    let id = draft_sales(&w, Uuid::new_v4(), None, vec![
        NewTaxLine { account_id: ppn, basis: "output".into(), description: None, rate: d("11"), tax_amount: d("11000") },
    ]).await;
    let env = w.build_ar_post(id).await.unwrap();
    assert!(env.is_balanced());
    let (dr, cr) = env.totals();
    assert_eq!((dr, cr), (d("111000.00"), d("111000.00")));
    // grand persisted = net + output.
    let grand: Decimal = sqlx::query_scalar("SELECT grand_total FROM billing.sales_invoices WHERE id=$1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(grand, d("111000.00"));
}

/// A GL sink that parks on a barrier BEFORE returning the ack, so two concurrent posts are both
/// guaranteed to be past `short_circuit_posted` (which sees `pending` for both) at the same instant —
/// making the pending→posted UPDATE race deterministic.
#[derive(Clone)]
struct BarrierGl { gate: Arc<tokio::sync::Barrier>, journal: Uuid, post: Uuid }
#[async_trait::async_trait]
impl GlPostSink for BarrierGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        self.gate.wait().await; // both callers meet here, having already cleared short_circuit
        // Accounting dedupes on source_id, so a real ledger returns a valid ack to BOTH racers.
        Ok(GlPostAck { post_id: self.post, journal_id: self.journal, idempotent_reuse: false })
    }
}
#[derive(Default, Clone)]
struct Recorder { events: Arc<Mutex<Vec<BillingEvent>>> }
impl BillingEventSink for Recorder {
    fn publish(&self, e: BillingEvent) { self.events.lock().unwrap().push(e); }
}

// IP-4 (council 2026-07-05, skeptic): the seam event is emitted EXACTLY once even under a concurrent
// double-post. Without gating the publish on the pending→posted UPDATE's rows_affected, both racers
// would publish `PurchaseInvoicePosted`, double-advancing the source PO's billed_qty via
// buying::mark_billed and corrupting the 3-way match billing exists to close.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_post_emits_the_seam_event_once() {
    let pool = pool().await;
    let rec = Recorder::default();
    let w = Arc::new(BillingWriteService::with_sink(pool.clone(), Arc::new(rec.clone())));
    let (company, item, exp, ap) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let po = Uuid::new_v4();
    let inv = w.create_purchase_invoice(NewPurchaseInvoice {
        invoice_number: uq("PI"), company_id: company, branch_id: None, supplier_id: Uuid::new_v4(),
        source_po_id: Some(po), posting_date: day(), due_date: None, currency: None, payable_account_id: ap,
        lines: vec![line(item, exp, "10", "90000")], tax_lines: vec![],
    }).await.unwrap();

    let gl = BarrierGl { gate: Arc::new(tokio::sync::Barrier::new(2)), journal: Uuid::new_v4(), post: Uuid::new_v4() };
    let (w1, w2, g1, g2) = (w.clone(), w.clone(), gl.clone(), gl.clone());
    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { w1.post_purchase_invoice(inv, &g1).await }),
        tokio::spawn(async move { w2.post_purchase_invoice(inv, &g2).await }),
    );
    // Both calls succeed (one posts, one reconciles idempotently) — neither errors.
    r1.unwrap().unwrap();
    r2.unwrap().unwrap();

    let emitted = rec.events.lock().unwrap().iter().filter(|e| matches!(e, BillingEvent::PurchaseInvoicePosted(p) if p.invoice_id == inv)).count();
    assert_eq!(emitted, 1, "the seam event must fire exactly once, even under a concurrent double-post");
}

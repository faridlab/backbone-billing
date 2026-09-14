//! Integrity probes for billing — the invariants that must hold against a REAL Postgres, beyond the
//! golden math. Requires DATABASE_URL (:5433/backbone_billing).
//!
//! IP-1..IP-4    the posting/recovery/balance/seam invariants (service level).
//! IGT           the tenancy posture on the guarded HTTP surface — the module mounts no guard of
//!               its own (the composing service authenticates), so the leg proves a smuggled
//!               tenant field in the body is tolerated and cannot name a tenant.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use backbone_auth::org::OrgContext;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use backbone_billing::presentation::http::create_guarded_billing_routes;
use backbone_billing::BillingModule;

use backbone_billing::application::service::billing_events::{BillingEvent, BillingEventSink};
use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope, GlPostAck, GlPostRejected, GlPostSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingError, BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewSalesInvoice,
    NewTaxLine,
};

fn d(s: &str) -> Decimal {
    Decimal::from_str_exact(s).unwrap()
}
fn day() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap()
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

/// A sink that ALWAYS rejects — proves a rejected post never marks the invoice posted, and the
/// failure is recoverable (posting_state=failed, retryable).
struct RejectingGl;
#[async_trait::async_trait]
impl GlPostSink for RejectingGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        Err(GlPostRejected {
            code: "period_closed".into(),
            message: "accounting period is closed".into(),
        })
    }
}
/// A sink that records + acks — for the retry-after-failure probe.
#[derive(Default, Clone)]
struct OkGl {
    hits: Arc<Mutex<usize>>,
    journal: Uuid,
    post: Uuid,
}
#[async_trait::async_trait]
impl GlPostSink for OkGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        *self.hits.lock().unwrap() += 1;
        Ok(GlPostAck {
            post_id: self.post,
            journal_id: self.journal,
            idempotent_reuse: false,
        })
    }
}

fn line(item: Uuid, acct: Uuid, qty: &str, price: &str) -> NewInvoiceLine {
    NewInvoiceLine {
        item_id: item,
        account_id: acct,
        description: None,
        quantity: d(qty),
        unit_price: d(price),
        tax_template_id: None,
    }
}

async fn draft_sales(
    w: &BillingWriteService,
    currency: Option<String>,
    tax: Vec<NewTaxLine>,
) -> Uuid {
    let (item, rev, ar) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    w.create_sales_invoice(NewSalesInvoice {
        invoice_number: uq("SI"),
        branch_id: None,
        customer_id: Uuid::new_v4(),
        source_so_id: None,
        posting_date: day(),
        due_date: None,
        payment_term_id: None,
        currency,
        receivable_account_id: ar,
        lines: vec![line(item, rev, "1", "100000")],
        tax_lines: tax,
    })
    .await
    .unwrap()
}

// IP-1: a rejected GL post leaves the invoice NOT posted and RECOVERABLE — posting_state=failed,
// status still draft, no journal id. A later successful post then completes it.
#[tokio::test]
async fn rejected_post_is_recoverable() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let id = draft_sales(&w, None, vec![]).await;

    let e = w.post_sales_invoice(id, &RejectingGl).await.unwrap_err();
    assert!(matches!(e, BillingError::GlRejected { .. }));
    let (ps, st, jid): (String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT posting_state::text, status::text, journal_id FROM billing.sales_invoices WHERE id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(ps, "failed");
    assert_eq!(st, "draft", "a failed post must not submit the invoice");
    assert!(jid.is_none());

    // Recovery: a working sink posts it cleanly.
    let ok = OkGl {
        hits: Arc::new(Mutex::new(0)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    w.post_sales_invoice(id, &ok).await.unwrap();
    let (ps2, st2): (String, String) = sqlx::query_as(
        "SELECT posting_state::text, status::text FROM billing.sales_invoices WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
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
    let id = draft_sales(&w, Some("USD".into()), vec![]).await;
    let ok = OkGl {
        hits: Arc::new(Mutex::new(0)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let e = w.post_sales_invoice(id, &ok).await.unwrap_err();
    assert!(matches!(e, BillingError::UnsupportedCurrency(c) if c == "USD"));
    assert_eq!(
        *ok.hits.lock().unwrap(),
        0,
        "the sink is never reached for an unsupported currency"
    );
}

// IP-3: build_ar_post is self-balancing regardless of a supplied tax line's own account — the A/R
// debit always equals net + Σoutput. (A tampered tax_amount that broke balance would be caught by
// is_balanced → UnbalancedPost rather than posting a broken journal.)
#[tokio::test]
async fn ar_post_is_balanced_with_tax() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let ppn = Uuid::new_v4();
    let id = draft_sales(
        &w,
        None,
        vec![NewTaxLine {
            account_id: ppn,
            basis: "output".into(),
            description: None,
            rate: d("11"),
            tax_amount: d("11000"),
            taxable_base: Decimal::ZERO,
            tax_template_id: None,
            repartition_line_id: None,
            real_account_id: None,
            exigibility: None,
        }],
    )
    .await;
    let env = w.build_ar_post(id).await.unwrap();
    assert!(env.is_balanced());
    let (dr, cr) = env.totals();
    assert_eq!((dr, cr), (d("111000.00"), d("111000.00")));
    // grand persisted = net + output.
    let grand: Decimal =
        sqlx::query_scalar("SELECT grand_total FROM billing.sales_invoices WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(grand, d("111000.00"));
}

/// A GL sink that parks on a barrier BEFORE returning the ack, so two concurrent posts are both
/// guaranteed to be past `short_circuit_posted` (which sees `pending` for both) at the same instant —
/// making the pending→posted UPDATE race deterministic.
#[derive(Clone)]
struct BarrierGl {
    gate: Arc<tokio::sync::Barrier>,
    journal: Uuid,
    post: Uuid,
}
#[async_trait::async_trait]
impl GlPostSink for BarrierGl {
    async fn post(&self, _e: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        self.gate.wait().await; // both callers meet here, having already cleared short_circuit
                                // Accounting dedupes on source_id, so a real ledger returns a valid ack to BOTH racers.
        Ok(GlPostAck {
            post_id: self.post,
            journal_id: self.journal,
            idempotent_reuse: false,
        })
    }
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

// IP-4 (council 2026-07-05; threat model corrected 2026-07-26): the seam event is emitted EXACTLY
// once even under a concurrent double-post. Without gating the publish on the pending→posted
// UPDATE's rows_affected, both racers would publish `PurchaseInvoicePosted`, and the duplicate
// would reach buying::mark_billed — where buying's own `allocate` cap refuses it with `OverBilling`
// + a ThreeWayMatchFailed{over_billing} broadcast (billed_qty is NOT silently double-advanced; the
// receiver defends in depth). The gate still matters: it kills that noisy, misleading procurement
// signal at the source. The receiver-side containment is proven separately in
// backbone-buying/tests/funnel_and_events.rs; this probe proves only billing's emit-once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_post_emits_the_seam_event_once() {
    let pool = pool().await;
    let rec = Recorder::default();
    let w = Arc::new(BillingWriteService::with_sink(
        pool.clone(),
        Arc::new(rec.clone()),
    ));
    let (item, exp, ap) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let po = Uuid::new_v4();
    let inv = w
        .create_purchase_invoice(NewPurchaseInvoice {
            invoice_number: uq("PI"),
            branch_id: None,
            supplier_id: Uuid::new_v4(),
            source_po_id: Some(po),
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            payable_account_id: ap,
            lines: vec![line(item, exp, "10", "90000")],
            tax_lines: vec![],
        })
        .await
        .unwrap();

    let gl = BarrierGl {
        gate: Arc::new(tokio::sync::Barrier::new(2)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let (w1, w2, g1, g2) = (w.clone(), w.clone(), gl.clone(), gl.clone());
    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { w1.post_purchase_invoice(inv, &g1).await }),
        tokio::spawn(async move { w2.post_purchase_invoice(inv, &g2).await }),
    );
    // Both calls succeed (one posts, one reconciles idempotently) — neither errors.
    r1.unwrap().unwrap();
    r2.unwrap().unwrap();

    let emitted = rec
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, BillingEvent::PurchaseInvoicePosted(p) if p.invoice_id == inv))
        .count();
    assert_eq!(
        emitted, 1,
        "the seam event must fire exactly once, even under a concurrent double-post"
    );
}

// ── guarded HTTP surface: tenancy ────────────────────────────────────────────

async fn module(pool: &PgPool) -> BillingModule {
    BillingModule::builder()
        .with_database(pool.clone())
        .build()
        .unwrap()
}
fn app(pool: &PgPool, m: &BillingModule) -> axum::Router {
    let write = std::sync::Arc::new(
        backbone_billing::application::service::billing_write_service::BillingWriteService::new(
            pool.clone(),
        ),
    );
    // The module ships no guard, so the test stands in for the composing service's outer org
    // guard: it inserts the OrgContext the write handlers extract.
    create_guarded_billing_routes(m, write).layer(axum::middleware::from_fn(
        |mut req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| async move {
            req.extensions_mut().insert(OrgContext {
                acting_unit_id: Uuid::nil(),
                entitled_units: vec![],
                legacy_company_id: None,
                user_id: "probe".to_string(),
            });
            next.run(req).await
        },
    ))
}

/// Send a request.
async fn req_with(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: Option<String>,
) -> (StatusCode, String) {
    let b = body.map(Body::from).unwrap_or(Body::empty());
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let resp = app.oneshot(builder.body(b).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// A well-formed sales-invoice body. `extra` injects raw JSON fields (e.g. a smuggled `companyId`).
fn sales_body(number: &str, extra: &str) -> String {
    format!(
        r#"{{"invoiceNumber":"{}",{}"customerId":"{}","postingDate":"2026-07-05",
             "receivableAccountId":"{}",
             "lines":[{{"itemId":"{}","accountId":"{}","quantity":"1","unitPrice":"100000"}}]}}"#,
        number,
        extra,
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
}

// IGT: the surface mounts no guard of its own — authentication is the composing service's duty, so
// the unauthenticated/no-tenant refusals belong to its authn probes, not here. What stays at module
// level: a `companyId`/`branchId` smuggled into the body is TOLERATED (unknown fields are ignored)
// and cannot break the invoice shape — the module has no tenant column to stamp, so nothing the
// body says can name a tenant.
#[tokio::test]
async fn smuggled_body_tenant_fields_are_ignored() {
    let pool = pool().await;
    let m = module(&pool).await;
    let attacker_company = Uuid::new_v4();
    let number = uq("SI");
    let body = sales_body(
        &number,
        &format!(
            r#""companyId":"{attacker_company}","branchId":"{}","#,
            Uuid::new_v4()
        ),
    );
    let (status, resp) = req_with(app(&pool, &m), "POST", "/sales-invoices", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "got: {resp}");

    let persisted: Uuid = sqlx::query_scalar(
        "SELECT id FROM billing.sales_invoices WHERE invoice_number = $1",
    )
    .bind(&number)
    .fetch_one(&pool)
    .await
    .expect("invoice row created despite the smuggled body field");
}

// IP-5 (outbox fence — mirrors backbone-payment::post_payment): when `with_outbox_schema` is set,
// `post_sales_invoice` stages `SalesInvoicePosted` into `<schema>.outbox_events` INSIDE the
// pending→posted transaction, so a crash after the transition can never lose the event (go-live
// durable bus). The legacy in-proc sink still fires too. Proves the staged payload is relay-readable.
#[tokio::test]
async fn posted_invoice_stages_seam_event_in_outbox() {
    let pool = pool().await;
    // Serialize the outbox bootstrap across this binary's tests: the type/table creation is
    // guarded but not race-safe, and #[tokio::test] runs the callers concurrently.
    sqlx::query("SELECT pg_advisory_lock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("lock the outbox bootstrap");
    backbone_outbox::outbox::migrate(&pool, "billing")
        .await
        .expect("migrate billing outbox");
    sqlx::query("SELECT pg_advisory_unlock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("unlock the outbox bootstrap");

    let rec = Recorder::default();
    let gl = OkGl {
        hits: Arc::new(Mutex::new(0)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let w = BillingWriteService::with_sink(pool.clone(), Arc::new(rec.clone()))
        .with_outbox_schema("billing");

    let (customer, item, ar) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let inv = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            branch_id: None,
            customer_id: customer,
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![line(item, ar, "1", "100000")],
            tax_lines: vec![],
        })
        .await
        .unwrap();

    w.post_sales_invoice(inv, &gl).await.unwrap();

    // Legacy in-proc sink fired exactly once.
    let in_proc = rec
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, BillingEvent::SalesInvoicePosted(p) if p.invoice_id == inv))
        .count();
    assert_eq!(
        in_proc, 1,
        "in-proc sink must still fire alongside the durable outbox"
    );

    // The durable outbox holds exactly one SalesInvoicePosted for this invoice — staged iff the
    // transition committed (the fence).
    let staged: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM billing.outbox_events WHERE event_type='SalesInvoicePosted' AND aggregate_id=$1")
        .bind(inv.to_string()).fetch_one(&pool).await.unwrap();
    assert_eq!(
        staged, 1,
        "SalesInvoicePosted must be staged in the outbox exactly once"
    );

    // The staged payload is the serialized seam event (relay-readable: carries the billed line).
    let payload_item: String = sqlx::query_scalar(
        "SELECT (payload->'billed_lines'->0->>'item_id') FROM billing.outbox_events WHERE aggregate_id=$1")
        .bind(inv.to_string()).fetch_one(&pool).await.unwrap();
    assert_eq!(
        payload_item,
        item.to_string(),
        "staged payload carries the billed line"
    );
}

// IP-6 (outbox fence, A/P side): `post_purchase_invoice` stages `PurchaseInvoicePosted` in the
// transition tx — the A/P seam event a relay routes to buying::mark_billed. Mirrors IP-5 for A/P.
#[tokio::test]
async fn posted_purchase_invoice_stages_seam_event_in_outbox() {
    let pool = pool().await;
    // Serialize the outbox bootstrap across this binary's tests: the type/table creation is
    // guarded but not race-safe, and #[tokio::test] runs the callers concurrently.
    sqlx::query("SELECT pg_advisory_lock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("lock the outbox bootstrap");
    backbone_outbox::outbox::migrate(&pool, "billing")
        .await
        .expect("migrate billing outbox");
    sqlx::query("SELECT pg_advisory_unlock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("unlock the outbox bootstrap");

    let rec = Recorder::default();
    let gl = OkGl {
        hits: Arc::new(Mutex::new(0)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let w = BillingWriteService::with_sink(pool.clone(), Arc::new(rec.clone()))
        .with_outbox_schema("billing");

    let (supplier, item, ap) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let inv = w
        .create_purchase_invoice(NewPurchaseInvoice {
            invoice_number: uq("PI"),
            branch_id: None,
            supplier_id: supplier,
            source_po_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            payable_account_id: ap,
            lines: vec![line(item, ap, "1", "100000")],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    w.post_purchase_invoice(inv, &gl).await.unwrap();

    let in_proc = rec
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, BillingEvent::PurchaseInvoicePosted(p) if p.invoice_id == inv))
        .count();
    assert_eq!(in_proc, 1, "in-proc sink must fire for the A/P post");

    let staged: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM billing.outbox_events WHERE event_type='PurchaseInvoicePosted' AND aggregate_id=$1")
        .bind(inv.to_string()).fetch_one(&pool).await.unwrap();
    assert_eq!(
        staged, 1,
        "PurchaseInvoicePosted must be staged exactly once"
    );
}

// IP-7 (outbox fence, credit note): `reverse_sales_invoice` stages `InvoiceCancelled` in the
// cancelled-transition tx, so a crash after the reversal can never lose the cancel event.
#[tokio::test]
async fn reversed_invoice_stages_cancel_in_outbox() {
    let pool = pool().await;
    // Serialize the outbox bootstrap across this binary's tests: the type/table creation is
    // guarded but not race-safe, and #[tokio::test] runs the callers concurrently.
    sqlx::query("SELECT pg_advisory_lock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("lock the outbox bootstrap");
    backbone_outbox::outbox::migrate(&pool, "billing")
        .await
        .expect("migrate billing outbox");
    sqlx::query("SELECT pg_advisory_unlock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("unlock the outbox bootstrap");

    let rec = Recorder::default();
    let gl = OkGl {
        hits: Arc::new(Mutex::new(0)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let w = BillingWriteService::with_sink(pool.clone(), Arc::new(rec.clone()))
        .with_outbox_schema("billing");

    let (customer, item, ar) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let inv = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            branch_id: None,
            customer_id: customer,
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![line(item, ar, "1", "100000")],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    w.post_sales_invoice(inv, &gl).await.unwrap(); // posted → outstanding set
    w.reverse_sales_invoice(inv, &gl).await.unwrap(); // credit note → cancelled

    let in_proc = rec
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| matches!(e, BillingEvent::InvoiceCancelled(c) if c.invoice_id == inv))
        .count();
    assert_eq!(in_proc, 1, "in-proc sink must fire InvoiceCancelled");

    let staged: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM billing.outbox_events WHERE event_type='InvoiceCancelled' AND aggregate_id=$1")
        .bind(inv.to_string()).fetch_one(&pool).await.unwrap();
    assert_eq!(staged, 1, "InvoiceCancelled must be staged exactly once");
}

// IP-8 (the tax consumer's payload contract): the staged posted-invoice and cancel payloads carry
// every field the e-Faktur consumer reads off the wire — `invoice_id`, `company_id`,
// `posting_date`, `taxable_base`, and the per-kind totals; `kind` on the cancel.
//
// Why this lives here rather than on the consuming side. The consumer is a composition seam: it
// deserializes these payloads into its own wire DTO, and its tests hand-build that JSON. So nothing
// over there proves billing actually STAGES the shape it assumes — a renamed or dropped field would
// leave both sides green and break only in production, silently, as an event that drains and does
// nothing. Billing owns the producing half of the contract, so billing pins it. The module keeps
// zero cargo edges to siblings: this asserts the serialized JSON, never a consumer type.
#[tokio::test]
async fn staged_payloads_carry_the_fields_the_tax_consumer_reads() {
    let pool = pool().await;
    // Serialize the outbox bootstrap across this binary's tests: the type/table creation is
    // guarded but not race-safe, and #[tokio::test] runs the callers concurrently.
    sqlx::query("SELECT pg_advisory_lock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("lock the outbox bootstrap");
    backbone_outbox::outbox::migrate(&pool, "billing")
        .await
        .expect("migrate billing outbox");
    sqlx::query("SELECT pg_advisory_unlock(hashtext('billing_outbox_migrate'))")
        .execute(&pool)
        .await
        .expect("unlock the outbox bootstrap");

    let gl = OkGl {
        hits: Arc::new(Mutex::new(0)),
        journal: Uuid::new_v4(),
        post: Uuid::new_v4(),
    };
    let w = BillingWriteService::new(pool.clone()).with_outbox_schema("billing");

    /// The staged payload for one event type on one invoice.
    async fn staged(pool: &PgPool, event_type: &str, invoice: Uuid) -> serde_json::Value {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT payload FROM billing.outbox_events \
              WHERE event_type = $1 AND aggregate_id = $2",
        )
        .bind(event_type)
        .bind(invoice.to_string())
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("no staged {event_type} for {invoice}: {e}"))
    }

    /// Every named key must be present and non-null. A field the consumer reads that arrives as
    /// JSON null deserializes into a missing required field, which is the same outage as a rename.
    fn carries(payload: &serde_json::Value, event: &str, keys: &[&str]) {
        for key in keys {
            let value = payload.get(key).unwrap_or_else(|| {
                panic!("{event} payload has no `{key}` — the tax consumer reads it: {payload}")
            });
            assert!(
                !value.is_null(),
                "{event}.{key} staged as null — the consumer's required field cannot decode"
            );
        }
    }

    // ── A/R: a posted sales invoice ──
    let (customer, item, ar, ppn) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let sales = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            branch_id: None,
            customer_id: customer,
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![line(item, ar, "1", "1000000")],
            tax_lines: vec![NewTaxLine {
                account_id: ppn,
                basis: "output".into(),
                description: None,
                rate: d("0.11"),
                tax_amount: d("110000"),
                taxable_base: Default::default(),
                tax_template_id: None,
                repartition_line_id: None,
                real_account_id: None,
                exigibility: None,
            }],
        })
        .await
        .unwrap();
    w.post_sales_invoice(sales, &gl).await.unwrap();

    let posted = staged(&pool, "SalesInvoicePosted", sales).await;
    carries(
        &posted,
        "SalesInvoicePosted",
        &[
            "invoice_id",
            "company_id",
            "posting_date",
            "taxable_base",
            "output_total",
        ],
    );
    assert_eq!(
        posted["invoice_id"].as_str(),
        Some(sales.to_string().as_str()),
        "the payload names the invoice it was staged for"
    );

    // ── A/P: a posted purchase invoice carries the input-VAT side instead ──
    let (supplier, ap) = (Uuid::new_v4(), Uuid::new_v4());
    let purchase = w
        .create_purchase_invoice(NewPurchaseInvoice {
            invoice_number: uq("PI"),
            branch_id: None,
            supplier_id: supplier,
            source_po_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            payable_account_id: ap,
            lines: vec![line(item, ap, "1", "1000000")],
            tax_lines: vec![NewTaxLine {
                account_id: ppn,
                basis: "input".into(),
                description: None,
                rate: d("0.11"),
                tax_amount: d("110000"),
                taxable_base: Default::default(),
                tax_template_id: None,
                repartition_line_id: None,
                real_account_id: None,
                exigibility: None,
            }],
        })
        .await
        .unwrap();
    w.post_purchase_invoice(purchase, &gl).await.unwrap();

    carries(
        &staged(&pool, "PurchaseInvoicePosted", purchase).await,
        "PurchaseInvoicePosted",
        &[
            "invoice_id",
            "company_id",
            "posting_date",
            "taxable_base",
            "input_total",
            "withholding_total",
        ],
    );

    // ── The cancel leg: the consumer routes on `kind`, so a missing one voids nothing ──
    w.reverse_sales_invoice(sales, &gl).await.unwrap();
    let cancelled = staged(&pool, "InvoiceCancelled", sales).await;
    carries(
        &cancelled,
        "InvoiceCancelled",
        &["invoice_id", "company_id", "kind"],
    );
    assert_eq!(
        cancelled["kind"].as_str(),
        Some("sales"),
        "a cancelled sales invoice is staged as the sales kind — the consumer voids by kind"
    );
}

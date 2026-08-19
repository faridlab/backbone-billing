//! The settlement seam against the REAL reconciliation graph — end-to-end across TWO modules:
//! **billing → accounting** through the shared `ReconcileSink` port.
//!
//! Flow: billing posts a Sales Invoice into the real ledger (`Dr A/R · Cr Revenue · Cr PPN`);
//! a payment posts its `Dr Bank · Cr A/R` journal with source_type `payment`; then
//! `apply_settlement` / `reverse_settlement` (and the `PaymentSettled` / `PaymentCancelled`
//! consumers) run against BOTH the billing subledger and accounting's partial-reconcile edges —
//! in ONE transaction, so `outstanding == grand_total − Σ edge amounts` holds by construction.
//!
//! The in-test `AccountingReconcileSink` adapter is the reference implementation of the port:
//! the composing host implements exactly this shape over accounting's `ReconcileWriteService`.
//! Requires DATABASE_URL (:5433 with `billing` + `accounting` schemas migrated — one database,
//! e.g. via `metaphor migration run-all --database-url …`).

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use backbone_billing::application::service::billing_gl::{
    ReconcileEdgeAck, ReconcileLine, ReconcileOrigin, ReconcilePairRequest, ReconcileRejected,
    ReconcileSink, UnreconcilePairRequest,
};
use backbone_billing::application::service::billing_write_service::{
    BillingError, BillingWriteService, NewInvoiceLine, NewSalesInvoice,
};
use backbone_billing::application::service::settlement_consumer::{
    PaymentCancelledDto, PaymentCancelledHandler, PaymentSettledDto, PaymentSettledHandler,
    SettledInvoiceDto,
};

use backbone_accounting::application::service::posting_service::{
    PostingLine, PostingRequest, PostingService,
};
use backbone_accounting::application::service::reconcile_write_service::ReconcileWriteService;
use backbone_accounting::domain::reconcile_graph::{LineLocator, PairRequest};
use backbone_accounting::infrastructure::persistence::{
    SqlxPostingRepository, SqlxReconcileGraphRepository,
};

/// ACL: the reconciliation port over accounting's write service. The host's implementation shape.
struct AccountingReconcileSink {
    svc: ReconcileWriteService,
}

#[async_trait::async_trait]
impl ReconcileSink for AccountingReconcileSink {
    async fn reconcile_pair_on(
        &self,
        conn: &mut sqlx::PgConnection,
        req: &ReconcilePairRequest,
    ) -> Result<ReconcileEdgeAck, ReconcileRejected> {
        let origin = match req.origin {
            ReconcileOrigin::Settlement => "settlement",
            ReconcileOrigin::Clearing => "clearing",
            ReconcileOrigin::Manual => "manual",
        };
        let to_loc = |l: &ReconcileLine| LineLocator {
            source_type: l.source_type.clone(),
            source_id: l.source_id,
            account_id: l.account_id,
            reversing: l.reversing,
        };
        match self
            .svc
            .reconcile_pair_on(
                conn,
                &PairRequest {
                    company_id: req.company_id,
                    debit: to_loc(&req.debit),
                    credit: to_loc(&req.credit),
                    amount: req.amount,
                    origin: origin.to_string(),
                    actor: None,
                },
            )
            .await
        {
            Ok(o) => Ok(ReconcileEdgeAck {
                partial_id: o.partial_id,
                applied: o.applied,
                full_reconcile_id: o.full_reconcile_id,
            }),
            Err(e) => Err(ReconcileRejected { code: e.code().to_string(), message: e.to_string() }),
        }
    }

    async fn unreconcile_pair_on(
        &self,
        conn: &mut sqlx::PgConnection,
        req: &UnreconcilePairRequest,
    ) -> Result<(), ReconcileRejected> {
        let to_loc = |l: &ReconcileLine| LineLocator {
            source_type: l.source_type.clone(),
            source_id: l.source_id,
            account_id: l.account_id,
            reversing: l.reversing,
        };
        self.svc
            .unreconcile_pair_on(conn, req.company_id, &to_loc(&req.debit), &to_loc(&req.credit))
            .await
            .map_err(|e| ReconcileRejected { code: e.code().to_string(), message: e.to_string() })
    }
}

fn d(s: &str) -> Decimal {
    Decimal::from_str_exact(s).unwrap()
}
fn day() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()
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

/// A/R + Bank chart; the A/R control is `is_reconcilable` — settlement edges live on it.
async fn seed_coa(pool: &PgPool) -> (Uuid, HashMap<&'static str, Uuid>) {
    let company = Uuid::new_v4();
    let coa: &[(&str, &str, &str, &str, &str, bool)] = &[
        ("1200", "Piutang Usaha", "asset", "accounts_receivable", "debit", true),
        ("1100", "Bank", "asset", "bank", "debit", false),
    ];
    let mut m = HashMap::new();
    for (code, name, at, st, nb, rec) in coa {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO accounting.accounts (id, company_id, account_number, account_code, name,
                account_type, account_subtype, normal_balance, is_header, is_detail, is_reconcilable, status)
               VALUES ($1,$2,$3,$4,$5,$6::account_type,$7::account_subtype,$8::normal_balance,false,true,$9,'active'::account_status)"#,
        )
        .bind(id).bind(company).bind(code).bind(code).bind(name).bind(at).bind(st).bind(nb).bind(rec)
        .execute(pool).await.expect("seed acct");
        m.insert(*code, id);
    }
    (company, m)
}

fn reconcile_sink(pool: &PgPool) -> AccountingReconcileSink {
    AccountingReconcileSink {
        svc: ReconcileWriteService::new(
            Arc::new(SqlxReconcileGraphRepository::new()),
            Arc::new(SqlxPostingRepository::new(pool.clone())),
            pool.clone(),
            None,
        ),
    }
}

/// Post a rounded-amount sales invoice (no tax) — `Dr A/R grand · Cr Revenue grand` — and return
/// its id. `amount` lands as the grand total (quantity 1 × unit price).
async fn post_invoice(
    pool: &PgPool,
    billing: &BillingWriteService,
    company: Uuid,
    coa: &HashMap<&'static str, Uuid>,
    customer: Uuid,
    amount: Decimal,
) -> Uuid {
    let inv = billing
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: customer,
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            currency: None,
            receivable_account_id: coa["1200"],
            lines: vec![NewInvoiceLine {
                item_id: Uuid::new_v4(),
                account_id: coa["1100"], // revenue substitute: the second leg of the balanced post
                description: None,
                quantity: d("1"),
                unit_price: amount,
            }],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    let gl = GlCashAdapter { svc: PostingService::new(Arc::new(SqlxPostingRepository::new(pool.clone()))) };
    billing.post_sales_invoice(inv, &gl).await.unwrap();
    inv
}

/// The GL adapter for the invoice post (ar_seam's shape, kept local).
struct GlCashAdapter {
    svc: PostingService,
}
#[async_trait::async_trait]
impl backbone_billing::application::service::billing_gl::GlPostSink for GlCashAdapter {
    async fn post(
        &self,
        e: &backbone_billing::application::service::billing_gl::AccountingPostEnvelope,
    ) -> Result<backbone_billing::application::service::billing_gl::GlPostAck, backbone_billing::application::service::billing_gl::GlPostRejected> {
        let mut r = PostingRequest::original(e.company_id, &e.source_type, e.source_id, e.posting_date);
        r.source_reference = e.source_reference.clone();
        r.lines = e
            .lines
            .iter()
            .map(|l| PostingLine {
                account_id: l.account_id,
                debit: l.debit,
                credit: l.credit,
                party_type: l.party_type.clone(),
                party_id: l.party_id,
                cost_center_id: None,
                project_id: None,
                department_id: None,
                description: l.description.clone(),
            })
            .collect();
        match self.svc.post(r, None).await {
            Ok(x) => Ok(backbone_billing::application::service::billing_gl::GlPostAck {
                post_id: x.post_id,
                journal_id: x.journal_id,
                idempotent_reuse: x.idempotent_reuse,
            }),
            Err(x) => Err(backbone_billing::application::service::billing_gl::GlPostRejected {
                code: x.code().to_string(),
                message: x.to_string(),
            }),
        }
    }
}

/// Post the payment's journal — `Dr Bank · Cr A/R (customer party)` — source-stamped `payment`,
/// exactly as backbone-payment posts it.
async fn post_payment(
    pool: &PgPool,
    company: Uuid,
    coa: &HashMap<&'static str, Uuid>,
    customer: Uuid,
    payment_id: Uuid,
    amount: Decimal,
) {
    let svc = PostingService::new(Arc::new(SqlxPostingRepository::new(pool.clone())));
    let mut r = PostingRequest::original(company, "payment", payment_id, day());
    r.lines = vec![
        PostingLine {
            account_id: coa["1100"],
            debit: amount,
            credit: Decimal::ZERO,
            party_type: None,
            party_id: None,
            cost_center_id: None,
            project_id: None,
            department_id: None,
            description: None,
        },
        PostingLine {
            account_id: coa["1200"],
            debit: Decimal::ZERO,
            credit: amount,
            party_type: Some("customer".into()),
            party_id: Some(customer),
            cost_center_id: None,
            project_id: None,
            department_id: None,
            description: None,
        },
    ];
    svc.post(r, None).await.unwrap();
}

async fn invoice_state(pool: &PgPool, inv: Uuid) -> (Decimal, Decimal, String) {
    sqlx::query_as(
        "SELECT outstanding_amount, grand_total, status::text FROM billing.sales_invoices WHERE id=$1",
    )
    .bind(inv)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Σ settlement edges + their count for the company.
async fn settlement_edges(pool: &PgPool, company: Uuid) -> (Decimal, i64) {
    sqlx::query_as(
        "SELECT COALESCE(SUM(amount),0), COUNT(*) FROM accounting.partial_reconciles \
         WHERE company_id=$1 AND origin='settlement'",
    )
    .bind(company)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The control-account line's residual: its signed amount minus every partial touching it.
/// The account pin matters — a source posts SEVERAL lines (payment posts Bank + A/R), all stamped
/// with the same source identity; only the control line carries the edges.
async fn residual(pool: &PgPool, company: Uuid, source_type: &str, source_id: Uuid, account: Uuid) -> Decimal {
    sqlx::query_scalar(
        r#"SELECT (CASE WHEN base_debit_amount > 0 THEN base_debit_amount ELSE base_credit_amount END)
                 - COALESCE((SELECT SUM(p.amount) FROM accounting.partial_reconciles p WHERE p.debit_move_id=jl.id),0)
                 - COALESCE((SELECT SUM(p.amount) FROM accounting.partial_reconciles p WHERE p.credit_move_id=jl.id),0)
             FROM accounting.journal_lines jl
            WHERE jl.company_id=$1 AND jl.source_type=$2 AND jl.source_id=$3 AND jl.account_id=$4 AND jl.is_posted"#,
    )
    .bind(company)
    .bind(source_type)
    .bind(source_id)
    .bind(account)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn line_reconciled(pool: &PgPool, company: Uuid, source_type: &str, source_id: Uuid, account: Uuid) -> (bool, Option<Uuid>) {
    sqlx::query_as(
        "SELECT is_reconciled, full_reconcile_id FROM accounting.journal_lines \
         WHERE company_id=$1 AND source_type=$2 AND source_id=$3 AND account_id=$4 AND is_posted",
    )
    .bind(company)
    .bind(source_type)
    .bind(source_id)
    .bind(account)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn full_groups(pool: &PgPool, company: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM accounting.full_reconciles WHERE company_id=$1")
        .bind(company)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A settled invoice draws its subledger outstanding down AND writes the graph edge, in one
/// transaction — the cache and the ledger agree at every step.
#[tokio::test]
async fn settlement_writes_its_graph_edge_and_the_cache_agrees() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let sink = reconcile_sink(&pool);
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let payment = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, payment, d("100")).await;

    let out = billing
        .apply_settlement(company, inv, "sales", d("60"), payment, &sink)
        .await
        .unwrap();
    assert_eq!(out.applied, d("60"));
    assert_eq!(out.remainder, d("0"));

    // Subledger: outstanding drawn to 40, status advanced.
    let (outstanding, grand, status) = invoice_state(&pool, inv).await;
    assert_eq!((outstanding, grand), (d("40"), d("100")));
    assert_eq!(status, "partially_paid");

    // Graph: exactly one settlement edge of 60; both lines' residuals 40.
    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (d("60"), 1), "one settlement edge of 60");
    assert_eq!(residual(&pool, company, "order", inv, coa["1200"]).await, d("40"));
    assert_eq!(residual(&pool, company, "payment", payment, coa["1200"]).await, d("40"));

    // The invariant the whole seam hangs on: outstanding == grand_total − Σ edges.
    assert_eq!(outstanding, grand - sum, "billing cache must equal the graph");
}

/// An over-settlement clamps to the invoice: the surplus stays as the payment line's own residual
/// (the on-account credit), unreconciled — no edge pretends it was applied.
#[tokio::test]
async fn over_settle_leaves_the_on_account_credit_unreconciled() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let sink = reconcile_sink(&pool);
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let payment = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, payment, d("150")).await;

    let out = billing
        .apply_settlement(company, inv, "sales", d("150"), payment, &sink)
        .await
        .unwrap();
    assert_eq!((out.applied, out.remainder), (d("100"), d("50")), "clamped to the invoice");

    let (outstanding, grand, status) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, Decimal::ZERO);
    assert_eq!(status, "paid");

    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (d("100"), 1), "edges total exactly the invoice");
    assert_eq!(outstanding, grand - sum);

    // Invoice side fully reconciled; payment side still owes 50 — no full group, no flag.
    assert_eq!(residual(&pool, company, "order", inv, coa["1200"]).await, Decimal::ZERO);
    assert_eq!(residual(&pool, company, "payment", payment, coa["1200"]).await, d("50"), "on-account credit");
    let (pay_rec, pay_full) = line_reconciled(&pool, company, "payment", payment, coa["1200"]).await;
    assert!(!pay_rec, "the payment line is NOT fully reconciled");
    assert!(pay_full.is_none());
    assert_eq!(full_groups(&pool, company).await, 0);
}

/// The second payment completes the pair: a full-reconcile group stamps both A/R lines.
#[tokio::test]
async fn second_payment_completes_the_full_reconcile_group() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let sink = reconcile_sink(&pool);
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let p1 = Uuid::new_v4();
    let p2 = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, p1, d("60")).await;
    post_payment(&pool, company, &coa, customer, p2, d("40")).await;

    billing.apply_settlement(company, inv, "sales", d("60"), p1, &sink).await.unwrap();
    billing.apply_settlement(company, inv, "sales", d("40"), p2, &sink).await.unwrap();

    let (outstanding, grand, status) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, Decimal::ZERO);
    assert_eq!(status.as_str(), "paid");
    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (d("100"), 2), "one edge per payment");
    assert_eq!(outstanding, grand - sum);

    // Invoice A/R line fully reconciled into a group; both payment lines consumed.
    let (inv_rec, inv_full) = line_reconciled(&pool, company, "order", inv, coa["1200"]).await;
    assert!(inv_rec, "invoice A/R line is reconciled");
    let group = inv_full.expect("group id stamped");
    let (r1, f1) = line_reconciled(&pool, company, "payment", p1, coa["1200"]).await;
    assert!(r1);
    assert_eq!(f1, Some(group), "same group from every member");
    assert_eq!(residual(&pool, company, "payment", p1, coa["1200"]).await, Decimal::ZERO);
    assert_eq!(residual(&pool, company, "payment", p2, coa["1200"]).await, Decimal::ZERO);
    assert_eq!(full_groups(&pool, company).await, 1);
}

/// Reversing a settlement unlinks its edge FIRST (side-effecting), then restores the outstanding —
/// the subledger and the graph unwind together.
#[tokio::test]
async fn reverse_settlement_unlinks_the_edge_and_restores_outstanding() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let sink = reconcile_sink(&pool);
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let payment = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, payment, d("100")).await;
    billing.apply_settlement(company, inv, "sales", d("60"), payment, &sink).await.unwrap();

    let restored = billing
        .reverse_settlement(company, inv, "sales", d("60"), payment, &sink)
        .await
        .unwrap();
    assert_eq!(restored, d("60"));

    let (outstanding, grand, status) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, d("100"), "fully re-owed");
    assert_eq!(status.as_str(), "submitted");
    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (Decimal::ZERO, 0), "the edge is gone");
    assert_eq!(outstanding, grand - sum);
    assert_eq!(residual(&pool, company, "order", inv, coa["1200"]).await, d("100"));
    assert_eq!(residual(&pool, company, "payment", payment, coa["1200"]).await, d("100"));
}

/// Two payments racing the same invoice clamp THROUGH the graph: combined edges never exceed the
/// invoice (the FOR UPDATE + graph line locks arbitrate).
#[tokio::test]
async fn racing_settlements_clamp_through_the_graph() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = Arc::new(BillingWriteService::new(pool.clone()));
    let sink = Arc::new(reconcile_sink(&pool));
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let p1 = Uuid::new_v4();
    let p2 = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, p1, d("60")).await;
    post_payment(&pool, company, &coa, customer, p2, d("60")).await;

    let (a, b) = tokio::join!(
        async { BillingWriteService::apply_settlement(&*billing, company, inv, "sales", d("60"), p1, &*sink).await },
        async { BillingWriteService::apply_settlement(&*billing, company, inv, "sales", d("60"), p2, &*sink).await },
    );
    let applied: Decimal = [a.unwrap().applied, b.unwrap().applied].iter().sum();
    assert_eq!(applied, d("100"), "60 + clamped 40 — never 120");

    let (outstanding, grand, _) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, Decimal::ZERO);
    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (d("100"), 2));
    assert_eq!(outstanding, grand - sum);
    assert_eq!(residual(&pool, company, "order", inv, coa["1200"]).await, Decimal::ZERO);
    // The losing payment keeps its 60 on-account.
    let r1 = residual(&pool, company, "payment", p1, coa["1200"]).await;
    let r2 = residual(&pool, company, "payment", p2, coa["1200"]).await;
    assert_eq!([r1, r2].iter().filter(|r| **r == Decimal::ZERO).count(), 1, "exactly one payment consumed fully");
}

/// The bus consumer is exactly-once: an at-least-once redelivery of `PaymentSettled` is a no-op.
#[tokio::test]
async fn apply_settlements_once_is_exactly_once() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let sink = reconcile_sink(&pool);
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let payment = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, payment, d("60")).await;
    let event = Uuid::new_v4();
    let allocs = vec![(inv, "sales".to_string(), d("60"))];

    let first = billing
        .apply_settlements_once(event, "probe", company, payment, &allocs, &sink)
        .await
        .unwrap();
    assert_eq!(first.applied, d("60"));
    let redelivered = billing
        .apply_settlements_once(event, "probe", company, payment, &allocs, &sink)
        .await
        .unwrap();
    assert_eq!(redelivered.applied, Decimal::ZERO, "redelivery no-ops");

    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (d("60"), 1), "still exactly one edge");
    let (outstanding, grand, _) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, d("40"));
    assert_eq!(outstanding, grand - sum);
}

/// The relay-facing consumers drive the whole loop: `PaymentSettled` applies + edges;
/// `PaymentCancelled` unlinks + restores — over the DTOs a host deserializes from the bus.
#[tokio::test]
async fn payment_event_consumers_round_trip_the_seam() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = Arc::new(BillingWriteService::new(pool.clone()));
    let sink = Arc::new(reconcile_sink(&pool));
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let payment = Uuid::new_v4();
    post_payment(&pool, company, &coa, customer, payment, d("100")).await;

    let settled = PaymentSettledHandler::new(billing.clone(), sink.clone(), "probe-relay");
    let ev_id = Uuid::new_v4();
    let dto = PaymentSettledDto {
        payment_id: payment,
        company_id: company,
        payment_type: "receive".into(),
        paid_amount: d("100"),
        allocations: vec![SettledInvoiceDto {
            invoice_ref: inv,
            invoice_kind: "sales".into(),
            amount: d("100"),
        }],
    };
    let out = settled.handle(ev_id, &dto).await.unwrap();
    assert_eq!(out.applied, d("100"));
    let (outstanding, _, status) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, Decimal::ZERO);
    assert_eq!(status.as_str(), "paid");
    let (inv_rec, _) = line_reconciled(&pool, company, "order", inv, coa["1200"]).await;
    assert!(inv_rec, "full reconcile on the invoice line");

    // The cancellation event reverses it — edges gone, outstanding restored.
    let cancelled = PaymentCancelledHandler::new(billing.clone(), sink.clone(), "probe-relay");
    let ev2 = Uuid::new_v4();
    let cdto = PaymentCancelledDto {
        payment_id: payment,
        company_id: company,
        payment_type: "receive".into(),
        paid_amount: d("100"),
        allocations: dto.allocations.clone(),
    };
    let restored = cancelled.handle(ev2, &cdto).await.unwrap();
    assert_eq!(restored, d("100"));
    let (outstanding, _, status) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, d("100"));
    assert_eq!(status.as_str(), "submitted");
    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (Decimal::ZERO, 0), "unlinked");
    assert_eq!(residual(&pool, company, "order", inv, coa["1200"]).await, d("100"));
}

/// A settlement whose payment journal does not exist FAILS CLOSED: the refused edge rolls the
/// drawdown back with it — the cache never moves without the ledger.
#[tokio::test]
async fn unposted_payment_refuses_and_rolls_the_drawdown_back() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let sink = reconcile_sink(&pool);
    let inv = post_invoice(&pool, &billing, company, &coa, customer, d("100")).await;
    let payment = Uuid::new_v4(); // NO journal posted for this payment

    let err = billing
        .apply_settlement(company, inv, "sales", d("60"), payment, &sink)
        .await
        .unwrap_err();
    match &err {
        BillingError::ReconcileRefused { code, .. } => assert_eq!(code, "line_not_found"),
        other => panic!("expected ReconcileRefused, got {other:?}"),
    }

    // Nothing moved: outstanding intact, no edge, invoice untouched.
    let (outstanding, grand, status) = invoice_state(&pool, inv).await;
    assert_eq!(outstanding, d("100"));
    assert_eq!(status.as_str(), "submitted");
    let (sum, n) = settlement_edges(&pool, company).await;
    assert_eq!((sum, n), (Decimal::ZERO, 0));
    assert_eq!(outstanding, grand - sum);
}

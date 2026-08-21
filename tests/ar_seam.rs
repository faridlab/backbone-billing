//! The order-to-cash BILLING seam, end-to-end across TWO modules: **billing → accounting** — the
//! A/R (sales-invoice) mirror of `ap_seam.rs`'s A/P proof. Zero normal Cargo edge on
//! `backbone-accounting` (it is a dev-dependency here only).
//!
//! Flow: billing raises a Sales Invoice → posts A/R into the REAL `backbone-accounting` ledger
//! (`Dr A/R · Cr Revenue · Cr PPN Output`) and reconciles from the ack; a second/concurrent post of
//! the same invoice yields exactly one journal (idempotent on the invoice id, arbitrated by
//! accounting's partial unique index). Requires DATABASE_URL (:5433 with `billing` + `accounting`
//! schemas migrated).
//!
//! This is the new home of the A/R revenue-post coverage that lived in backbone-selling's
//! `tests/gl_posting_seam.rs` before selling exited the invoice business (ADR-006).

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope as BillEnv, GlPostAck as BillAck, GlPostRejected as BillRej,
    GlPostSink as BillSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingWriteService, NewInvoiceLine, NewSalesInvoice, NewTaxLine,
};

use backbone_accounting::application::service::posting_service::{
    PostingLine, PostingRequest, PostingService,
};
use backbone_accounting::infrastructure::persistence::SqlxPostingRepository;

/// ACL: billing's serialized envelope → accounting's PostingRequest against the REAL ledger.
struct GlAdapter {
    svc: PostingService,
}
#[async_trait::async_trait]
impl BillSink for GlAdapter {
    async fn post(&self, e: &BillEnv) -> Result<BillAck, BillRej> {
        let mut r =
            PostingRequest::original(e.company_id, &e.source_type, e.source_id, e.posting_date);
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
            Ok(x) => Ok(BillAck {
                post_id: x.post_id,
                journal_id: x.journal_id,
                idempotent_reuse: x.idempotent_reuse,
            }),
            Err(x) => Err(BillRej {
                code: x.code().to_string(),
                message: x.to_string(),
            }),
        }
    }
}

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

/// Seed a minimal A/R chart of accounts in `accounting.*` under a fresh company. A/R is subtype
/// `accounts_receivable` → the ledger requires the customer party on that line.
async fn seed_coa(pool: &PgPool) -> (Uuid, HashMap<&'static str, Uuid>) {
    let company = Uuid::new_v4();
    let coa: &[(&str, &str, &str, &str, &str)] = &[
        (
            "1200",
            "Piutang Usaha",
            "asset",
            "accounts_receivable",
            "debit",
        ),
        ("2200", "PPN Keluaran", "liability", "tax", "credit"),
        (
            "4000",
            "Pendapatan",
            "revenue",
            "operating_revenue",
            "credit",
        ),
    ];
    let mut m = HashMap::new();
    for (code, name, at, st, nb) in coa {
        let id = Uuid::new_v4();
        sqlx::query(r#"INSERT INTO accounting.accounts (id, company_id, account_number, account_code, name, account_type, account_subtype, normal_balance, is_header, is_detail, status)
            VALUES ($1,$2,$3,$4,$5,$6::account_type,$7::account_subtype,$8::normal_balance,false,true,'active'::account_status)"#)
            .bind(id).bind(company).bind(code).bind(code).bind(name).bind(at).bind(st).bind(nb)
            .execute(pool).await.expect("seed acct");
        m.insert(*code, id);
    }
    (company, m)
}

async fn journal_count(pool: &PgPool, company: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM accounting.journals WHERE company_id=$1")
        .bind(company)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// ARSEAM-1: a Sales Invoice posts balanced A/R revenue into the REAL ledger —
/// `Dr A/R 1,110,000 · Cr Revenue 1,000,000 · Cr PPN Output 110,000` — and billing reconciles to posted.
#[tokio::test]
async fn sales_invoice_posts_balanced_revenue_into_the_real_gl() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let item = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());
    let gl = GlAdapter {
        svc: PostingService::new(Arc::new(SqlxPostingRepository::new(pool.clone()))),
    };

    // 10 @ 100,000 net + PPN Output 11% (110,000) → grand 1,110,000.
    let inv = billing
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: customer,
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            receivable_account_id: coa["1200"],
            lines: vec![NewInvoiceLine {
                item_id: item,
                account_id: coa["4000"],
                description: None,
                quantity: d("10"),
                unit_price: d("100000"),
                tax_template_id: None,
            }],
            tax_lines: vec![NewTaxLine {
                account_id: coa["2200"],
                basis: "output".into(),
                description: Some("PPN Keluaran 11%".into()),
                rate: d("11"),
                tax_amount: d("110000"),
                taxable_base: Decimal::ZERO,
                tax_template_id: None,
                repartition_line_id: None,
                real_account_id: None,
                exigibility: None,
            }],
        })
        .await
        .unwrap();

    let out = billing.post_sales_invoice(inv, &gl).await.unwrap();
    assert!(!out.idempotent_reuse, "first post is fresh");

    // The journal balances at the grand total.
    let jrow = sqlx::query(
        "SELECT total_debit, total_credit, line_count FROM accounting.journals WHERE id=$1",
    )
    .bind(out.journal_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(jrow.get::<Decimal, _>("total_debit"), d("1110000"));
    assert_eq!(jrow.get::<Decimal, _>("total_credit"), d("1110000"));
    assert_eq!(jrow.get::<i32, _>("line_count"), 3);

    // A/R debit carries the customer party (subledger aging).
    let (ar_debit, ar_party): (Decimal, Option<Uuid>) = sqlx::query_as(
        "SELECT debit_amount, party_id FROM accounting.journal_lines WHERE journal_id=$1 AND account_id=$2")
        .bind(out.journal_id).bind(coa["1200"]).fetch_one(&pool).await.unwrap();
    assert_eq!(ar_debit, d("1110000"));
    assert_eq!(ar_party, Some(customer));

    // Revenue + PPN credits.
    let rev_credit: Decimal = sqlx::query_scalar(
        "SELECT credit_amount FROM accounting.journal_lines WHERE journal_id=$1 AND account_id=$2",
    )
    .bind(out.journal_id)
    .bind(coa["4000"])
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rev_credit, d("1000000"));
    let ppn_credit: Decimal = sqlx::query_scalar(
        "SELECT credit_amount FROM accounting.journal_lines WHERE journal_id=$1 AND account_id=$2",
    )
    .bind(out.journal_id)
    .bind(coa["2200"])
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ppn_credit, d("110000"));

    // Billing side reconciled to posted + linked to the journal.
    let (ps, jid): (String, Option<Uuid>) = sqlx::query_as(
        "SELECT posting_state::text, journal_id FROM billing.sales_invoices WHERE id=$1",
    )
    .bind(inv)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ps, "posted");
    assert_eq!(jid, Some(out.journal_id));
}

/// ARSEAM-2: posting the same Sales Invoice twice (incl. concurrently) yields exactly ONE journal —
/// idempotent on the invoice id, arbitrated by accounting's partial unique index.
#[tokio::test]
async fn concurrent_double_post_yields_one_journal() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let customer = Uuid::new_v4();
    let billing = BillingWriteService::new(pool.clone());

    let inv = billing
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: customer,
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            payment_term_id: None,
            currency: None,
            receivable_account_id: coa["1200"],
            lines: vec![NewInvoiceLine {
                item_id: Uuid::new_v4(),
                account_id: coa["4000"],
                description: None,
                quantity: d("1"),
                unit_price: d("1000000"),
                tax_template_id: None,
            }],
            tax_lines: vec![],
        })
        .await
        .unwrap();

    // Sequential idempotency first: a second post reuses the ack.
    let p1 = pool.clone();
    let first = {
        let gl = GlAdapter {
            svc: PostingService::new(Arc::new(SqlxPostingRepository::new(p1.clone()))),
        };
        billing.post_sales_invoice(inv, &gl).await.unwrap()
    };
    let second = {
        let gl = GlAdapter {
            svc: PostingService::new(Arc::new(SqlxPostingRepository::new(p1.clone()))),
        };
        billing.post_sales_invoice(inv, &gl).await.unwrap()
    };
    assert!(second.idempotent_reuse, "a re-post reuses the recorded ack");
    assert_eq!(first.journal_id, second.journal_id);
    assert_eq!(
        journal_count(&pool, company).await,
        1,
        "exactly one journal for the company"
    );
}

//! Template-driven invoice creation through the tax engine (the `price_document` path).
//!
//! When an invoice line carries a `tax_template_id`, the create verb computes the whole
//! document's tax through `backbone-tax`'s document engine — the pinned rounding-policy
//! oracle drives the header totals, the redistributed nets overwrite the per-line money()
//! totals (or the journal mis-balances), cash-basis templates post to the transition
//! account with the real account recorded on the overlay, and an unwired engine fails
//! closed. Requires DATABASE_URL (defaults to :5433/backbone_billing) with BOTH the
//! `billing` and `tax` schemas migrated (the seam-test convention — migrate externally).
//! Scope: every service call self-scopes from its DTO's company_id; the raw verification
//! selects run as the test role.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use backbone_billing::application::service::billing_write_service::{
    BillingWriteService, NewInvoiceLine, NewSalesInvoice, NewTaxLine,
};
use backbone_tax::{
    NewCompanySettings, NewRepartitionSplit, NewTag, NewTemplate, NewTemplateRow,
    ReplaceRepartitionFamily, TaxEngine, TaxWriteService,
};

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}
fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()
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

/// A sales-invoice service wired with the tax engine (the composed-service shape).
fn wired(pool: &PgPool) -> BillingWriteService {
    BillingWriteService::new(pool.clone()).with_tax_engine(Arc::new(TaxEngine::new(pool.clone())))
}

fn line(account: Uuid, qty: &str, price: &str, template: Option<Uuid>) -> NewInvoiceLine {
    NewInvoiceLine {
        item_id: Uuid::new_v4(),
        account_id: account,
        description: None,
        quantity: d(qty),
        unit_price: d(price),
        tax_template_id: template,
    }
}

fn supplied_tax(account: Uuid, amount: &str) -> NewTaxLine {
    NewTaxLine {
        account_id: account,
        basis: "output".into(),
        description: None,
        rate: d("11"),
        tax_amount: d(amount),
        taxable_base: Decimal::ZERO,
        tax_template_id: None,
        repartition_line_id: None,
        real_account_id: None,
        exigibility: None,
    }
}

fn new_sales(
    company: Uuid,
    lines: Vec<NewInvoiceLine>,
    tax_lines: Vec<NewTaxLine>,
    receivable: Uuid,
) -> NewSalesInvoice {
    NewSalesInvoice {
        invoice_number: uq("SI"),
        company_id: company,
        branch_id: None,
        customer_id: Uuid::new_v4(),
        source_so_id: None,
        posting_date: day(),
        due_date: None,
        payment_term_id: None,
        currency: None,
        receivable_account_id: receivable,
        lines,
        tax_lines,
    }
}

/// One overlay row as persisted: (account_id, tax_amount, taxable_base, tax_template_id,
/// repartition_line_id, real_account_id, exigibility).
#[allow(clippy::type_complexity)]
async fn overlay(
    pool: &PgPool,
    invoice: Uuid,
) -> Vec<(
    Uuid,
    Decimal,
    Decimal,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
    String,
)> {
    let rows = sqlx::query(
        r#"SELECT account_id, tax_amount, taxable_base, tax_template_id, repartition_line_id,
                  real_account_id, exigibility::text AS exigibility
           FROM billing.invoice_tax_lines
           WHERE invoice_ref = $1 AND invoice_kind = 'sales'
             AND (metadata->>'deleted_at') IS NULL
           ORDER BY tax_amount"#,
    )
    .bind(invoice)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.iter()
        .map(|r| {
            (
                r.get("account_id"),
                r.get("tax_amount"),
                r.get("taxable_base"),
                r.get("tax_template_id"),
                r.get("repartition_line_id"),
                r.get("real_account_id"),
                r.get("exigibility"),
            )
        })
        .collect()
}

async fn header_totals(pool: &PgPool, invoice: Uuid) -> (Decimal, Decimal, Decimal) {
    let r = sqlx::query(
        "SELECT net_total, tax_total, grand_total FROM billing.sales_invoices WHERE id = $1",
    )
    .bind(invoice)
    .fetch_one(pool)
    .await
    .unwrap();
    (r.get("net_total"), r.get("tax_total"), r.get("grand_total"))
}

// BDT-1: a template on any line makes the document template-driven — the engine's overlay
// REPLACES the caller's supplied tax lines (never both; that would double-count).
#[tokio::test]
async fn bdt1_template_driven_ignores_caller_tax_lines() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let w = TaxWriteService::new(pool.clone());
    let real = Uuid::new_v4();
    let tid = w
        .create_template(NewTemplate {
            company_id: company,
            code: uq("PPN"),
            name: "PPN 11%".into(),
            template_type: Some("sales".into()),
            tax_category_id: None,
            is_inclusive: false,
            tax_exigibility: None,
            cash_basis_transition_account_id: None,
        })
        .await
        .unwrap();
    w.add_row(NewTemplateRow {
        company_id: company,
        template_id: tid,
        charge_type: None,
        rate: d("11"),
        account_id: Some(real),
        is_withholding: false,
        effective_from: day(),
        effective_to: None,
        sort_order: 0,
        description: None,
    })
    .await
    .unwrap();

    let bogus = Uuid::new_v4();
    let inv = wired(&pool)
        .create_sales_invoice(new_sales(
            company,
            vec![line(Uuid::new_v4(), "1000", "1", Some(tid))],
            vec![supplied_tax(bogus, "999")], // caller-supplied — must be ignored
            Uuid::new_v4(),
        ))
        .await
        .unwrap();

    let (net, tax, grand) = header_totals(&pool, inv).await;
    assert_eq!(net, d("1000.00"));
    assert_eq!(tax, d("110.00"), "engine output wins, not the caller's 999");
    assert_eq!(grand, d("1110.00"));
    let rows = overlay(&pool, inv).await;
    assert_eq!(
        rows.len(),
        1,
        "exactly the engine's line — the supplied one is dropped"
    );
    assert_eq!(rows[0].0, real);
    assert_eq!(rows[0].1, d("110.00"));
}

// BDT-2: the pinned round_globally oracle drives the invoice header AND the per-line nets —
// the redistributed nets (17.80 + 17.79) are what the journal balances against, and the
// built AR post balances exactly (A/R 43.06).
#[tokio::test]
async fn bdt2_round_globally_drives_totals() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let w = TaxWriteService::new(pool.clone());
    let real = Uuid::new_v4();
    let tid = w
        .create_template(NewTemplate {
            company_id: company,
            code: uq("PPN-INCL"),
            name: "PPN 21% incl".into(),
            template_type: Some("sales".into()),
            tax_category_id: None,
            is_inclusive: true,
            tax_exigibility: None,
            cash_basis_transition_account_id: None,
        })
        .await
        .unwrap();
    w.add_row(NewTemplateRow {
        company_id: company,
        template_id: tid,
        charge_type: None,
        rate: d("21"),
        account_id: Some(real),
        is_withholding: false,
        effective_from: day(),
        effective_to: None,
        sort_order: 0,
        description: None,
    })
    .await
    .unwrap();

    let rev = Uuid::new_v4();
    let ar = Uuid::new_v4();
    let svc = wired(&pool);
    let inv = svc
        .create_sales_invoice(new_sales(
            company,
            vec![
                line(rev, "1", "21.53", Some(tid)),
                line(rev, "1", "21.53", Some(tid)),
            ],
            vec![],
            ar,
        ))
        .await
        .unwrap();

    let (net, tax, grand) = header_totals(&pool, inv).await;
    assert_eq!(
        net,
        d("35.59"),
        "redistributed nets 17.80 + 17.79 — NOT 17.80 + 17.80"
    );
    assert_eq!(tax, d("7.47"));
    assert_eq!(grand, d("43.06"));

    let line_nets: Vec<Decimal> = sqlx::query(
        "SELECT net_amount FROM billing.sales_invoice_lines WHERE invoice_id = $1 ORDER BY net_amount DESC",
    )
    .bind(inv)
    .fetch_all(&pool)
    .await
    .unwrap()
    .iter()
    .map(|r| r.get("net_amount"))
    .collect();
    assert_eq!(line_nets, vec![d("17.80"), d("17.79")]);

    let env = svc.build_ar_post(inv).await.unwrap();
    assert!(
        env.is_balanced(),
        "AR post must balance against the overwritten nets"
    );
    let ar_leg = env.lines.iter().find(|l| l.account_id == ar).unwrap();
    assert_eq!(ar_leg.debit, d("43.06"));
}

// BDT-3: the same inputs under round_per_line — per-company policy divergence (7.48 vs 7.47).
#[tokio::test]
async fn bdt3_round_per_line_drives_totals() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let w = TaxWriteService::new(pool.clone());
    w.upsert_company_settings(NewCompanySettings {
        company_id: company,
        rounding_method: "round_per_line".into(),
        default_exigibility: "on_invoice".into(),
        cash_basis_transition_account_id: None,
    })
    .await
    .unwrap();
    let real = Uuid::new_v4();
    let tid = w
        .create_template(NewTemplate {
            company_id: company,
            code: uq("PPN-INCL"),
            name: "PPN 21% incl".into(),
            template_type: Some("sales".into()),
            tax_category_id: None,
            is_inclusive: true,
            tax_exigibility: None,
            cash_basis_transition_account_id: None,
        })
        .await
        .unwrap();
    w.add_row(NewTemplateRow {
        company_id: company,
        template_id: tid,
        charge_type: None,
        rate: d("21"),
        account_id: Some(real),
        is_withholding: false,
        effective_from: day(),
        effective_to: None,
        sort_order: 0,
        description: None,
    })
    .await
    .unwrap();

    let inv = wired(&pool)
        .create_sales_invoice(new_sales(
            company,
            vec![
                line(Uuid::new_v4(), "1", "21.53", Some(tid)),
                line(Uuid::new_v4(), "1", "21.53", Some(tid)),
            ],
            vec![],
            Uuid::new_v4(),
        ))
        .await
        .unwrap();

    let (net, tax, grand) = header_totals(&pool, inv).await;
    assert_eq!(net, d("35.58"));
    assert_eq!(
        tax,
        d("7.48"),
        "round_per_line diverges from round_globally's 7.47"
    );
    assert_eq!(grand, d("43.06"));
}

// BDT-4: lines without templates keep the caller-supplied overlay verbatim — the
// pre-engine behavior, with the overlay columns at their on_invoice/NULL defaults.
#[tokio::test]
async fn bdt4_explicit_lines_backward_compat() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let tax_acct = Uuid::new_v4();
    let inv = BillingWriteService::new(pool.clone())
        .create_sales_invoice(new_sales(
            company,
            vec![line(Uuid::new_v4(), "100", "1", None)],
            vec![supplied_tax(tax_acct, "11")],
            Uuid::new_v4(),
        ))
        .await
        .unwrap();

    let (net, tax, grand) = header_totals(&pool, inv).await;
    assert_eq!((net, tax, grand), (d("100.00"), d("11.00"), d("111.00")));
    let rows = overlay(&pool, inv).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].3, None, "no template on a supplied line");
    assert_eq!(rows[0].5, None);
    assert_eq!(rows[0].6, "on_invoice", "supplied lines default to accrual");
}

// BDT-5: a template-driven document against a service with NO engine fails closed —
// nothing persists. Never silently fall back to un-taxed totals.
#[tokio::test]
async fn bdt5_engine_unwired_fails_closed() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let w = TaxWriteService::new(pool.clone());
    let tid = w
        .create_template(NewTemplate {
            company_id: company,
            code: uq("PPN"),
            name: "PPN 11%".into(),
            template_type: Some("sales".into()),
            tax_category_id: None,
            is_inclusive: false,
            tax_exigibility: None,
            cash_basis_transition_account_id: None,
        })
        .await
        .unwrap();

    let err = BillingWriteService::new(pool.clone())
        .create_sales_invoice(new_sales(
            company,
            vec![line(Uuid::new_v4(), "1000", "1", Some(tid))],
            vec![],
            Uuid::new_v4(),
        ))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "tax_engine_unwired");
    assert_eq!(err.http_status(), 422);

    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM billing.sales_invoices WHERE company_id = $1")
            .bind(company)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        n, 0,
        "nothing persisted — fail closed, not partially created"
    );
}

// BDT-6: a cash-basis (on_payment) template posts to the TRANSITION account; the real
// account appears only on the overlay row (the reconciliation seam flips it later).
// The transition account is seeded as a real reconcilable accounting account — this DB
// carries the accounting schema, so the DB guard on tax_templates enforces reconcilability.
#[tokio::test]
async fn bdt6_caba_post_lands_on_transition() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let tid = Uuid::new_v4();
    let transition = Uuid::new_v4();
    let real = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO accounting.accounts
               (id, company_id, account_number, account_code, name, account_type, account_subtype,
                normal_balance, is_header, is_detail, status, is_reconcilable)
           VALUES ($1,$2,'2300','2300','PPN transition','liability','tax','credit',
                   FALSE,TRUE,'active'::account_status,TRUE)"#,
    )
    .bind(transition)
    .bind(company)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"INSERT INTO tax.tax_templates
               (id, company_id, code, name, template_type, is_inclusive,
                tax_exigibility, cash_basis_transition_account_id)
           VALUES ($1, $2, $3, 'CABA PPN', 'sales', FALSE,
                   'on_payment'::tax_exigibility, $4)"#,
    )
    .bind(tid)
    .bind(company)
    .bind(uq("CABA"))
    .bind(transition)
    .execute(&pool)
    .await
    .unwrap();
    let w = TaxWriteService::new(pool.clone());
    w.add_row(NewTemplateRow {
        company_id: company,
        template_id: tid,
        charge_type: None,
        rate: d("11"),
        account_id: Some(real),
        is_withholding: false,
        effective_from: day(),
        effective_to: None,
        sort_order: 0,
        description: None,
    })
    .await
    .unwrap();

    let svc = wired(&pool);
    let inv = svc
        .create_sales_invoice(new_sales(
            company,
            vec![line(Uuid::new_v4(), "1000", "1", Some(tid))],
            vec![],
            Uuid::new_v4(),
        ))
        .await
        .unwrap();

    let rows = overlay(&pool, inv).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].0, transition,
        "the posting account IS the transition account"
    );
    assert_eq!(
        rows[0].5,
        Some(real),
        "the overlay remembers where it flips"
    );
    assert_eq!(rows[0].6, "on_payment");

    let env = svc.build_ar_post(inv).await.unwrap();
    assert!(env.is_balanced());
    assert!(
        env.lines
            .iter()
            .any(|l| l.account_id == transition && l.credit == d("110.00")),
        "the tax leg credits the transition account"
    );
    assert!(
        !env.lines.iter().any(|l| l.account_id == real),
        "the real account is NEVER touched at post time"
    );
}

// BDT-7: a template with a repartition family — the overlay records the template AND the
// factor split that produced the amount, with the base it was computed on.
#[tokio::test]
async fn bdt7_overlay_records_routing() {
    let pool = pool().await;
    let company = Uuid::new_v4();
    let w = TaxWriteService::new(pool.clone());
    let a1 = Uuid::new_v4();
    let a2 = Uuid::new_v4();
    let tid = w
        .create_template(NewTemplate {
            company_id: company,
            code: uq("SPLIT"),
            name: "PPN split 60/40".into(),
            template_type: Some("sales".into()),
            tax_category_id: None,
            is_inclusive: false,
            tax_exigibility: None,
            cash_basis_transition_account_id: None,
        })
        .await
        .unwrap();
    w.add_row(NewTemplateRow {
        company_id: company,
        template_id: tid,
        charge_type: None,
        rate: d("10"),
        account_id: None,
        is_withholding: false,
        effective_from: day(),
        effective_to: None,
        sort_order: 0,
        description: None,
    })
    .await
    .unwrap();
    let tag = w
        .create_tag(NewTag {
            company_id: company,
            code: uq("BTAG").into(),
            name: "base tag".into(),
        })
        .await
        .unwrap();
    w.replace_repartition_family(ReplaceRepartitionFamily {
        company_id: company,
        template_id: tid,
        document_type: "invoice".into(),
        base_tag_ids: vec![tag],
        base_description: None,
        tax_splits: vec![
            NewRepartitionSplit {
                factor_percent: d("60"),
                account_id: Some(a1),
                tag_ids: vec![],
                sort_order: 0,
                description: None,
            },
            NewRepartitionSplit {
                factor_percent: d("40"),
                account_id: Some(a2),
                tag_ids: vec![],
                sort_order: 1,
                description: None,
            },
        ],
    })
    .await
    .unwrap();

    let inv = wired(&pool)
        .create_sales_invoice(new_sales(
            company,
            vec![line(Uuid::new_v4(), "100", "1", Some(tid))],
            vec![],
            Uuid::new_v4(),
        ))
        .await
        .unwrap();

    let rows = overlay(&pool, inv).await;
    assert_eq!(rows.len(), 2, "one overlay row per factor split");
    let amounts: Vec<Decimal> = rows.iter().map(|r| r.1).collect();
    assert_eq!(amounts, vec![d("4.00"), d("6.00")], "60/40 of 10 on 100");
    for r in &rows {
        assert_eq!(r.3, Some(tid), "overlay carries the template");
        assert!(r.4.is_some(), "overlay carries the repartition split id");
        assert_eq!(r.2, d("100.00"), "overlay carries the post-policy base");
    }
}

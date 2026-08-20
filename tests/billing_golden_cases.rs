//! Golden oracle for the billing write path (AR/AP invoicing intent). Billing-only — the A/P
//! posting seam into the real ledger + buying is proven in `ap_seam.rs`. Posting here uses a
//! FAKE `GlPostSink` (deterministic ack) so the math + state machine are tested in isolation.
//! Requires DATABASE_URL (:5433/backbone_billing).

use std::sync::{Arc, Mutex};

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

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

/// A fake GL sink: records the envelope, returns a fixed ack. Asserts double-entry balance so a
/// broken build_*_post surfaces here, not silently.
#[derive(Default, Clone)]
struct FakeGl {
    seen: Arc<Mutex<Vec<AccountingPostEnvelope>>>,
    journal: Uuid,
    post: Uuid,
}
impl FakeGl {
    fn new() -> Self {
        Self {
            seen: Arc::new(Mutex::new(Vec::new())),
            journal: Uuid::new_v4(),
            post: Uuid::new_v4(),
        }
    }
    fn last(&self) -> AccountingPostEnvelope {
        self.seen.lock().unwrap().last().unwrap().clone()
    }
}
#[async_trait::async_trait]
impl GlPostSink for FakeGl {
    async fn post(&self, env: &AccountingPostEnvelope) -> Result<GlPostAck, GlPostRejected> {
        assert!(
            env.is_balanced(),
            "billing emitted an UNBALANCED post: {env:?}"
        );
        self.seen.lock().unwrap().push(env.clone());
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
fn tax(acct: Uuid, basis: &str, amt: &str) -> NewTaxLine {
    NewTaxLine {
        account_id: acct,
        basis: basis.into(),
        description: None,
        rate: d("11"),
        tax_amount: d(amt),
        taxable_base: Decimal::ZERO,
        tax_template_id: None,
        repartition_line_id: None,
        real_account_id: None,
        exigibility: None,
    }
}

// SIGC-1: Sales invoice math + AR post — 10 × 100,000 net, PPN Output 11% (110,000) supplied →
// net_total 1,000,000, tax_total 110,000, grand_total 1,110,000; post is
// Dr A/R 1,110,000 · Cr Revenue 1,000,000 · Cr PPN Output 110,000 (balanced).
#[tokio::test]
async fn sales_invoice_math_and_ar_post() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let (company, item, rev, ar, ppn) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let id = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![line(item, rev, "10", "100000")],
            tax_lines: vec![tax(ppn, "output", "110000")],
        })
        .await
        .unwrap();

    let r = sqlx::query("SELECT net_total, tax_total, grand_total, status::text AS st FROM billing.sales_invoices WHERE id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(r.get::<Decimal, _>("net_total"), d("1000000.00"));
    assert_eq!(r.get::<Decimal, _>("tax_total"), d("110000.00"));
    assert_eq!(r.get::<Decimal, _>("grand_total"), d("1110000.00"));
    assert_eq!(r.get::<String, _>("st"), "draft");

    let gl = FakeGl::new();
    let out = w.post_sales_invoice(id, &gl).await.unwrap();
    assert_eq!(out.journal_id, gl.journal);
    let env = gl.last();
    let (dr, cr) = env.totals();
    assert_eq!((dr, cr), (d("1110000.00"), d("1110000.00")));
    // A/R debit carries the customer party.
    let ar_line = env.lines.iter().find(|l| l.account_id == ar).unwrap();
    assert_eq!(ar_line.debit, d("1110000.00"));
    assert_eq!(ar_line.party_type.as_deref(), Some("customer"));
    // posted state + outstanding = grand.
    let (ps, outst): (String, Decimal) = sqlx::query_as(
        "SELECT posting_state::text, outstanding_amount FROM billing.sales_invoices WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ps, "posted");
    assert_eq!(outst, d("1110000.00"));
}

// SIGC-2: Purchase invoice math + A/P post — 10 × 90,000 net, PPN Input 11% (99,000),
// PPh 23 2% withholding (18,000) → grand = net + input − wht = 900,000 + 99,000 − 18,000 = 981,000;
// post is Dr Expense 900,000 · Dr PPN Input 99,000 · Cr A/P 981,000 · Cr PPh 18,000 (balanced).
#[tokio::test]
async fn purchase_invoice_math_and_ap_post() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let (company, item, exp, ap, ppn_in, pph) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let id = w
        .create_purchase_invoice(NewPurchaseInvoice {
            invoice_number: uq("PI"),
            company_id: company,
            branch_id: None,
            supplier_id: Uuid::new_v4(),
            source_po_id: None,
            posting_date: day(),
            due_date: None,
            currency: None,
            payable_account_id: ap,
            lines: vec![line(item, exp, "10", "90000")],
            tax_lines: vec![
                tax(ppn_in, "input", "99000"),
                tax(pph, "withholding", "18000"),
            ],
        })
        .await
        .unwrap();

    let r = sqlx::query("SELECT net_total, tax_total, withholding_total, grand_total FROM billing.purchase_invoices WHERE id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(r.get::<Decimal, _>("net_total"), d("900000.00"));
    assert_eq!(r.get::<Decimal, _>("tax_total"), d("99000.00"));
    assert_eq!(r.get::<Decimal, _>("withholding_total"), d("18000.00"));
    assert_eq!(r.get::<Decimal, _>("grand_total"), d("981000.00"));

    let gl = FakeGl::new();
    w.post_purchase_invoice(id, &gl).await.unwrap();
    let env = gl.last();
    assert_eq!(env.totals(), (d("999000.00"), d("999000.00")));
    let ap_line = env.lines.iter().find(|l| l.account_id == ap).unwrap();
    assert_eq!(ap_line.credit, d("981000.00"));
    assert_eq!(ap_line.party_type.as_deref(), Some("supplier"));
}

// SIGC-3: posting is idempotent on the invoice id — a second post reuses the ack, emits nothing new.
#[tokio::test]
async fn posting_is_idempotent() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let (company, item, rev, ar) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let id = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![line(item, rev, "1", "50000")],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    let gl = FakeGl::new();
    let first = w.post_sales_invoice(id, &gl).await.unwrap();
    assert!(!first.idempotent_reuse);
    let second = w.post_sales_invoice(id, &gl).await.unwrap();
    assert!(second.idempotent_reuse, "re-post reuses the recorded ack");
    assert_eq!(first.journal_id, second.journal_id);
    assert_eq!(
        gl.seen.lock().unwrap().len(),
        1,
        "the sink is hit exactly once"
    );
}

// SIGC-4: validation gates — empty document, negative amount, duplicate number.
#[tokio::test]
async fn validation_gates() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let (company, item, rev, ar) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let base = |num: String, lines: Vec<NewInvoiceLine>| NewSalesInvoice {
        invoice_number: num,
        company_id: company,
        branch_id: None,
        customer_id: Uuid::new_v4(),
        source_so_id: None,
        posting_date: day(),
        due_date: None,
        currency: None,
        receivable_account_id: ar,
        lines,
        tax_lines: vec![],
    };
    // empty
    assert!(matches!(
        w.create_sales_invoice(base(uq("SI"), vec![]))
            .await
            .unwrap_err(),
        BillingError::EmptyDocument
    ));
    // negative
    assert!(matches!(
        w.create_sales_invoice(base(uq("SI"), vec![line(item, rev, "-1", "100")]))
            .await
            .unwrap_err(),
        BillingError::NegativeAmount
    ));
    // duplicate number
    let num = uq("DUP");
    w.create_sales_invoice(base(num.clone(), vec![line(item, rev, "1", "100")]))
        .await
        .unwrap();
    assert!(matches!(
        w.create_sales_invoice(base(num, vec![line(item, rev, "1", "100")]))
            .await
            .unwrap_err(),
        BillingError::DuplicateNumber(_)
    ));
}

// SIGC-5: a tax-free invoice still posts clean (net only) — proving the tax overlay is removable.
#[tokio::test]
async fn tax_free_invoice_posts_net_only() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let (company, item, rev, ar) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let id = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![line(item, rev, "2", "75000")],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    let grand: Decimal =
        sqlx::query_scalar("SELECT grand_total FROM billing.sales_invoices WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(grand, d("150000.00"), "no tax → grand == net");
    let gl = FakeGl::new();
    w.post_sales_invoice(id, &gl).await.unwrap();
    let env = gl.last();
    assert_eq!(env.lines.len(), 2, "Dr A/R + Cr Revenue only — no tax line");
    assert_eq!(env.totals(), (d("150000.00"), d("150000.00")));
}

// SIGC-6: revenue grouped by income account — two income accounts produce two separate credit
// lines (each the sum of its own lines); the single A/R debit = grand total.
#[tokio::test]
async fn sales_invoice_revenue_grouped_by_account() {
    let pool = pool().await;
    let w = BillingWriteService::new(pool.clone());
    let (company, item, ar, rev_a, rev_b) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let id = w
        .create_sales_invoice(NewSalesInvoice {
            invoice_number: uq("SI"),
            company_id: company,
            branch_id: None,
            customer_id: Uuid::new_v4(),
            source_so_id: None,
            posting_date: day(),
            due_date: None,
            currency: None,
            receivable_account_id: ar,
            lines: vec![
                line(item, rev_a, "1", "100000"),
                line(item, rev_a, "1", "50000"),
                line(item, rev_b, "1", "200000"),
            ],
            tax_lines: vec![],
        })
        .await
        .unwrap();
    let gl = FakeGl::new();
    w.post_sales_invoice(id, &gl).await.unwrap();
    let env = gl.last();
    assert!(env.is_balanced());
    // No tax → 1 A/R debit + 2 revenue credits (rev_a summed across its two lines, rev_b separate).
    assert_eq!(env.lines.len(), 3, "Dr A/R + Cr rev_a + Cr rev_b");
    let credit_for = |acct: Uuid| {
        env.lines
            .iter()
            .find(|l| l.account_id == acct && l.credit > Decimal::ZERO)
            .map(|l| l.credit)
    };
    assert_eq!(
        credit_for(rev_a),
        Some(d("150000")),
        "rev_a summed across its two lines"
    );
    assert_eq!(credit_for(rev_b), Some(d("200000")));
    let ar_debit = env.lines.iter().find(|l| l.account_id == ar).unwrap().debit;
    assert_eq!(ar_debit, d("350000"));
}

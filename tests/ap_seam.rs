//! The procure-to-pay BILLING seam, end-to-end across THREE modules: **buying → billing →
//! accounting → buying** — retiring buying's *simulated* billing leg with a real Purchase Invoice.
//! Zero normal Cargo edges (buying + accounting are dev-deps only).
//!
//! Flow: buying confirms a PO + records receipt (→ `to_bill`) → billing raises a Purchase Invoice
//! against that PO → posts A/P into the REAL ledger (Dr Expense · Dr PPN Input · Cr A/P · Cr PPh) +
//! emits `PurchaseInvoicePosted{source_po_id, billed_lines}`; an ACL routes it → buying `mark_billed`
//! → `billed_qty` advances → PO `completed`. All three schemas co-locate in one DB.
//! Requires DATABASE_URL (:5433/backbone_billing with all three schemas migrated).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use backbone_billing::application::service::billing_events::{BillingEvent, BillingEventSink};
use backbone_billing::application::service::billing_gl::{
    AccountingPostEnvelope as BillEnv, GlPostAck as BillAck, GlPostRejected as BillRej, GlPostSink as BillSink,
};
use backbone_billing::application::service::billing_write_service::{
    BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewTaxLine,
};

use backbone_buying::application::service::buying_write_service::{
    BuyingWriteService, NewLine, NewPurchaseOrder,
};

use backbone_accounting::application::service::posting_service::{PostingLine, PostingRequest, PostingService};

/// ACL: billing's serialized envelope → accounting's PostingRequest against the REAL ledger.
struct GlAdapter { svc: PostingService }
#[async_trait::async_trait]
impl BillSink for GlAdapter {
    async fn post(&self, e: &BillEnv) -> Result<BillAck, BillRej> {
        let mut r = PostingRequest::original(e.company_id, &e.source_type, e.source_id, e.posting_date);
        r.source_reference = e.source_reference.clone();
        r.lines = e.lines.iter().map(|l| PostingLine {
            account_id: l.account_id, debit: l.debit, credit: l.credit,
            party_type: l.party_type.clone(), party_id: l.party_id,
            cost_center_id: None, project_id: None, department_id: None, description: l.description.clone(),
        }).collect();
        match self.svc.post(r, None).await {
            Ok(x) => Ok(BillAck { post_id: x.post_id, journal_id: x.journal_id, idempotent_reuse: x.idempotent_reuse }),
            Err(x) => Err(BillRej { code: x.code().to_string(), message: x.to_string() }),
        }
    }
}

/// Records billing domain events so the test can route `PurchaseInvoicePosted` → buying.
#[derive(Default, Clone)]
struct RecordingBillSink { events: Arc<Mutex<Vec<BillingEvent>>> }
impl BillingEventSink for RecordingBillSink {
    fn publish(&self, e: BillingEvent) { self.events.lock().unwrap().push(e); }
}

fn d(s: &str) -> Decimal { Decimal::from_str_exact(s).unwrap() }
fn day() -> chrono::NaiveDate { chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap() }
fn uq(p: &str) -> String { format!("{p}-{}", &Uuid::new_v4().simple().to_string()[..8]) }
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/backbone_billing".to_string());
    PgPool::connect(&url).await.expect("connect DB")
}
async fn seed_coa(pool: &PgPool) -> (Uuid, HashMap<&'static str, Uuid>) {
    let company = Uuid::new_v4();
    // A/P is subtype `payable` → the ledger requires a party on that line.
    let coa: &[(&str, &str, &str, &str, &str)] = &[
        ("5100", "Beban Persediaan", "expense", "direct_cost", "debit"),
        ("1520", "PPN Masukan", "asset", "tax", "debit"),
        ("2100", "Hutang Usaha", "liability", "accounts_payable", "credit"),
        ("2160", "Hutang PPh 23", "liability", "current_liability", "credit"),
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
async fn po_status(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM buying.purchase_orders WHERE id=$1").bind(id).fetch_one(pool).await.unwrap()
}
async fn journal_totals(pool: &PgPool, jid: Uuid) -> (Decimal, Decimal) {
    let r = sqlx::query("SELECT total_debit, total_credit FROM accounting.journals WHERE id=$1").bind(jid).fetch_one(pool).await.unwrap();
    (r.get("total_debit"), r.get("total_credit"))
}

/// APSEAM-1: procure-to-pay billing across buying, billing, and the real ledger.
#[tokio::test]
async fn purchase_invoice_bills_po_across_three_modules() {
    let pool = pool().await;
    let (company, coa) = seed_coa(&pool).await;
    let item = Uuid::new_v4();
    let supplier = Uuid::new_v4();

    let buying = BuyingWriteService::new(pool.clone());
    let recorder = RecordingBillSink::default();
    let billing = BillingWriteService::with_sink(pool.clone(), Arc::new(recorder.clone()));
    let gl = GlAdapter { svc: PostingService::new(pool.clone()) };

    // 1) buying: PO for 10 @ 90,000, confirm, record receipt → to_bill.
    let po = buying.create_purchase_order(NewPurchaseOrder {
        po_number: uq("PO"), supplier_quotation_id: None, order_kind: None, company_id: company,
        branch_id: None, supplier_id: supplier, order_date: day(), schedule_date: None, currency: None,
        tax_rate: Decimal::ZERO, notes: None,
        lines: vec![NewLine { item_id: item, warehouse_id: None, description: None, quantity: d("10"), rate: d("90000") }],
    }).await.unwrap();
    buying.confirm_purchase_order(po).await.unwrap();
    buying.mark_received(po, &[(item, d("10"))]).await.unwrap();
    assert_eq!(po_status(&pool, po).await, "to_bill", "received, awaiting the real invoice");

    // 2) billing: Purchase Invoice against that PO — 10 @ 90,000 net, PPN Input 11% (99,000),
    //    PPh 23 2% (18,000). grand = 900,000 + 99,000 − 18,000 = 981,000.
    let inv = billing.create_purchase_invoice(NewPurchaseInvoice {
        invoice_number: uq("PI"), company_id: company, branch_id: None, supplier_id: supplier,
        source_po_id: Some(po), posting_date: day(), due_date: None, currency: None,
        payable_account_id: coa["2100"],
        lines: vec![NewInvoiceLine { item_id: item, account_id: coa["5100"], description: None, quantity: d("10"), unit_price: d("90000") }],
        tax_lines: vec![
            NewTaxLine { account_id: coa["1520"], basis: "input".into(), description: Some("PPN Masukan 11%".into()), rate: d("11"), tax_amount: d("99000") },
            NewTaxLine { account_id: coa["2160"], basis: "withholding".into(), description: Some("PPh 23 2%".into()), rate: d("2"), tax_amount: d("18000") },
        ],
    }).await.unwrap();

    // 3) billing posts A/P into the REAL ledger.
    let out = billing.post_purchase_invoice(inv, &gl).await.unwrap();
    // Dr Expense 900,000 + Dr PPN Input 99,000 = Cr A/P 981,000 + Cr PPh 18,000 = 999,000 each side.
    assert_eq!(journal_totals(&pool, out.journal_id).await, (d("999000"), d("999000")));

    // 4) ACL: PurchaseInvoicePosted (source_po_id = our PO) → buying.mark_billed.
    let evts = recorder.events.lock().unwrap().clone();
    let posted = evts.iter().find_map(|e| match e {
        BillingEvent::PurchaseInvoicePosted(p) if p.source_po_id == Some(po) => Some(p.clone()), _ => None,
    }).expect("PurchaseInvoicePosted for our PO");
    assert_eq!(posted.grand_total, d("981000.00"));
    let billed: Vec<(Uuid, Decimal)> = posted.billed_lines.iter().map(|l| (l.item_id, l.quantity)).collect();
    buying.mark_billed(po, &billed).await.unwrap();

    // 5) the PO is now fully billed against a real invoice → completed.
    assert_eq!(po_status(&pool, po).await, "completed");
    let bq: Decimal = sqlx::query_scalar("SELECT billed_qty FROM buying.purchase_order_items WHERE order_id=$1").bind(po).fetch_one(&pool).await.unwrap();
    assert_eq!(bq, d("10.0000"));

    // billing invoice reconciled from the ack.
    let (ps, jid): (String, Option<Uuid>) = sqlx::query_as("SELECT posting_state::text, journal_id FROM billing.purchase_invoices WHERE id=$1")
        .bind(inv).fetch_one(&pool).await.unwrap();
    assert_eq!(ps, "posted");
    assert_eq!(jid, Some(out.journal_id));
}

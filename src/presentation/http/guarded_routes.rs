//! Guarded route composition — the RECOMMENDED way to mount the billing module.
//!
//! Hand-authored (user-owned). Read documents + **validated create** (sales-invoice /
//! purchase-invoice); generic create/update/delete CRUD is NOT mounted, so a caller cannot
//! write an invoice with inconsistent totals or bypass the AR/AP posting path.
//! `BillingWriteService` is built from the pool (regen-safe). Posting (`post_sales_invoice` /
//! `post_purchase_invoice`) needs a `GlPostSink` composition layer, so it is service/job-driven,
//! not an HTTP route.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::service::billing_write_service::{
    BillingError, BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewSalesInvoice, NewTaxLine,
};
use crate::BillingModule;

use super::{create_purchase_invoice_read_routes, create_sales_invoice_read_routes};

#[derive(Debug, Serialize)]
struct ErrorBody { error: String, message: String }
#[derive(Debug, Serialize)]
struct IdResponse { id: Uuid }
fn err(e: BillingError) -> axum::response::Response {
    let s = StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (s, Json(ErrorBody { error: e.code(), message: e.to_string() })).into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LineBody {
    item_id: Uuid,
    account_id: Uuid,
    #[serde(default)] description: Option<String>,
    quantity: Decimal,
    unit_price: Decimal,
}
impl From<LineBody> for NewInvoiceLine {
    fn from(b: LineBody) -> Self {
        NewInvoiceLine { item_id: b.item_id, account_id: b.account_id, description: b.description, quantity: b.quantity, unit_price: b.unit_price }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaxLineBody {
    account_id: Uuid,
    basis: String,
    #[serde(default)] description: Option<String>,
    #[serde(default)] rate: Decimal,
    tax_amount: Decimal,
}
impl From<TaxLineBody> for NewTaxLine {
    fn from(b: TaxLineBody) -> Self {
        NewTaxLine { account_id: b.account_id, basis: b.basis, description: b.description, rate: b.rate, tax_amount: b.tax_amount }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSalesInvoiceBody {
    invoice_number: String,
    company_id: Uuid,
    #[serde(default)] branch_id: Option<Uuid>,
    customer_id: Uuid,
    #[serde(default)] source_so_id: Option<Uuid>,
    posting_date: chrono::NaiveDate,
    #[serde(default)] due_date: Option<chrono::NaiveDate>,
    #[serde(default)] currency: Option<String>,
    receivable_account_id: Uuid,
    lines: Vec<LineBody>,
    #[serde(default)] tax_lines: Vec<TaxLineBody>,
}
async fn create_sales_invoice(State(svc): State<Arc<BillingWriteService>>, Json(b): Json<CreateSalesInvoiceBody>) -> axum::response::Response {
    let inv = NewSalesInvoice {
        invoice_number: b.invoice_number, company_id: b.company_id, branch_id: b.branch_id,
        customer_id: b.customer_id, source_so_id: b.source_so_id, posting_date: b.posting_date,
        due_date: b.due_date, currency: b.currency, receivable_account_id: b.receivable_account_id,
        lines: b.lines.into_iter().map(Into::into).collect(),
        tax_lines: b.tax_lines.into_iter().map(Into::into).collect(),
    };
    match svc.create_sales_invoice(inv).await {
        Ok(id) => (StatusCode::CREATED, Json(IdResponse { id })).into_response(),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePurchaseInvoiceBody {
    invoice_number: String,
    company_id: Uuid,
    #[serde(default)] branch_id: Option<Uuid>,
    supplier_id: Uuid,
    #[serde(default)] source_po_id: Option<Uuid>,
    posting_date: chrono::NaiveDate,
    #[serde(default)] due_date: Option<chrono::NaiveDate>,
    #[serde(default)] currency: Option<String>,
    payable_account_id: Uuid,
    lines: Vec<LineBody>,
    #[serde(default)] tax_lines: Vec<TaxLineBody>,
}
async fn create_purchase_invoice(State(svc): State<Arc<BillingWriteService>>, Json(b): Json<CreatePurchaseInvoiceBody>) -> axum::response::Response {
    let inv = NewPurchaseInvoice {
        invoice_number: b.invoice_number, company_id: b.company_id, branch_id: b.branch_id,
        supplier_id: b.supplier_id, source_po_id: b.source_po_id, posting_date: b.posting_date,
        due_date: b.due_date, currency: b.currency, payable_account_id: b.payable_account_id,
        lines: b.lines.into_iter().map(Into::into).collect(),
        tax_lines: b.tax_lines.into_iter().map(Into::into).collect(),
    };
    match svc.create_purchase_invoice(inv).await {
        Ok(id) => (StatusCode::CREATED, Json(IdResponse { id })).into_response(),
        Err(e) => err(e),
    }
}

fn write_routes(svc: Arc<BillingWriteService>) -> Router {
    Router::new()
        .route("/sales-invoices", post(create_sales_invoice))
        .route("/purchase-invoices", post(create_purchase_invoice))
        .with_state(svc)
}

/// Mount the billing module: read documents + validated creates. Generic mutation is not mounted.
/// **Prefer this over `BillingModule::all_crud_routes()` for any real deployment.**
pub fn create_guarded_billing_routes(m: &BillingModule, pool: PgPool) -> Router {
    let write = Arc::new(BillingWriteService::new(pool));
    Router::new()
        .merge(create_sales_invoice_read_routes(m.sales_invoice_service.clone()))
        .merge(create_purchase_invoice_read_routes(m.purchase_invoice_service.clone()))
        .merge(write_routes(write))
}

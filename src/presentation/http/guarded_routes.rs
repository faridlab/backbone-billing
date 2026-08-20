//! Guarded route composition — the RECOMMENDED way to mount the billing module.
//!
//! Hand-authored (user-owned). Read documents + **validated create** (sales-invoice /
//! purchase-invoice); generic create/update/delete CRUD is NOT mounted, so a caller cannot
//! write an invoice with inconsistent totals or bypass the AR/AP posting path.
//! Every write derives its tenant from a **signed** Bearer token (`CompanyContext`) rather than the
//! request body, so a caller cannot stamp an invoice with a company it does not belong to.
//! `BillingWriteService` is built from the pool (regen-safe). Posting (`post_sales_invoice` /
//! `post_purchase_invoice`) needs a `GlPostSink` composition layer, so it is service/job-driven,
//! not an HTTP route.

use std::sync::Arc;

use axum::{
    extract::State, http::StatusCode, middleware::from_fn_with_state, response::IntoResponse,
    routing::post, Json, Router,
};
use backbone_auth::company::{company_auth, CompanyContext, CompanyVerifier};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::service::billing_write_service::{
    BillingError, BillingWriteService, NewInvoiceLine, NewPurchaseInvoice, NewSalesInvoice,
    NewTaxLine,
};
use crate::BillingModule;

use super::{create_purchase_invoice_read_routes, create_sales_invoice_read_routes};

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}
#[derive(Debug, Serialize)]
struct IdResponse {
    id: Uuid,
}
fn err(e: BillingError) -> axum::response::Response {
    let s = StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        s,
        Json(ErrorBody {
            error: e.code(),
            message: e.to_string(),
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LineBody {
    item_id: Uuid,
    account_id: Uuid,
    #[serde(default)]
    description: Option<String>,
    quantity: Decimal,
    unit_price: Decimal,
    /// Optional tax template — supplying it on ANY line makes the document
    /// template-driven (the tax engine computes the overlay; supplied tax lines
    /// are ignored for that document).
    #[serde(default)]
    tax_template_id: Option<Uuid>,
}
impl From<LineBody> for NewInvoiceLine {
    fn from(b: LineBody) -> Self {
        NewInvoiceLine {
            item_id: b.item_id,
            account_id: b.account_id,
            description: b.description,
            quantity: b.quantity,
            unit_price: b.unit_price,
            tax_template_id: b.tax_template_id,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaxLineBody {
    account_id: Uuid,
    basis: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    rate: Decimal,
    tax_amount: Decimal,
}
impl From<TaxLineBody> for NewTaxLine {
    fn from(b: TaxLineBody) -> Self {
        NewTaxLine {
            account_id: b.account_id,
            basis: b.basis,
            description: b.description,
            rate: b.rate,
            tax_amount: b.tax_amount,
            taxable_base: Decimal::ZERO,
            tax_template_id: None,
            repartition_line_id: None,
            real_account_id: None,
            exigibility: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSalesInvoiceBody {
    invoice_number: String,
    // No `company_id` / `branch_id`: the tenant is derived from the signed token via
    // `CompanyContext`, never from the request body — a client must not be able to name the tenant
    // it writes into.
    customer_id: Uuid,
    #[serde(default)]
    source_so_id: Option<Uuid>,
    posting_date: chrono::NaiveDate,
    #[serde(default)]
    due_date: Option<chrono::NaiveDate>,
    #[serde(default)]
    currency: Option<String>,
    receivable_account_id: Uuid,
    lines: Vec<LineBody>,
    #[serde(default)]
    tax_lines: Vec<TaxLineBody>,
}
async fn create_sales_invoice(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
    Json(b): Json<CreateSalesInvoiceBody>,
) -> axum::response::Response {
    let inv = NewSalesInvoice {
        invoice_number: b.invoice_number,
        company_id: tenant.company_id,
        branch_id: tenant.branch_id,
        customer_id: b.customer_id,
        source_so_id: b.source_so_id,
        posting_date: b.posting_date,
        due_date: b.due_date,
        currency: b.currency,
        receivable_account_id: b.receivable_account_id,
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
    // Tenant comes from the signed token (`CompanyContext`), not the body.
    supplier_id: Uuid,
    #[serde(default)]
    source_po_id: Option<Uuid>,
    posting_date: chrono::NaiveDate,
    #[serde(default)]
    due_date: Option<chrono::NaiveDate>,
    #[serde(default)]
    currency: Option<String>,
    payable_account_id: Uuid,
    lines: Vec<LineBody>,
    #[serde(default)]
    tax_lines: Vec<TaxLineBody>,
}
async fn create_purchase_invoice(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
    Json(b): Json<CreatePurchaseInvoiceBody>,
) -> axum::response::Response {
    let inv = NewPurchaseInvoice {
        invoice_number: b.invoice_number,
        company_id: tenant.company_id,
        branch_id: tenant.branch_id,
        supplier_id: b.supplier_id,
        source_po_id: b.source_po_id,
        posting_date: b.posting_date,
        due_date: b.due_date,
        currency: b.currency,
        payable_account_id: b.payable_account_id,
        lines: b.lines.into_iter().map(Into::into).collect(),
        tax_lines: b.tax_lines.into_iter().map(Into::into).collect(),
    };
    match svc.create_purchase_invoice(inv).await {
        Ok(id) => (StatusCode::CREATED, Json(IdResponse { id })).into_response(),
        Err(e) => err(e),
    }
}

fn write_routes(svc: Arc<BillingWriteService>, verifier: CompanyVerifier) -> Router {
    Router::new()
        .route("/sales-invoices", post(create_sales_invoice))
        .route("/purchase-invoices", post(create_purchase_invoice))
        // Every write above is tenant-scoped: `company_auth` rejects a request whose token is absent,
        // invalid, or carries no `company_id`, so a handler only ever runs with a proven tenant.
        //
        // `route_layer`, not `layer`: `layer` would also wrap this router's fallback, so once merged
        // every *unmatched* path (e.g. the generic CRUD paths this surface deliberately does not
        // mount) would answer 401 instead of 404 — leaking "auth required" for routes that do not
        // exist, and masking the CRUD-bypass probes.
        .route_layer(from_fn_with_state(verifier, company_auth))
        .with_state(svc)
}

/// Mount the billing module: read documents + validated, tenant-scoped creates. Generic mutation is
/// not mounted. **Prefer this over `BillingModule::all_crud_routes()` for any real deployment.**
///
/// The composing service builds one [`CompanyVerifier`] from its JWT secret and passes it here; the
/// write surface derives `company_id` from the token, so no tenant crosses the wire in a body.
pub fn create_guarded_billing_routes(
    m: &BillingModule,
    pool: PgPool,
    verifier: CompanyVerifier,
) -> Router {
    let write = Arc::new(BillingWriteService::new(pool));
    Router::new()
        .merge(create_sales_invoice_read_routes(
            m.sales_invoice_service.clone(),
        ))
        .merge(create_purchase_invoice_read_routes(
            m.purchase_invoice_service.clone(),
        ))
        .merge(write_routes(write, verifier))
}

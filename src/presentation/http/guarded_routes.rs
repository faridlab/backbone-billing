//! Guarded route composition — the RECOMMENDED way to mount the billing module.
//!
//! Hand-authored (user-owned). Read documents + **validated create** (sales-invoice /
//! purchase-invoice); generic create/update/delete CRUD is NOT mounted, so a caller cannot
//! write an invoice with inconsistent totals or bypass the AR/AP posting path.
//! Every write derives its tenant from a **signed** Bearer token (`CompanyContext`) rather than the
//! request body, so a caller cannot stamp an invoice with a company it does not belong to.
//! `BillingWriteService` is passed in by the composing service so routes, verbs, and event
//! consumers all share one configured instance (regen-safe). Posting (`post_sales_invoice` /
//! `post_purchase_invoice`) needs a `GlPostSink` composition layer, so it is service/job-driven,
//! not an HTTP route.

use axum::{
    extract::State,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use backbone_auth::company::{company_auth, CompanyContext, CompanyVerifier};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
    payment_term_id: Option<Uuid>,
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
        payment_term_id: b.payment_term_id,
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
    payment_term_id: Option<Uuid>,
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
        payment_term_id: b.payment_term_id,
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
        .route(
            "/payment-terms",
            post(create_payment_term).get(list_payment_terms),
        )
        .route("/payment-terms/:id/preview", get(preview_payment_term))
        .route("/payment-terms/:id/status", put(set_payment_term_status))
        // Derived read (no stored overdue flag to drift): open, GL-posted invoices past due.
        .route("/overdue-invoices", get(list_overdue_invoices))
        // Every route above is tenant-scoped: `company_auth` rejects a request whose token is
        // absent, invalid, or carries no `company_id`, so a handler only ever runs with a proven
        // tenant.
        //
        // `route_layer`, not `layer`: `layer` would also wrap this router's fallback, so once merged
        // every *unmatched* path (e.g. the generic CRUD paths this surface deliberately does not
        // mount) would answer 401 instead of 404 — leaking "auth required" for routes that do not
        // exist, and masking the CRUD-bypass probes.
        .route_layer(from_fn_with_state(verifier, company_auth))
        .with_state(svc)
}

// ---- Payment terms -----------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TermLineBody {
    value: String,
    #[serde(default)]
    value_amount: Decimal,
    nb_days: i32,
    #[serde(default)]
    day_of_month: Option<i32>,
    #[serde(default = "default_delay_type")]
    delay_type: String,
    #[serde(default = "default_anchor")]
    anchor: String,
    #[serde(default = "default_sequence")]
    sequence: i32,
}
fn default_delay_type() -> String {
    "days".into()
}
fn default_anchor() -> String {
    "invoice_date".into()
}
fn default_sequence() -> i32 {
    10
}
impl From<TermLineBody> for crate::application::service::term_schedule::NewTermLine {
    fn from(b: TermLineBody) -> Self {
        Self {
            value: b.value,
            value_amount: b.value_amount,
            nb_days: b.nb_days,
            day_of_month: b.day_of_month,
            delay_type: b.delay_type,
            anchor: b.anchor,
            sequence: b.sequence,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePaymentTermBody {
    name: String,
    #[serde(default)]
    note: Option<String>,
    #[serde(default = "default_sequence")]
    sequence: i32,
    lines: Vec<TermLineBody>,
    #[serde(default)]
    early_discount: bool,
    #[serde(default)]
    discount_percent: Decimal,
    #[serde(default)]
    discount_days: i32,
    #[serde(default)]
    discount_account_id: Option<Uuid>,
    #[serde(default = "default_discount_tax_basis")]
    discount_tax_basis: String,
}
fn default_discount_tax_basis() -> String {
    "included".into()
}
async fn create_payment_term(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
    Json(b): Json<CreatePaymentTermBody>,
) -> axum::response::Response {
    let lines: Vec<_> = b.lines.into_iter().map(Into::into).collect();
    match svc
        .create_payment_term(
            tenant.company_id,
            &b.name,
            b.note.as_deref(),
            b.sequence,
            &lines,
            b.early_discount,
            b.discount_percent,
            b.discount_days,
            b.discount_account_id,
            &b.discount_tax_basis,
        )
        .await
    {
        Ok(id) => (StatusCode::CREATED, Json(IdResponse { id })).into_response(),
        Err(e) => err(e),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TermSummary {
    id: Uuid,
    name: String,
    is_global: bool,
    early_discount: bool,
    discount_percent: Decimal,
    discount_days: i32,
}
async fn list_payment_terms(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
) -> axum::response::Response {
    match svc.list_payment_terms(tenant.company_id).await {
        Ok(terms) => (
            StatusCode::OK,
            Json(
                terms
                    .into_iter()
                    .map(|t| TermSummary {
                        id: t.id,
                        name: t.name,
                        is_global: t.company_id.is_none(),
                        early_discount: t.early_discount,
                        discount_percent: t.discount_percent,
                        discount_days: t.discount_days,
                    })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewTermQuery {
    posting_date: chrono::NaiveDate,
    grand_total: Decimal,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleSlice {
    due_date: chrono::NaiveDate,
    amount: Decimal,
}
async fn preview_payment_term(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<PreviewTermQuery>,
) -> axum::response::Response {
    match svc
        .preview_payment_term(tenant.company_id, id, q.posting_date, q.grand_total)
        .await
    {
        Ok(slices) => (
            StatusCode::OK,
            Json(
                slices
                    .into_iter()
                    .map(|(due_date, amount)| ScheduleSlice { due_date, amount })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetTermStatusBody {
    status: String,
}
async fn set_payment_term_status(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(b): Json<SetTermStatusBody>,
) -> axum::response::Response {
    if b.status != "active" && b.status != "inactive" {
        return err(BillingError::TermInvalid {
            code: "term_status_invalid".into(),
            message: "status must be 'active' or 'inactive'".into(),
        });
    }
    match svc
        .set_payment_term_status(tenant.company_id, id, &b.status)
        .await
    {
        Ok(affected) if affected > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => err(BillingError::TermNotFound(id)),
        Err(e) => err(e),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OverdueInvoice {
    id: Uuid,
    /// "sales" | "purchase"
    kind: String,
    invoice_number: String,
    due_date: chrono::NaiveDate,
    outstanding_amount: Decimal,
    grand_total: Decimal,
    days_overdue: i64,
}
async fn list_overdue_invoices(
    State(svc): State<Arc<BillingWriteService>>,
    tenant: CompanyContext,
) -> axum::response::Response {
    let today = chrono::Utc::now().date_naive();
    match svc.list_overdue_invoices(tenant.company_id, today).await {
        Ok(rows) => (
            StatusCode::OK,
            Json(
                rows.into_iter()
                    .map(|r| OverdueInvoice {
                        id: r.id,
                        kind: r.kind,
                        invoice_number: r.invoice_number,
                        due_date: r.due_date,
                        outstanding_amount: r.outstanding_amount,
                        grand_total: r.grand_total,
                        days_overdue: (today - r.due_date).num_days(),
                    })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(e) => err(e),
    }
}

/// Mount the billing module: read documents + validated, tenant-scoped creates. Generic mutation is
/// not mounted. **Prefer this over `BillingModule::all_crud_routes()` for any real deployment.**
///
/// The composing service builds one [`CompanyVerifier`] from its JWT secret and passes it here; the
/// write surface derives `company_id` from the token, so no tenant crosses the wire in a body.
///
/// The write service is passed in — the SAME configured instance the host wires its settlement
/// consumers and finance verbs to. Constructing one here would silently fork the configuration:
/// a host-set collaborator (e.g. the tax engine behind template-driven lines) would exist on the
/// verbs' instance but not the create routes', and template lines would be refused with
/// `tax_engine_unwired` at runtime while every suite passes.
pub fn create_guarded_billing_routes(
    m: &BillingModule,
    write: Arc<BillingWriteService>,
    verifier: CompanyVerifier,
) -> Router {
    Router::new()
        .merge(create_sales_invoice_read_routes(
            m.sales_invoice_service.clone(),
        ))
        .merge(create_purchase_invoice_read_routes(
            m.purchase_invoice_service.clone(),
        ))
        .merge(write_routes(write, verifier))
}

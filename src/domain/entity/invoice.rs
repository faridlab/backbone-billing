use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::InvoiceType;
use super::InvoiceableType;
use super::InvoiceStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Invoice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvoiceId(pub Uuid);

impl InvoiceId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for InvoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for InvoiceId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for InvoiceId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<InvoiceId> for Uuid {
    fn from(id: InvoiceId) -> Self { id.0 }
}

impl AsRef<Uuid> for InvoiceId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for InvoiceId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub id: Uuid,
    pub invoice_number: String,
    pub invoice_type: InvoiceType,
    pub invoiceable_type: InvoiceableType,
    pub invoiceable_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<Uuid>,
    pub issue_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<NaiveDate>,
    pub currency: String,
    pub subtotal: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<Decimal>,
    pub total_amount: Decimal,
    pub status: InvoiceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_via: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_generated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    pub data: serde_json::Value,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Invoice {
    /// Create a builder for Invoice
    pub fn builder() -> InvoiceBuilder {
        InvoiceBuilder::default()
    }

    /// Create a new Invoice with required fields
    pub fn new(invoice_number: String, invoice_type: InvoiceType, invoiceable_type: InvoiceableType, invoiceable_id: Uuid, issue_date: NaiveDate, currency: String, subtotal: Decimal, discount_amount: Decimal, tax_amount: Decimal, total_amount: Decimal, status: InvoiceStatus, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            invoice_number,
            invoice_type,
            invoiceable_type,
            invoiceable_id,
            order_id: None,
            settlement_id: None,
            issue_date,
            due_date: None,
            currency,
            subtotal,
            discount_amount,
            tax_amount,
            tax_rate: None,
            total_amount,
            status,
            sent_via: None,
            paid_at: None,
            payment_id: None,
            pdf_url: None,
            pdf_generated_at: None,
            notes: None,
            terms: None,
            footer: None,
            data,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> InvoiceId {
        InvoiceId(self.id)
    }

    /// Get when this entity was created
    pub fn created_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.created_at.as_ref()
    }

    /// Get when this entity was last updated
    pub fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.updated_at.as_ref()
    }

    /// Check if this entity is soft deleted
    pub fn is_deleted(&self) -> bool {
        self.metadata.deleted_at.is_some()
    }

    /// Check if this entity is active (not deleted)
    pub fn is_active(&self) -> bool {
        self.metadata.deleted_at.is_none()
    }

    /// Get when this entity was deleted
    pub fn deleted_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.deleted_at.as_ref()
    }

    /// Get who created this entity
    pub fn created_by(&self) -> Option<&Uuid> {
        self.metadata.created_by.as_ref()
    }

    /// Get who last updated this entity
    pub fn updated_by(&self) -> Option<&Uuid> {
        self.metadata.updated_by.as_ref()
    }

    /// Get who deleted this entity
    pub fn deleted_by(&self) -> Option<&Uuid> {
        self.metadata.deleted_by.as_ref()
    }

    /// Get the current status
    pub fn status(&self) -> &InvoiceStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the order_id field (chainable)
    pub fn with_order_id(mut self, value: Uuid) -> Self {
        self.order_id = Some(value);
        self
    }

    /// Set the settlement_id field (chainable)
    pub fn with_settlement_id(mut self, value: Uuid) -> Self {
        self.settlement_id = Some(value);
        self
    }

    /// Set the due_date field (chainable)
    pub fn with_due_date(mut self, value: NaiveDate) -> Self {
        self.due_date = Some(value);
        self
    }

    /// Set the tax_rate field (chainable)
    pub fn with_tax_rate(mut self, value: Decimal) -> Self {
        self.tax_rate = Some(value);
        self
    }

    /// Set the sent_via field (chainable)
    pub fn with_sent_via(mut self, value: String) -> Self {
        self.sent_via = Some(value);
        self
    }

    /// Set the paid_at field (chainable)
    pub fn with_paid_at(mut self, value: DateTime<Utc>) -> Self {
        self.paid_at = Some(value);
        self
    }

    /// Set the payment_id field (chainable)
    pub fn with_payment_id(mut self, value: Uuid) -> Self {
        self.payment_id = Some(value);
        self
    }

    /// Set the pdf_url field (chainable)
    pub fn with_pdf_url(mut self, value: String) -> Self {
        self.pdf_url = Some(value);
        self
    }

    /// Set the pdf_generated_at field (chainable)
    pub fn with_pdf_generated_at(mut self, value: DateTime<Utc>) -> Self {
        self.pdf_generated_at = Some(value);
        self
    }

    /// Set the notes field (chainable)
    pub fn with_notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Set the terms field (chainable)
    pub fn with_terms(mut self, value: String) -> Self {
        self.terms = Some(value);
        self
    }

    /// Set the footer field (chainable)
    pub fn with_footer(mut self, value: String) -> Self {
        self.footer = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "invoice_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoice_number = v; }
                }
                "invoice_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoice_type = v; }
                }
                "invoiceable_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoiceable_type = v; }
                }
                "invoiceable_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoiceable_id = v; }
                }
                "order_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.order_id = v; }
                }
                "settlement_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.settlement_id = v; }
                }
                "issue_date" => {
                    if let Ok(v) = serde_json::from_value(value) { self.issue_date = v; }
                }
                "due_date" => {
                    if let Ok(v) = serde_json::from_value(value) { self.due_date = v; }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.currency = v; }
                }
                "subtotal" => {
                    if let Ok(v) = serde_json::from_value(value) { self.subtotal = v; }
                }
                "discount_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.discount_amount = v; }
                }
                "tax_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.tax_amount = v; }
                }
                "tax_rate" => {
                    if let Ok(v) = serde_json::from_value(value) { self.tax_rate = v; }
                }
                "total_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.total_amount = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "sent_via" => {
                    if let Ok(v) = serde_json::from_value(value) { self.sent_via = v; }
                }
                "paid_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.paid_at = v; }
                }
                "payment_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payment_id = v; }
                }
                "pdf_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.pdf_url = v; }
                }
                "pdf_generated_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.pdf_generated_at = v; }
                }
                "notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.notes = v; }
                }
                "terms" => {
                    if let Ok(v) = serde_json::from_value(value) { self.terms = v; }
                }
                "footer" => {
                    if let Ok(v) = serde_json::from_value(value) { self.footer = v; }
                }
                "data" => {
                    if let Ok(v) = serde_json::from_value(value) { self.data = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for Invoice {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Invoice"
    }
}

impl backbone_core::PersistentEntity for Invoice {
    fn entity_id(&self) -> String {
        self.id.to_string()
    }
    fn set_entity_id(&mut self, id: String) {
        if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            self.id = uuid;
        }
    }
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.created_at
    }
    fn set_created_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.created_at = Some(ts);
    }
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.updated_at
    }
    fn set_updated_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.updated_at = Some(ts);
    }
    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.deleted_at
    }
    fn set_deleted_at(&mut self, ts: Option<chrono::DateTime<chrono::Utc>>) {
        self.metadata.deleted_at = ts;
    }
}

impl backbone_orm::EntityRepoMeta for Invoice {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("invoiceable_id".to_string(), "uuid".to_string());
        m.insert("order_id".to_string(), "uuid".to_string());
        m.insert("settlement_id".to_string(), "uuid".to_string());
        m.insert("payment_id".to_string(), "uuid".to_string());
        m.insert("invoice_type".to_string(), "invoice_type".to_string());
        m.insert("invoiceable_type".to_string(), "invoiceable_type".to_string());
        m.insert("status".to_string(), "invoice_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["invoice_number", "currency"]
    }
}

/// Builder for Invoice entity
///
/// Provides a fluent API for constructing Invoice instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct InvoiceBuilder {
    invoice_number: Option<String>,
    invoice_type: Option<InvoiceType>,
    invoiceable_type: Option<InvoiceableType>,
    invoiceable_id: Option<Uuid>,
    order_id: Option<Uuid>,
    settlement_id: Option<Uuid>,
    issue_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    currency: Option<String>,
    subtotal: Option<Decimal>,
    discount_amount: Option<Decimal>,
    tax_amount: Option<Decimal>,
    tax_rate: Option<Decimal>,
    total_amount: Option<Decimal>,
    status: Option<InvoiceStatus>,
    sent_via: Option<String>,
    paid_at: Option<DateTime<Utc>>,
    payment_id: Option<Uuid>,
    pdf_url: Option<String>,
    pdf_generated_at: Option<DateTime<Utc>>,
    notes: Option<String>,
    terms: Option<String>,
    footer: Option<String>,
    data: Option<serde_json::Value>,
}

impl InvoiceBuilder {
    /// Set the invoice_number field (required)
    pub fn invoice_number(mut self, value: String) -> Self {
        self.invoice_number = Some(value);
        self
    }

    /// Set the invoice_type field (default: `InvoiceType::default()`)
    pub fn invoice_type(mut self, value: InvoiceType) -> Self {
        self.invoice_type = Some(value);
        self
    }

    /// Set the invoiceable_type field (required)
    pub fn invoiceable_type(mut self, value: InvoiceableType) -> Self {
        self.invoiceable_type = Some(value);
        self
    }

    /// Set the invoiceable_id field (required)
    pub fn invoiceable_id(mut self, value: Uuid) -> Self {
        self.invoiceable_id = Some(value);
        self
    }

    /// Set the order_id field (optional)
    pub fn order_id(mut self, value: Uuid) -> Self {
        self.order_id = Some(value);
        self
    }

    /// Set the settlement_id field (optional)
    pub fn settlement_id(mut self, value: Uuid) -> Self {
        self.settlement_id = Some(value);
        self
    }

    /// Set the issue_date field (required)
    pub fn issue_date(mut self, value: NaiveDate) -> Self {
        self.issue_date = Some(value);
        self
    }

    /// Set the due_date field (optional)
    pub fn due_date(mut self, value: NaiveDate) -> Self {
        self.due_date = Some(value);
        self
    }

    /// Set the currency field (default: `"IDR".to_string()`)
    pub fn currency(mut self, value: String) -> Self {
        self.currency = Some(value);
        self
    }

    /// Set the subtotal field (required)
    pub fn subtotal(mut self, value: Decimal) -> Self {
        self.subtotal = Some(value);
        self
    }

    /// Set the discount_amount field (default: `Decimal::from(0)`)
    pub fn discount_amount(mut self, value: Decimal) -> Self {
        self.discount_amount = Some(value);
        self
    }

    /// Set the tax_amount field (default: `Decimal::from(0)`)
    pub fn tax_amount(mut self, value: Decimal) -> Self {
        self.tax_amount = Some(value);
        self
    }

    /// Set the tax_rate field (optional)
    pub fn tax_rate(mut self, value: Decimal) -> Self {
        self.tax_rate = Some(value);
        self
    }

    /// Set the total_amount field (required)
    pub fn total_amount(mut self, value: Decimal) -> Self {
        self.total_amount = Some(value);
        self
    }

    /// Set the status field (default: `InvoiceStatus::default()`)
    pub fn status(mut self, value: InvoiceStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the sent_via field (optional)
    pub fn sent_via(mut self, value: String) -> Self {
        self.sent_via = Some(value);
        self
    }

    /// Set the paid_at field (optional)
    pub fn paid_at(mut self, value: DateTime<Utc>) -> Self {
        self.paid_at = Some(value);
        self
    }

    /// Set the payment_id field (optional)
    pub fn payment_id(mut self, value: Uuid) -> Self {
        self.payment_id = Some(value);
        self
    }

    /// Set the pdf_url field (optional)
    pub fn pdf_url(mut self, value: String) -> Self {
        self.pdf_url = Some(value);
        self
    }

    /// Set the pdf_generated_at field (optional)
    pub fn pdf_generated_at(mut self, value: DateTime<Utc>) -> Self {
        self.pdf_generated_at = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Set the terms field (optional)
    pub fn terms(mut self, value: String) -> Self {
        self.terms = Some(value);
        self
    }

    /// Set the footer field (optional)
    pub fn footer(mut self, value: String) -> Self {
        self.footer = Some(value);
        self
    }

    /// Set the data field (default: `serde_json::json!({"paid_amount":0,"balance_due":0,"sent_at":null,"viewed_at":null})`)
    pub fn data(mut self, value: serde_json::Value) -> Self {
        self.data = Some(value);
        self
    }

    /// Build the Invoice entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Invoice, String> {
        let invoice_number = self.invoice_number.ok_or_else(|| "invoice_number is required".to_string())?;
        let invoiceable_type = self.invoiceable_type.ok_or_else(|| "invoiceable_type is required".to_string())?;
        let invoiceable_id = self.invoiceable_id.ok_or_else(|| "invoiceable_id is required".to_string())?;
        let issue_date = self.issue_date.ok_or_else(|| "issue_date is required".to_string())?;
        let subtotal = self.subtotal.ok_or_else(|| "subtotal is required".to_string())?;
        let total_amount = self.total_amount.ok_or_else(|| "total_amount is required".to_string())?;

        Ok(Invoice {
            id: Uuid::new_v4(),
            invoice_number,
            invoice_type: self.invoice_type.unwrap_or(InvoiceType::default()),
            invoiceable_type,
            invoiceable_id,
            order_id: self.order_id,
            settlement_id: self.settlement_id,
            issue_date,
            due_date: self.due_date,
            currency: self.currency.unwrap_or("IDR".to_string()),
            subtotal,
            discount_amount: self.discount_amount.unwrap_or(Decimal::from(0)),
            tax_amount: self.tax_amount.unwrap_or(Decimal::from(0)),
            tax_rate: self.tax_rate,
            total_amount,
            status: self.status.unwrap_or(InvoiceStatus::default()),
            sent_via: self.sent_via,
            paid_at: self.paid_at,
            payment_id: self.payment_id,
            pdf_url: self.pdf_url,
            pdf_generated_at: self.pdf_generated_at,
            notes: self.notes,
            terms: self.terms,
            footer: self.footer,
            data: self.data.unwrap_or(serde_json::json!({"paid_amount":0,"balance_due":0,"sent_at":null,"viewed_at":null})),
            metadata: AuditMetadata::default(),
        })
    }
}

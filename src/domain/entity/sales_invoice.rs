use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::AuditMetadata;
use super::GlPostingState;
use super::InvoiceStatus;

/// Strongly-typed ID for SalesInvoice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SalesInvoiceId(pub Uuid);

impl SalesInvoiceId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for SalesInvoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SalesInvoiceId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for SalesInvoiceId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<SalesInvoiceId> for Uuid {
    fn from(id: SalesInvoiceId) -> Self {
        id.0
    }
}

impl AsRef<Uuid> for SalesInvoiceId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::ops::Deref for SalesInvoiceId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SalesInvoice {
    pub id: Uuid,
    pub invoice_number: String,
    pub company_id: Uuid,
    pub branch_id: Option<Uuid>,
    pub customer_id: Uuid,
    pub source_so_id: Option<Uuid>,
    pub status: InvoiceStatus,
    pub posting_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub currency: String,
    pub net_total: Decimal,
    pub tax_total: Decimal,
    pub grand_total: Decimal,
    pub outstanding_amount: Decimal,
    pub receivable_account_id: Uuid,
    pub posting_state: GlPostingState,
    pub journal_id: Option<Uuid>,
    pub accounting_post_id: Option<Uuid>,
    pub posted_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl SalesInvoice {
    /// Create a builder for SalesInvoice
    pub fn builder() -> SalesInvoiceBuilder {
        SalesInvoiceBuilder::default()
    }

    /// Create a new SalesInvoice with required fields
    pub fn new(
        invoice_number: String,
        company_id: Uuid,
        customer_id: Uuid,
        status: InvoiceStatus,
        posting_date: NaiveDate,
        currency: String,
        net_total: Decimal,
        tax_total: Decimal,
        grand_total: Decimal,
        outstanding_amount: Decimal,
        receivable_account_id: Uuid,
        posting_state: GlPostingState,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            invoice_number,
            company_id,
            branch_id: None,
            customer_id,
            source_so_id: None,
            status,
            posting_date,
            due_date: None,
            currency,
            net_total,
            tax_total,
            grand_total,
            outstanding_amount,
            receivable_account_id,
            posting_state,
            journal_id: None,
            accounting_post_id: None,
            posted_at: None,
            notes: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> SalesInvoiceId {
        SalesInvoiceId(self.id)
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

    /// Set the branch_id field (chainable)
    pub fn with_branch_id(mut self, value: Uuid) -> Self {
        self.branch_id = Some(value);
        self
    }

    /// Set the source_so_id field (chainable)
    pub fn with_source_so_id(mut self, value: Uuid) -> Self {
        self.source_so_id = Some(value);
        self
    }

    /// Set the due_date field (chainable)
    pub fn with_due_date(mut self, value: NaiveDate) -> Self {
        self.due_date = Some(value);
        self
    }

    /// Set the journal_id field (chainable)
    pub fn with_journal_id(mut self, value: Uuid) -> Self {
        self.journal_id = Some(value);
        self
    }

    /// Set the accounting_post_id field (chainable)
    pub fn with_accounting_post_id(mut self, value: Uuid) -> Self {
        self.accounting_post_id = Some(value);
        self
    }

    /// Set the posted_at field (chainable)
    pub fn with_posted_at(mut self, value: DateTime<Utc>) -> Self {
        self.posted_at = Some(value);
        self
    }

    /// Set the notes field (chainable)
    pub fn with_notes(mut self, value: String) -> Self {
        self.notes = Some(value);
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
                    if let Ok(v) = serde_json::from_value(value) {
                        self.invoice_number = v;
                    }
                }
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.company_id = v;
                    }
                }
                "branch_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.branch_id = v;
                    }
                }
                "customer_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.customer_id = v;
                    }
                }
                "source_so_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.source_so_id = v;
                    }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.status = v;
                    }
                }
                "posting_date" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.posting_date = v;
                    }
                }
                "due_date" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.due_date = v;
                    }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.currency = v;
                    }
                }
                "net_total" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.net_total = v;
                    }
                }
                "tax_total" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.tax_total = v;
                    }
                }
                "grand_total" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.grand_total = v;
                    }
                }
                "outstanding_amount" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.outstanding_amount = v;
                    }
                }
                "receivable_account_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.receivable_account_id = v;
                    }
                }
                "posting_state" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.posting_state = v;
                    }
                }
                "journal_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.journal_id = v;
                    }
                }
                "accounting_post_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.accounting_post_id = v;
                    }
                }
                "posted_at" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.posted_at = v;
                    }
                }
                "notes" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.notes = v;
                    }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for SalesInvoice {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "SalesInvoice"
    }
}

impl backbone_core::PersistentEntity for SalesInvoice {
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

impl backbone_orm::EntityRepoMeta for SalesInvoice {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("branch_id".to_string(), "uuid".to_string());
        m.insert("customer_id".to_string(), "uuid".to_string());
        m.insert("source_so_id".to_string(), "uuid".to_string());
        m.insert("receivable_account_id".to_string(), "uuid".to_string());
        m.insert("journal_id".to_string(), "uuid".to_string());
        m.insert("accounting_post_id".to_string(), "uuid".to_string());
        m.insert("status".to_string(), "invoice_status".to_string());
        m.insert("posting_state".to_string(), "gl_posting_state".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["invoice_number", "currency"]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
}

/// Builder for SalesInvoice entity
///
/// Provides a fluent API for constructing SalesInvoice instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct SalesInvoiceBuilder {
    invoice_number: Option<String>,
    company_id: Option<Uuid>,
    branch_id: Option<Uuid>,
    customer_id: Option<Uuid>,
    source_so_id: Option<Uuid>,
    status: Option<InvoiceStatus>,
    posting_date: Option<NaiveDate>,
    due_date: Option<NaiveDate>,
    currency: Option<String>,
    net_total: Option<Decimal>,
    tax_total: Option<Decimal>,
    grand_total: Option<Decimal>,
    outstanding_amount: Option<Decimal>,
    receivable_account_id: Option<Uuid>,
    posting_state: Option<GlPostingState>,
    journal_id: Option<Uuid>,
    accounting_post_id: Option<Uuid>,
    posted_at: Option<DateTime<Utc>>,
    notes: Option<String>,
}

impl SalesInvoiceBuilder {
    /// Set the invoice_number field (required)
    pub fn invoice_number(mut self, value: String) -> Self {
        self.invoice_number = Some(value);
        self
    }

    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the branch_id field (optional)
    pub fn branch_id(mut self, value: Uuid) -> Self {
        self.branch_id = Some(value);
        self
    }

    /// Set the customer_id field (required)
    pub fn customer_id(mut self, value: Uuid) -> Self {
        self.customer_id = Some(value);
        self
    }

    /// Set the source_so_id field (optional)
    pub fn source_so_id(mut self, value: Uuid) -> Self {
        self.source_so_id = Some(value);
        self
    }

    /// Set the status field (default: `InvoiceStatus::default()`)
    pub fn status(mut self, value: InvoiceStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the posting_date field (required)
    pub fn posting_date(mut self, value: NaiveDate) -> Self {
        self.posting_date = Some(value);
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

    /// Set the net_total field (default: `Decimal::from(0)`)
    pub fn net_total(mut self, value: Decimal) -> Self {
        self.net_total = Some(value);
        self
    }

    /// Set the tax_total field (default: `Decimal::from(0)`)
    pub fn tax_total(mut self, value: Decimal) -> Self {
        self.tax_total = Some(value);
        self
    }

    /// Set the grand_total field (default: `Decimal::from(0)`)
    pub fn grand_total(mut self, value: Decimal) -> Self {
        self.grand_total = Some(value);
        self
    }

    /// Set the outstanding_amount field (default: `Decimal::from(0)`)
    pub fn outstanding_amount(mut self, value: Decimal) -> Self {
        self.outstanding_amount = Some(value);
        self
    }

    /// Set the receivable_account_id field (required)
    pub fn receivable_account_id(mut self, value: Uuid) -> Self {
        self.receivable_account_id = Some(value);
        self
    }

    /// Set the posting_state field (default: `GlPostingState::default()`)
    pub fn posting_state(mut self, value: GlPostingState) -> Self {
        self.posting_state = Some(value);
        self
    }

    /// Set the journal_id field (optional)
    pub fn journal_id(mut self, value: Uuid) -> Self {
        self.journal_id = Some(value);
        self
    }

    /// Set the accounting_post_id field (optional)
    pub fn accounting_post_id(mut self, value: Uuid) -> Self {
        self.accounting_post_id = Some(value);
        self
    }

    /// Set the posted_at field (optional)
    pub fn posted_at(mut self, value: DateTime<Utc>) -> Self {
        self.posted_at = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Build the SalesInvoice entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<SalesInvoice, String> {
        let invoice_number = self
            .invoice_number
            .ok_or_else(|| "invoice_number is required".to_string())?;
        let company_id = self
            .company_id
            .ok_or_else(|| "company_id is required".to_string())?;
        let customer_id = self
            .customer_id
            .ok_or_else(|| "customer_id is required".to_string())?;
        let posting_date = self
            .posting_date
            .ok_or_else(|| "posting_date is required".to_string())?;
        let receivable_account_id = self
            .receivable_account_id
            .ok_or_else(|| "receivable_account_id is required".to_string())?;

        Ok(SalesInvoice {
            id: Uuid::new_v4(),
            invoice_number,
            company_id,
            branch_id: self.branch_id,
            customer_id,
            source_so_id: self.source_so_id,
            status: self.status.unwrap_or(InvoiceStatus::default()),
            posting_date,
            due_date: self.due_date,
            currency: self.currency.unwrap_or("IDR".to_string()),
            net_total: self.net_total.unwrap_or(Decimal::from(0)),
            tax_total: self.tax_total.unwrap_or(Decimal::from(0)),
            grand_total: self.grand_total.unwrap_or(Decimal::from(0)),
            outstanding_amount: self.outstanding_amount.unwrap_or(Decimal::from(0)),
            receivable_account_id,
            posting_state: self.posting_state.unwrap_or(GlPostingState::default()),
            journal_id: self.journal_id,
            accounting_post_id: self.accounting_post_id,
            posted_at: self.posted_at,
            notes: self.notes,
            metadata: AuditMetadata::default(),
        })
    }
}

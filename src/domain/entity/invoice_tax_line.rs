use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::AuditMetadata;
use super::InvoiceKind;
use super::TaxBasis;

/// Strongly-typed ID for InvoiceTaxLine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvoiceTaxLineId(pub Uuid);

impl InvoiceTaxLineId {
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

impl std::fmt::Display for InvoiceTaxLineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for InvoiceTaxLineId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for InvoiceTaxLineId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<InvoiceTaxLineId> for Uuid {
    fn from(id: InvoiceTaxLineId) -> Self {
        id.0
    }
}

impl AsRef<Uuid> for InvoiceTaxLineId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::ops::Deref for InvoiceTaxLineId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceTaxLine {
    pub id: Uuid,
    pub invoice_ref: Uuid,
    pub invoice_kind: InvoiceKind,
    pub company_id: Uuid,
    pub account_id: Uuid,
    pub basis: TaxBasis,
    pub description: Option<String>,
    pub taxable_base: Decimal,
    pub rate: Decimal,
    pub tax_amount: Decimal,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl InvoiceTaxLine {
    /// Create a builder for InvoiceTaxLine
    pub fn builder() -> InvoiceTaxLineBuilder {
        InvoiceTaxLineBuilder::default()
    }

    /// Create a new InvoiceTaxLine with required fields
    pub fn new(
        invoice_ref: Uuid,
        invoice_kind: InvoiceKind,
        company_id: Uuid,
        account_id: Uuid,
        basis: TaxBasis,
        taxable_base: Decimal,
        rate: Decimal,
        tax_amount: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            invoice_ref,
            invoice_kind,
            company_id,
            account_id,
            basis,
            description: None,
            taxable_base,
            rate,
            tax_amount,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> InvoiceTaxLineId {
        InvoiceTaxLineId(self.id)
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

    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the description field (chainable)
    pub fn with_description(mut self, value: String) -> Self {
        self.description = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "invoice_ref" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.invoice_ref = v;
                    }
                }
                "invoice_kind" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.invoice_kind = v;
                    }
                }
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.company_id = v;
                    }
                }
                "account_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.account_id = v;
                    }
                }
                "basis" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.basis = v;
                    }
                }
                "description" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.description = v;
                    }
                }
                "taxable_base" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.taxable_base = v;
                    }
                }
                "rate" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.rate = v;
                    }
                }
                "tax_amount" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.tax_amount = v;
                    }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for InvoiceTaxLine {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "InvoiceTaxLine"
    }
}

impl backbone_core::PersistentEntity for InvoiceTaxLine {
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

impl backbone_orm::EntityRepoMeta for InvoiceTaxLine {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("account_id".to_string(), "uuid".to_string());
        m.insert("invoice_kind".to_string(), "invoice_kind".to_string());
        m.insert("basis".to_string(), "tax_basis".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
}

/// Builder for InvoiceTaxLine entity
///
/// Provides a fluent API for constructing InvoiceTaxLine instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct InvoiceTaxLineBuilder {
    invoice_ref: Option<Uuid>,
    invoice_kind: Option<InvoiceKind>,
    company_id: Option<Uuid>,
    account_id: Option<Uuid>,
    basis: Option<TaxBasis>,
    description: Option<String>,
    taxable_base: Option<Decimal>,
    rate: Option<Decimal>,
    tax_amount: Option<Decimal>,
}

impl InvoiceTaxLineBuilder {
    /// Set the invoice_ref field (required)
    pub fn invoice_ref(mut self, value: Uuid) -> Self {
        self.invoice_ref = Some(value);
        self
    }

    /// Set the invoice_kind field (required)
    pub fn invoice_kind(mut self, value: InvoiceKind) -> Self {
        self.invoice_kind = Some(value);
        self
    }

    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the account_id field (required)
    pub fn account_id(mut self, value: Uuid) -> Self {
        self.account_id = Some(value);
        self
    }

    /// Set the basis field (required)
    pub fn basis(mut self, value: TaxBasis) -> Self {
        self.basis = Some(value);
        self
    }

    /// Set the description field (optional)
    pub fn description(mut self, value: String) -> Self {
        self.description = Some(value);
        self
    }

    /// Set the taxable_base field (default: `Decimal::from(0)`)
    pub fn taxable_base(mut self, value: Decimal) -> Self {
        self.taxable_base = Some(value);
        self
    }

    /// Set the rate field (default: `Decimal::from(0)`)
    pub fn rate(mut self, value: Decimal) -> Self {
        self.rate = Some(value);
        self
    }

    /// Set the tax_amount field (required)
    pub fn tax_amount(mut self, value: Decimal) -> Self {
        self.tax_amount = Some(value);
        self
    }

    /// Build the InvoiceTaxLine entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<InvoiceTaxLine, String> {
        let invoice_ref = self
            .invoice_ref
            .ok_or_else(|| "invoice_ref is required".to_string())?;
        let invoice_kind = self
            .invoice_kind
            .ok_or_else(|| "invoice_kind is required".to_string())?;
        let company_id = self
            .company_id
            .ok_or_else(|| "company_id is required".to_string())?;
        let account_id = self
            .account_id
            .ok_or_else(|| "account_id is required".to_string())?;
        let basis = self.basis.ok_or_else(|| "basis is required".to_string())?;
        let tax_amount = self
            .tax_amount
            .ok_or_else(|| "tax_amount is required".to_string())?;

        Ok(InvoiceTaxLine {
            id: Uuid::new_v4(),
            invoice_ref,
            invoice_kind,
            company_id,
            account_id,
            basis,
            description: self.description,
            taxable_base: self.taxable_base.unwrap_or(Decimal::from(0)),
            rate: self.rate.unwrap_or(Decimal::from(0)),
            tax_amount,
            metadata: AuditMetadata::default(),
        })
    }
}

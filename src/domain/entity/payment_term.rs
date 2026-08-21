use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::AuditMetadata;
use super::DiscountTaxBasis;
use super::PaymentTermStatus;

/// Strongly-typed ID for PaymentTerm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaymentTermId(pub Uuid);

impl PaymentTermId {
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

impl std::fmt::Display for PaymentTermId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PaymentTermId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for PaymentTermId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<PaymentTermId> for Uuid {
    fn from(id: PaymentTermId) -> Self {
        id.0
    }
}

impl AsRef<Uuid> for PaymentTermId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::ops::Deref for PaymentTermId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentTerm {
    pub id: Uuid,
    pub company_id: Option<Uuid>,
    pub name: String,
    pub note: Option<String>,
    pub sequence: i32,
    pub status: PaymentTermStatus,
    pub early_discount: bool,
    pub discount_percent: Decimal,
    pub discount_days: i32,
    pub discount_account_id: Option<Uuid>,
    pub discount_tax_basis: DiscountTaxBasis,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl PaymentTerm {
    /// Create a builder for PaymentTerm
    pub fn builder() -> PaymentTermBuilder {
        <PaymentTermBuilder as Default>::default()
    }

    /// Create a new PaymentTerm with required fields
    pub fn new(
        name: String,
        sequence: i32,
        status: PaymentTermStatus,
        early_discount: bool,
        discount_percent: Decimal,
        discount_days: i32,
        discount_tax_basis: DiscountTaxBasis,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id: None,
            name,
            note: None,
            sequence,
            status,
            early_discount,
            discount_percent,
            discount_days,
            discount_account_id: None,
            discount_tax_basis,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> PaymentTermId {
        PaymentTermId(self.id)
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
    pub fn status(&self) -> &PaymentTermStatus {
        &self.status
    }

    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the company_id field (chainable)
    pub fn with_company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the note field (chainable)
    pub fn with_note(mut self, value: String) -> Self {
        self.note = Some(value);
        self
    }

    /// Set the discount_account_id field (chainable)
    pub fn with_discount_account_id(mut self, value: Uuid) -> Self {
        self.discount_account_id = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.company_id = v;
                    }
                }
                "name" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.name = v;
                    }
                }
                "note" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.note = v;
                    }
                }
                "sequence" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.sequence = v;
                    }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.status = v;
                    }
                }
                "early_discount" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.early_discount = v;
                    }
                }
                "discount_percent" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.discount_percent = v;
                    }
                }
                "discount_days" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.discount_days = v;
                    }
                }
                "discount_account_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.discount_account_id = v;
                    }
                }
                "discount_tax_basis" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.discount_tax_basis = v;
                    }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for PaymentTerm {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "PaymentTerm"
    }
}

impl backbone_core::PersistentEntity for PaymentTerm {
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

impl backbone_orm::EntityRepoMeta for PaymentTerm {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("discount_account_id".to_string(), "uuid".to_string());
        m.insert("status".to_string(), "payment_term_status".to_string());
        m.insert(
            "discount_tax_basis".to_string(),
            "discount_tax_basis".to_string(),
        );
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["name"]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
}

/// Builder for PaymentTerm entity
///
/// Provides a fluent API for constructing PaymentTerm instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct PaymentTermBuilder {
    company_id: Option<Uuid>,
    name: Option<String>,
    note: Option<String>,
    sequence: Option<i32>,
    status: Option<PaymentTermStatus>,
    early_discount: Option<bool>,
    discount_percent: Option<Decimal>,
    discount_days: Option<i32>,
    discount_account_id: Option<Uuid>,
    discount_tax_basis: Option<DiscountTaxBasis>,
}

impl PaymentTermBuilder {
    /// Set the company_id field (optional)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the name field (required)
    pub fn name(mut self, value: String) -> Self {
        self.name = Some(value);
        self
    }

    /// Set the note field (optional)
    pub fn note(mut self, value: String) -> Self {
        self.note = Some(value);
        self
    }

    /// Set the sequence field (default: `10`)
    pub fn sequence(mut self, value: i32) -> Self {
        self.sequence = Some(value);
        self
    }

    /// Set the status field (default: `PaymentTermStatus::default()`)
    pub fn status(mut self, value: PaymentTermStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the early_discount field (default: `false`)
    pub fn early_discount(mut self, value: bool) -> Self {
        self.early_discount = Some(value);
        self
    }

    /// Set the discount_percent field (default: `Decimal::from(0)`)
    pub fn discount_percent(mut self, value: Decimal) -> Self {
        self.discount_percent = Some(value);
        self
    }

    /// Set the discount_days field (default: `0`)
    pub fn discount_days(mut self, value: i32) -> Self {
        self.discount_days = Some(value);
        self
    }

    /// Set the discount_account_id field (optional)
    pub fn discount_account_id(mut self, value: Uuid) -> Self {
        self.discount_account_id = Some(value);
        self
    }

    /// Set the discount_tax_basis field (default: `DiscountTaxBasis::default()`)
    pub fn discount_tax_basis(mut self, value: DiscountTaxBasis) -> Self {
        self.discount_tax_basis = Some(value);
        self
    }

    /// Build the PaymentTerm entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<PaymentTerm, String> {
        let name = self.name.ok_or_else(|| "name is required".to_string())?;

        Ok(PaymentTerm {
            id: Uuid::new_v4(),
            company_id: self.company_id,
            name,
            note: self.note,
            sequence: self.sequence.unwrap_or(10),
            status: self.status.unwrap_or_default(),
            early_discount: self.early_discount.unwrap_or(false),
            discount_percent: self.discount_percent.unwrap_or(Decimal::from(0)),
            discount_days: self.discount_days.unwrap_or(0),
            discount_account_id: self.discount_account_id,
            discount_tax_basis: self.discount_tax_basis.unwrap_or_default(),
            metadata: AuditMetadata::default(),
        })
    }
}

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::AuditMetadata;
use super::PaymentTermAnchor;
use super::PaymentTermDelayType;
use super::PaymentTermLineValue;

/// Strongly-typed ID for PaymentTermLine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaymentTermLineId(pub Uuid);

impl PaymentTermLineId {
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

impl std::fmt::Display for PaymentTermLineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PaymentTermLineId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for PaymentTermLineId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<PaymentTermLineId> for Uuid {
    fn from(id: PaymentTermLineId) -> Self {
        id.0
    }
}

impl AsRef<Uuid> for PaymentTermLineId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::ops::Deref for PaymentTermLineId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentTermLine {
    pub id: Uuid,
    pub term_id: Uuid,
    pub company_id: Option<Uuid>,
    pub value: PaymentTermLineValue,
    pub value_amount: Decimal,
    pub nb_days: i32,
    pub day_of_month: Option<i32>,
    pub delay_type: PaymentTermDelayType,
    pub anchor: PaymentTermAnchor,
    pub sequence: i32,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl PaymentTermLine {
    /// Create a builder for PaymentTermLine
    pub fn builder() -> PaymentTermLineBuilder {
        <PaymentTermLineBuilder as Default>::default()
    }

    /// Create a new PaymentTermLine with required fields
    pub fn new(
        term_id: Uuid,
        value: PaymentTermLineValue,
        value_amount: Decimal,
        nb_days: i32,
        delay_type: PaymentTermDelayType,
        anchor: PaymentTermAnchor,
        sequence: i32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            term_id,
            company_id: None,
            value,
            value_amount,
            nb_days,
            day_of_month: None,
            delay_type,
            anchor,
            sequence,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> PaymentTermLineId {
        PaymentTermLineId(self.id)
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

    /// Set the company_id field (chainable)
    pub fn with_company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the day_of_month field (chainable)
    pub fn with_day_of_month(mut self, value: i32) -> Self {
        self.day_of_month = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "term_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.term_id = v;
                    }
                }
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.company_id = v;
                    }
                }
                "value" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.value = v;
                    }
                }
                "value_amount" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.value_amount = v;
                    }
                }
                "nb_days" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.nb_days = v;
                    }
                }
                "day_of_month" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.day_of_month = v;
                    }
                }
                "delay_type" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.delay_type = v;
                    }
                }
                "anchor" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.anchor = v;
                    }
                }
                "sequence" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        self.sequence = v;
                    }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for PaymentTermLine {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "PaymentTermLine"
    }
}

impl backbone_core::PersistentEntity for PaymentTermLine {
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

impl backbone_orm::EntityRepoMeta for PaymentTermLine {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("term_id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("value".to_string(), "payment_term_line_value".to_string());
        m.insert(
            "delay_type".to_string(),
            "payment_term_delay_type".to_string(),
        );
        m.insert("anchor".to_string(), "payment_term_anchor".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
    fn relations() -> &'static [(&'static str, &'static str, &'static str)] {
        &[("term", "payment_terms", "termId")]
    }
}

/// Builder for PaymentTermLine entity
///
/// Provides a fluent API for constructing PaymentTermLine instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct PaymentTermLineBuilder {
    term_id: Option<Uuid>,
    company_id: Option<Uuid>,
    value: Option<PaymentTermLineValue>,
    value_amount: Option<Decimal>,
    nb_days: Option<i32>,
    day_of_month: Option<i32>,
    delay_type: Option<PaymentTermDelayType>,
    anchor: Option<PaymentTermAnchor>,
    sequence: Option<i32>,
}

impl PaymentTermLineBuilder {
    /// Set the term_id field (required)
    pub fn term_id(mut self, value: Uuid) -> Self {
        self.term_id = Some(value);
        self
    }

    /// Set the company_id field (optional)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the value field (default: `PaymentTermLineValue::default()`)
    pub fn value(mut self, value: PaymentTermLineValue) -> Self {
        self.value = Some(value);
        self
    }

    /// Set the value_amount field (default: `Decimal::from(0)`)
    pub fn value_amount(mut self, value: Decimal) -> Self {
        self.value_amount = Some(value);
        self
    }

    /// Set the nb_days field (required)
    pub fn nb_days(mut self, value: i32) -> Self {
        self.nb_days = Some(value);
        self
    }

    /// Set the day_of_month field (optional)
    pub fn day_of_month(mut self, value: i32) -> Self {
        self.day_of_month = Some(value);
        self
    }

    /// Set the delay_type field (default: `PaymentTermDelayType::default()`)
    pub fn delay_type(mut self, value: PaymentTermDelayType) -> Self {
        self.delay_type = Some(value);
        self
    }

    /// Set the anchor field (default: `PaymentTermAnchor::default()`)
    pub fn anchor(mut self, value: PaymentTermAnchor) -> Self {
        self.anchor = Some(value);
        self
    }

    /// Set the sequence field (default: `10`)
    pub fn sequence(mut self, value: i32) -> Self {
        self.sequence = Some(value);
        self
    }

    /// Build the PaymentTermLine entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<PaymentTermLine, String> {
        let term_id = self
            .term_id
            .ok_or_else(|| "term_id is required".to_string())?;
        let nb_days = self
            .nb_days
            .ok_or_else(|| "nb_days is required".to_string())?;

        Ok(PaymentTermLine {
            id: Uuid::new_v4(),
            term_id,
            company_id: self.company_id,
            value: self.value.unwrap_or_default(),
            value_amount: self.value_amount.unwrap_or(Decimal::from(0)),
            nb_days,
            day_of_month: self.day_of_month,
            delay_type: self.delay_type.unwrap_or_default(),
            anchor: self.anchor.unwrap_or_default(),
            sequence: self.sequence.unwrap_or(10),
            metadata: AuditMetadata::default(),
        })
    }
}

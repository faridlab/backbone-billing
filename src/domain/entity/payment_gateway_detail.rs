use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use super::AuditMetadata;

/// Strongly-typed ID for PaymentGatewayDetail
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaymentGatewayDetailId(pub Uuid);

impl PaymentGatewayDetailId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for PaymentGatewayDetailId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PaymentGatewayDetailId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for PaymentGatewayDetailId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<PaymentGatewayDetailId> for Uuid {
    fn from(id: PaymentGatewayDetailId) -> Self { id.0 }
}

impl AsRef<Uuid> for PaymentGatewayDetailId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for PaymentGatewayDetailId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PaymentGatewayDetail {
    pub id: Uuid,
    pub payment_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_code_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_account_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_account_bank: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_received_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl PaymentGatewayDetail {
    /// Create a builder for PaymentGatewayDetail
    pub fn builder() -> PaymentGatewayDetailBuilder {
        PaymentGatewayDetailBuilder::default()
    }

    /// Create a new PaymentGatewayDetail with required fields
    pub fn new(payment_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            payment_id,
            external_id: None,
            invoice_url: None,
            checkout_url: None,
            qr_code_url: None,
            virtual_account_number: None,
            virtual_account_bank: None,
            webhook_payload: None,
            webhook_received_at: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> PaymentGatewayDetailId {
        PaymentGatewayDetailId(self.id)
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

    /// Set the external_id field (chainable)
    pub fn with_external_id(mut self, value: String) -> Self {
        self.external_id = Some(value);
        self
    }

    /// Set the invoice_url field (chainable)
    pub fn with_invoice_url(mut self, value: String) -> Self {
        self.invoice_url = Some(value);
        self
    }

    /// Set the checkout_url field (chainable)
    pub fn with_checkout_url(mut self, value: String) -> Self {
        self.checkout_url = Some(value);
        self
    }

    /// Set the qr_code_url field (chainable)
    pub fn with_qr_code_url(mut self, value: String) -> Self {
        self.qr_code_url = Some(value);
        self
    }

    /// Set the virtual_account_number field (chainable)
    pub fn with_virtual_account_number(mut self, value: String) -> Self {
        self.virtual_account_number = Some(value);
        self
    }

    /// Set the virtual_account_bank field (chainable)
    pub fn with_virtual_account_bank(mut self, value: String) -> Self {
        self.virtual_account_bank = Some(value);
        self
    }

    /// Set the webhook_payload field (chainable)
    pub fn with_webhook_payload(mut self, value: serde_json::Value) -> Self {
        self.webhook_payload = Some(value);
        self
    }

    /// Set the webhook_received_at field (chainable)
    pub fn with_webhook_received_at(mut self, value: DateTime<Utc>) -> Self {
        self.webhook_received_at = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "payment_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payment_id = v; }
                }
                "external_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.external_id = v; }
                }
                "invoice_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoice_url = v; }
                }
                "checkout_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.checkout_url = v; }
                }
                "qr_code_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.qr_code_url = v; }
                }
                "virtual_account_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.virtual_account_number = v; }
                }
                "virtual_account_bank" => {
                    if let Ok(v) = serde_json::from_value(value) { self.virtual_account_bank = v; }
                }
                "webhook_payload" => {
                    if let Ok(v) = serde_json::from_value(value) { self.webhook_payload = v; }
                }
                "webhook_received_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.webhook_received_at = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for PaymentGatewayDetail {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "PaymentGatewayDetail"
    }
}

impl backbone_core::PersistentEntity for PaymentGatewayDetail {
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

impl backbone_orm::EntityRepoMeta for PaymentGatewayDetail {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("payment_id".to_string(), "uuid".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
}

/// Builder for PaymentGatewayDetail entity
///
/// Provides a fluent API for constructing PaymentGatewayDetail instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct PaymentGatewayDetailBuilder {
    payment_id: Option<Uuid>,
    external_id: Option<String>,
    invoice_url: Option<String>,
    checkout_url: Option<String>,
    qr_code_url: Option<String>,
    virtual_account_number: Option<String>,
    virtual_account_bank: Option<String>,
    webhook_payload: Option<serde_json::Value>,
    webhook_received_at: Option<DateTime<Utc>>,
}

impl PaymentGatewayDetailBuilder {
    /// Set the payment_id field (required)
    pub fn payment_id(mut self, value: Uuid) -> Self {
        self.payment_id = Some(value);
        self
    }

    /// Set the external_id field (optional)
    pub fn external_id(mut self, value: String) -> Self {
        self.external_id = Some(value);
        self
    }

    /// Set the invoice_url field (optional)
    pub fn invoice_url(mut self, value: String) -> Self {
        self.invoice_url = Some(value);
        self
    }

    /// Set the checkout_url field (optional)
    pub fn checkout_url(mut self, value: String) -> Self {
        self.checkout_url = Some(value);
        self
    }

    /// Set the qr_code_url field (optional)
    pub fn qr_code_url(mut self, value: String) -> Self {
        self.qr_code_url = Some(value);
        self
    }

    /// Set the virtual_account_number field (optional)
    pub fn virtual_account_number(mut self, value: String) -> Self {
        self.virtual_account_number = Some(value);
        self
    }

    /// Set the virtual_account_bank field (optional)
    pub fn virtual_account_bank(mut self, value: String) -> Self {
        self.virtual_account_bank = Some(value);
        self
    }

    /// Set the webhook_payload field (optional)
    pub fn webhook_payload(mut self, value: serde_json::Value) -> Self {
        self.webhook_payload = Some(value);
        self
    }

    /// Set the webhook_received_at field (optional)
    pub fn webhook_received_at(mut self, value: DateTime<Utc>) -> Self {
        self.webhook_received_at = Some(value);
        self
    }

    /// Build the PaymentGatewayDetail entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<PaymentGatewayDetail, String> {
        let payment_id = self.payment_id.ok_or_else(|| "payment_id is required".to_string())?;

        Ok(PaymentGatewayDetail {
            id: Uuid::new_v4(),
            payment_id,
            external_id: self.external_id,
            invoice_url: self.invoice_url,
            checkout_url: self.checkout_url,
            qr_code_url: self.qr_code_url,
            virtual_account_number: self.virtual_account_number,
            virtual_account_bank: self.virtual_account_bank,
            webhook_payload: self.webhook_payload,
            webhook_received_at: self.webhook_received_at,
            metadata: AuditMetadata::default(),
        })
    }
}

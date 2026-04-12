use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::PayableType;
use super::PayerType;
use super::PayeeType;
use super::PaymentGateway;
use super::PaymentChannel;
use super::PaymentTransactionStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Payment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaymentId(pub Uuid);

impl PaymentId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for PaymentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PaymentId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for PaymentId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<PaymentId> for Uuid {
    fn from(id: PaymentId) -> Self { id.0 }
}

impl AsRef<Uuid> for PaymentId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for PaymentId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub payment_number: String,
    pub payable_type: PayableType,
    pub payable_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<Uuid>,
    pub payer_type: PayerType,
    pub payer_id: Uuid,
    pub payee_type: PayeeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payee_id: Option<Uuid>,
    pub currency: String,
    pub amount: Decimal,
    pub net_amount: Decimal,
    pub gateway: PaymentGateway,
    pub channel: PaymentChannel,
    pub status: PaymentTransactionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub data: serde_json::Value,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Payment {
    /// Create a builder for Payment
    pub fn builder() -> PaymentBuilder {
        PaymentBuilder::default()
    }

    /// Create a new Payment with required fields
    pub fn new(payment_number: String, payable_type: PayableType, payable_id: Uuid, payer_type: PayerType, payer_id: Uuid, payee_type: PayeeType, currency: String, amount: Decimal, net_amount: Decimal, gateway: PaymentGateway, channel: PaymentChannel, status: PaymentTransactionStatus, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            payment_number,
            payable_type,
            payable_id,
            order_id: None,
            payer_type,
            payer_id,
            payee_type,
            payee_id: None,
            currency,
            amount,
            net_amount,
            gateway,
            channel,
            status,
            expires_at: None,
            paid_at: None,
            failed_at: None,
            cancelled_at: None,
            failure_code: None,
            failure_message: None,
            settlement_id: None,
            description: None,
            notes: None,
            data,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> PaymentId {
        PaymentId(self.id)
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
    pub fn status(&self) -> &PaymentTransactionStatus {
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

    /// Set the payee_id field (chainable)
    pub fn with_payee_id(mut self, value: Uuid) -> Self {
        self.payee_id = Some(value);
        self
    }

    /// Set the expires_at field (chainable)
    pub fn with_expires_at(mut self, value: DateTime<Utc>) -> Self {
        self.expires_at = Some(value);
        self
    }

    /// Set the paid_at field (chainable)
    pub fn with_paid_at(mut self, value: DateTime<Utc>) -> Self {
        self.paid_at = Some(value);
        self
    }

    /// Set the failed_at field (chainable)
    pub fn with_failed_at(mut self, value: DateTime<Utc>) -> Self {
        self.failed_at = Some(value);
        self
    }

    /// Set the cancelled_at field (chainable)
    pub fn with_cancelled_at(mut self, value: DateTime<Utc>) -> Self {
        self.cancelled_at = Some(value);
        self
    }

    /// Set the failure_code field (chainable)
    pub fn with_failure_code(mut self, value: String) -> Self {
        self.failure_code = Some(value);
        self
    }

    /// Set the failure_message field (chainable)
    pub fn with_failure_message(mut self, value: String) -> Self {
        self.failure_message = Some(value);
        self
    }

    /// Set the settlement_id field (chainable)
    pub fn with_settlement_id(mut self, value: Uuid) -> Self {
        self.settlement_id = Some(value);
        self
    }

    /// Set the description field (chainable)
    pub fn with_description(mut self, value: String) -> Self {
        self.description = Some(value);
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
                "payment_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payment_number = v; }
                }
                "payable_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payable_type = v; }
                }
                "payable_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payable_id = v; }
                }
                "order_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.order_id = v; }
                }
                "payer_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payer_type = v; }
                }
                "payer_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payer_id = v; }
                }
                "payee_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payee_type = v; }
                }
                "payee_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payee_id = v; }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.currency = v; }
                }
                "amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.amount = v; }
                }
                "net_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.net_amount = v; }
                }
                "gateway" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gateway = v; }
                }
                "channel" => {
                    if let Ok(v) = serde_json::from_value(value) { self.channel = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "expires_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.expires_at = v; }
                }
                "paid_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.paid_at = v; }
                }
                "failed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failed_at = v; }
                }
                "cancelled_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cancelled_at = v; }
                }
                "failure_code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failure_code = v; }
                }
                "failure_message" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failure_message = v; }
                }
                "settlement_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.settlement_id = v; }
                }
                "description" => {
                    if let Ok(v) = serde_json::from_value(value) { self.description = v; }
                }
                "notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.notes = v; }
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

impl super::Entity for Payment {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Payment"
    }
}

impl backbone_core::PersistentEntity for Payment {
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

impl backbone_orm::EntityRepoMeta for Payment {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("payable_id".to_string(), "uuid".to_string());
        m.insert("order_id".to_string(), "uuid".to_string());
        m.insert("payer_id".to_string(), "uuid".to_string());
        m.insert("payee_id".to_string(), "uuid".to_string());
        m.insert("settlement_id".to_string(), "uuid".to_string());
        m.insert("payable_type".to_string(), "payable_type".to_string());
        m.insert("payer_type".to_string(), "payer_type".to_string());
        m.insert("payee_type".to_string(), "payee_type".to_string());
        m.insert("gateway".to_string(), "payment_gateway".to_string());
        m.insert("channel".to_string(), "payment_channel".to_string());
        m.insert("status".to_string(), "payment_transaction_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["payment_number", "currency"]
    }
}

/// Builder for Payment entity
///
/// Provides a fluent API for constructing Payment instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct PaymentBuilder {
    payment_number: Option<String>,
    payable_type: Option<PayableType>,
    payable_id: Option<Uuid>,
    order_id: Option<Uuid>,
    payer_type: Option<PayerType>,
    payer_id: Option<Uuid>,
    payee_type: Option<PayeeType>,
    payee_id: Option<Uuid>,
    currency: Option<String>,
    amount: Option<Decimal>,
    net_amount: Option<Decimal>,
    gateway: Option<PaymentGateway>,
    channel: Option<PaymentChannel>,
    status: Option<PaymentTransactionStatus>,
    expires_at: Option<DateTime<Utc>>,
    paid_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    settlement_id: Option<Uuid>,
    description: Option<String>,
    notes: Option<String>,
    data: Option<serde_json::Value>,
}

impl PaymentBuilder {
    /// Set the payment_number field (required)
    pub fn payment_number(mut self, value: String) -> Self {
        self.payment_number = Some(value);
        self
    }

    /// Set the payable_type field (required)
    pub fn payable_type(mut self, value: PayableType) -> Self {
        self.payable_type = Some(value);
        self
    }

    /// Set the payable_id field (required)
    pub fn payable_id(mut self, value: Uuid) -> Self {
        self.payable_id = Some(value);
        self
    }

    /// Set the order_id field (optional)
    pub fn order_id(mut self, value: Uuid) -> Self {
        self.order_id = Some(value);
        self
    }

    /// Set the payer_type field (required)
    pub fn payer_type(mut self, value: PayerType) -> Self {
        self.payer_type = Some(value);
        self
    }

    /// Set the payer_id field (required)
    pub fn payer_id(mut self, value: Uuid) -> Self {
        self.payer_id = Some(value);
        self
    }

    /// Set the payee_type field (required)
    pub fn payee_type(mut self, value: PayeeType) -> Self {
        self.payee_type = Some(value);
        self
    }

    /// Set the payee_id field (optional)
    pub fn payee_id(mut self, value: Uuid) -> Self {
        self.payee_id = Some(value);
        self
    }

    /// Set the currency field (default: `"IDR".to_string()`)
    pub fn currency(mut self, value: String) -> Self {
        self.currency = Some(value);
        self
    }

    /// Set the amount field (required)
    pub fn amount(mut self, value: Decimal) -> Self {
        self.amount = Some(value);
        self
    }

    /// Set the net_amount field (required)
    pub fn net_amount(mut self, value: Decimal) -> Self {
        self.net_amount = Some(value);
        self
    }

    /// Set the gateway field (default: `PaymentGateway::default()`)
    pub fn gateway(mut self, value: PaymentGateway) -> Self {
        self.gateway = Some(value);
        self
    }

    /// Set the channel field (required)
    pub fn channel(mut self, value: PaymentChannel) -> Self {
        self.channel = Some(value);
        self
    }

    /// Set the status field (default: `PaymentTransactionStatus::default()`)
    pub fn status(mut self, value: PaymentTransactionStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the expires_at field (optional)
    pub fn expires_at(mut self, value: DateTime<Utc>) -> Self {
        self.expires_at = Some(value);
        self
    }

    /// Set the paid_at field (optional)
    pub fn paid_at(mut self, value: DateTime<Utc>) -> Self {
        self.paid_at = Some(value);
        self
    }

    /// Set the failed_at field (optional)
    pub fn failed_at(mut self, value: DateTime<Utc>) -> Self {
        self.failed_at = Some(value);
        self
    }

    /// Set the cancelled_at field (optional)
    pub fn cancelled_at(mut self, value: DateTime<Utc>) -> Self {
        self.cancelled_at = Some(value);
        self
    }

    /// Set the failure_code field (optional)
    pub fn failure_code(mut self, value: String) -> Self {
        self.failure_code = Some(value);
        self
    }

    /// Set the failure_message field (optional)
    pub fn failure_message(mut self, value: String) -> Self {
        self.failure_message = Some(value);
        self
    }

    /// Set the settlement_id field (optional)
    pub fn settlement_id(mut self, value: Uuid) -> Self {
        self.settlement_id = Some(value);
        self
    }

    /// Set the description field (optional)
    pub fn description(mut self, value: String) -> Self {
        self.description = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Set the data field (default: `serde_json::json!({"payer_name":null,"payer_email":null,"payer_phone":null,"is_settled":false,"settled_at":null,"is_refunded":false,"refunded_amount":0})`)
    pub fn data(mut self, value: serde_json::Value) -> Self {
        self.data = Some(value);
        self
    }

    /// Build the Payment entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Payment, String> {
        let payment_number = self.payment_number.ok_or_else(|| "payment_number is required".to_string())?;
        let payable_type = self.payable_type.ok_or_else(|| "payable_type is required".to_string())?;
        let payable_id = self.payable_id.ok_or_else(|| "payable_id is required".to_string())?;
        let payer_type = self.payer_type.ok_or_else(|| "payer_type is required".to_string())?;
        let payer_id = self.payer_id.ok_or_else(|| "payer_id is required".to_string())?;
        let payee_type = self.payee_type.ok_or_else(|| "payee_type is required".to_string())?;
        let amount = self.amount.ok_or_else(|| "amount is required".to_string())?;
        let net_amount = self.net_amount.ok_or_else(|| "net_amount is required".to_string())?;
        let channel = self.channel.ok_or_else(|| "channel is required".to_string())?;

        Ok(Payment {
            id: Uuid::new_v4(),
            payment_number,
            payable_type,
            payable_id,
            order_id: self.order_id,
            payer_type,
            payer_id,
            payee_type,
            payee_id: self.payee_id,
            currency: self.currency.unwrap_or("IDR".to_string()),
            amount,
            net_amount,
            gateway: self.gateway.unwrap_or(PaymentGateway::default()),
            channel,
            status: self.status.unwrap_or(PaymentTransactionStatus::default()),
            expires_at: self.expires_at,
            paid_at: self.paid_at,
            failed_at: self.failed_at,
            cancelled_at: self.cancelled_at,
            failure_code: self.failure_code,
            failure_message: self.failure_message,
            settlement_id: self.settlement_id,
            description: self.description,
            notes: self.notes,
            data: self.data.unwrap_or(serde_json::json!({"payer_name":null,"payer_email":null,"payer_phone":null,"is_settled":false,"settled_at":null,"is_refunded":false,"refunded_amount":0})),
            metadata: AuditMetadata::default(),
        })
    }
}

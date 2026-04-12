use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::SettlementFrequency;
use super::SettlementStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Settlement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SettlementId(pub Uuid);

impl SettlementId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for SettlementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SettlementId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for SettlementId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<SettlementId> for Uuid {
    fn from(id: SettlementId) -> Self { id.0 }
}

impl AsRef<Uuid> for SettlementId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for SettlementId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Settlement {
    pub id: Uuid,
    pub settlement_number: String,
    pub provider_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub frequency: SettlementFrequency,
    pub currency: String,
    pub gross_amount: Decimal,
    pub platform_commission: Decimal,
    pub platform_commission_rate: Decimal,
    pub transaction_fees: Decimal,
    pub payment_gateway_fees: Decimal,
    pub refunds_amount: Decimal,
    pub adjustments_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_notes: Option<String>,
    pub withholding_tax: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withholding_tax_rate: Option<Decimal>,
    pub net_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account_id: Option<Uuid>,
    pub status: SettlementStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_receipt_url: Option<String>,
    pub is_on_hold: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub held_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub held_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Uuid>,
    pub invoice_generated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub data: serde_json::Value,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Settlement {
    /// Create a builder for Settlement
    pub fn builder() -> SettlementBuilder {
        SettlementBuilder::default()
    }

    /// Create a new Settlement with required fields
    pub fn new(settlement_number: String, provider_id: Uuid, period_start: DateTime<Utc>, period_end: DateTime<Utc>, frequency: SettlementFrequency, currency: String, gross_amount: Decimal, platform_commission: Decimal, platform_commission_rate: Decimal, transaction_fees: Decimal, payment_gateway_fees: Decimal, refunds_amount: Decimal, adjustments_amount: Decimal, withholding_tax: Decimal, net_amount: Decimal, status: SettlementStatus, is_on_hold: bool, invoice_generated: bool, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            settlement_number,
            provider_id,
            period_start,
            period_end,
            frequency,
            currency,
            gross_amount,
            platform_commission,
            platform_commission_rate,
            transaction_fees,
            payment_gateway_fees,
            refunds_amount,
            adjustments_amount,
            adjustment_notes: None,
            withholding_tax,
            withholding_tax_rate: None,
            net_amount,
            bank_account_id: None,
            status,
            scheduled_at: None,
            processing_started_at: None,
            completed_at: None,
            failed_at: None,
            failure_reason: None,
            transfer_reference: None,
            transfer_receipt_url: None,
            is_on_hold,
            hold_reason: None,
            held_by: None,
            held_at: None,
            released_by: None,
            released_at: None,
            invoice_id: None,
            invoice_generated,
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
    pub fn typed_id(&self) -> SettlementId {
        SettlementId(self.id)
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
    pub fn status(&self) -> &SettlementStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the adjustment_notes field (chainable)
    pub fn with_adjustment_notes(mut self, value: String) -> Self {
        self.adjustment_notes = Some(value);
        self
    }

    /// Set the withholding_tax_rate field (chainable)
    pub fn with_withholding_tax_rate(mut self, value: Decimal) -> Self {
        self.withholding_tax_rate = Some(value);
        self
    }

    /// Set the bank_account_id field (chainable)
    pub fn with_bank_account_id(mut self, value: Uuid) -> Self {
        self.bank_account_id = Some(value);
        self
    }

    /// Set the scheduled_at field (chainable)
    pub fn with_scheduled_at(mut self, value: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(value);
        self
    }

    /// Set the processing_started_at field (chainable)
    pub fn with_processing_started_at(mut self, value: DateTime<Utc>) -> Self {
        self.processing_started_at = Some(value);
        self
    }

    /// Set the completed_at field (chainable)
    pub fn with_completed_at(mut self, value: DateTime<Utc>) -> Self {
        self.completed_at = Some(value);
        self
    }

    /// Set the failed_at field (chainable)
    pub fn with_failed_at(mut self, value: DateTime<Utc>) -> Self {
        self.failed_at = Some(value);
        self
    }

    /// Set the failure_reason field (chainable)
    pub fn with_failure_reason(mut self, value: String) -> Self {
        self.failure_reason = Some(value);
        self
    }

    /// Set the transfer_reference field (chainable)
    pub fn with_transfer_reference(mut self, value: String) -> Self {
        self.transfer_reference = Some(value);
        self
    }

    /// Set the transfer_receipt_url field (chainable)
    pub fn with_transfer_receipt_url(mut self, value: String) -> Self {
        self.transfer_receipt_url = Some(value);
        self
    }

    /// Set the hold_reason field (chainable)
    pub fn with_hold_reason(mut self, value: String) -> Self {
        self.hold_reason = Some(value);
        self
    }

    /// Set the held_by field (chainable)
    pub fn with_held_by(mut self, value: Uuid) -> Self {
        self.held_by = Some(value);
        self
    }

    /// Set the held_at field (chainable)
    pub fn with_held_at(mut self, value: DateTime<Utc>) -> Self {
        self.held_at = Some(value);
        self
    }

    /// Set the released_by field (chainable)
    pub fn with_released_by(mut self, value: Uuid) -> Self {
        self.released_by = Some(value);
        self
    }

    /// Set the released_at field (chainable)
    pub fn with_released_at(mut self, value: DateTime<Utc>) -> Self {
        self.released_at = Some(value);
        self
    }

    /// Set the invoice_id field (chainable)
    pub fn with_invoice_id(mut self, value: Uuid) -> Self {
        self.invoice_id = Some(value);
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
                "settlement_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.settlement_number = v; }
                }
                "provider_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.provider_id = v; }
                }
                "period_start" => {
                    if let Ok(v) = serde_json::from_value(value) { self.period_start = v; }
                }
                "period_end" => {
                    if let Ok(v) = serde_json::from_value(value) { self.period_end = v; }
                }
                "frequency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.frequency = v; }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.currency = v; }
                }
                "gross_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gross_amount = v; }
                }
                "platform_commission" => {
                    if let Ok(v) = serde_json::from_value(value) { self.platform_commission = v; }
                }
                "platform_commission_rate" => {
                    if let Ok(v) = serde_json::from_value(value) { self.platform_commission_rate = v; }
                }
                "transaction_fees" => {
                    if let Ok(v) = serde_json::from_value(value) { self.transaction_fees = v; }
                }
                "payment_gateway_fees" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payment_gateway_fees = v; }
                }
                "refunds_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.refunds_amount = v; }
                }
                "adjustments_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.adjustments_amount = v; }
                }
                "adjustment_notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.adjustment_notes = v; }
                }
                "withholding_tax" => {
                    if let Ok(v) = serde_json::from_value(value) { self.withholding_tax = v; }
                }
                "withholding_tax_rate" => {
                    if let Ok(v) = serde_json::from_value(value) { self.withholding_tax_rate = v; }
                }
                "net_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.net_amount = v; }
                }
                "bank_account_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.bank_account_id = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "scheduled_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.scheduled_at = v; }
                }
                "processing_started_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.processing_started_at = v; }
                }
                "completed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.completed_at = v; }
                }
                "failed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failed_at = v; }
                }
                "failure_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.failure_reason = v; }
                }
                "transfer_reference" => {
                    if let Ok(v) = serde_json::from_value(value) { self.transfer_reference = v; }
                }
                "transfer_receipt_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.transfer_receipt_url = v; }
                }
                "is_on_hold" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_on_hold = v; }
                }
                "hold_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.hold_reason = v; }
                }
                "held_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.held_by = v; }
                }
                "held_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.held_at = v; }
                }
                "released_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.released_by = v; }
                }
                "released_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.released_at = v; }
                }
                "invoice_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoice_id = v; }
                }
                "invoice_generated" => {
                    if let Ok(v) = serde_json::from_value(value) { self.invoice_generated = v; }
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

impl super::Entity for Settlement {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Settlement"
    }
}

impl backbone_core::PersistentEntity for Settlement {
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

impl backbone_orm::EntityRepoMeta for Settlement {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("provider_id".to_string(), "uuid".to_string());
        m.insert("bank_account_id".to_string(), "uuid".to_string());
        m.insert("invoice_id".to_string(), "uuid".to_string());
        m.insert("frequency".to_string(), "settlement_frequency".to_string());
        m.insert("status".to_string(), "settlement_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["settlement_number", "currency"]
    }
}

/// Builder for Settlement entity
///
/// Provides a fluent API for constructing Settlement instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct SettlementBuilder {
    settlement_number: Option<String>,
    provider_id: Option<Uuid>,
    period_start: Option<DateTime<Utc>>,
    period_end: Option<DateTime<Utc>>,
    frequency: Option<SettlementFrequency>,
    currency: Option<String>,
    gross_amount: Option<Decimal>,
    platform_commission: Option<Decimal>,
    platform_commission_rate: Option<Decimal>,
    transaction_fees: Option<Decimal>,
    payment_gateway_fees: Option<Decimal>,
    refunds_amount: Option<Decimal>,
    adjustments_amount: Option<Decimal>,
    adjustment_notes: Option<String>,
    withholding_tax: Option<Decimal>,
    withholding_tax_rate: Option<Decimal>,
    net_amount: Option<Decimal>,
    bank_account_id: Option<Uuid>,
    status: Option<SettlementStatus>,
    scheduled_at: Option<DateTime<Utc>>,
    processing_started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    transfer_reference: Option<String>,
    transfer_receipt_url: Option<String>,
    is_on_hold: Option<bool>,
    hold_reason: Option<String>,
    held_by: Option<Uuid>,
    held_at: Option<DateTime<Utc>>,
    released_by: Option<Uuid>,
    released_at: Option<DateTime<Utc>>,
    invoice_id: Option<Uuid>,
    invoice_generated: Option<bool>,
    notes: Option<String>,
    data: Option<serde_json::Value>,
}

impl SettlementBuilder {
    /// Set the settlement_number field (required)
    pub fn settlement_number(mut self, value: String) -> Self {
        self.settlement_number = Some(value);
        self
    }

    /// Set the provider_id field (required)
    pub fn provider_id(mut self, value: Uuid) -> Self {
        self.provider_id = Some(value);
        self
    }

    /// Set the period_start field (required)
    pub fn period_start(mut self, value: DateTime<Utc>) -> Self {
        self.period_start = Some(value);
        self
    }

    /// Set the period_end field (required)
    pub fn period_end(mut self, value: DateTime<Utc>) -> Self {
        self.period_end = Some(value);
        self
    }

    /// Set the frequency field (default: `SettlementFrequency::default()`)
    pub fn frequency(mut self, value: SettlementFrequency) -> Self {
        self.frequency = Some(value);
        self
    }

    /// Set the currency field (default: `"IDR".to_string()`)
    pub fn currency(mut self, value: String) -> Self {
        self.currency = Some(value);
        self
    }

    /// Set the gross_amount field (required)
    pub fn gross_amount(mut self, value: Decimal) -> Self {
        self.gross_amount = Some(value);
        self
    }

    /// Set the platform_commission field (default: `Decimal::from(0)`)
    pub fn platform_commission(mut self, value: Decimal) -> Self {
        self.platform_commission = Some(value);
        self
    }

    /// Set the platform_commission_rate field (required)
    pub fn platform_commission_rate(mut self, value: Decimal) -> Self {
        self.platform_commission_rate = Some(value);
        self
    }

    /// Set the transaction_fees field (default: `Decimal::from(0)`)
    pub fn transaction_fees(mut self, value: Decimal) -> Self {
        self.transaction_fees = Some(value);
        self
    }

    /// Set the payment_gateway_fees field (default: `Decimal::from(0)`)
    pub fn payment_gateway_fees(mut self, value: Decimal) -> Self {
        self.payment_gateway_fees = Some(value);
        self
    }

    /// Set the refunds_amount field (default: `Decimal::from(0)`)
    pub fn refunds_amount(mut self, value: Decimal) -> Self {
        self.refunds_amount = Some(value);
        self
    }

    /// Set the adjustments_amount field (default: `Decimal::from(0)`)
    pub fn adjustments_amount(mut self, value: Decimal) -> Self {
        self.adjustments_amount = Some(value);
        self
    }

    /// Set the adjustment_notes field (optional)
    pub fn adjustment_notes(mut self, value: String) -> Self {
        self.adjustment_notes = Some(value);
        self
    }

    /// Set the withholding_tax field (default: `Decimal::from(0)`)
    pub fn withholding_tax(mut self, value: Decimal) -> Self {
        self.withholding_tax = Some(value);
        self
    }

    /// Set the withholding_tax_rate field (optional)
    pub fn withholding_tax_rate(mut self, value: Decimal) -> Self {
        self.withholding_tax_rate = Some(value);
        self
    }

    /// Set the net_amount field (required)
    pub fn net_amount(mut self, value: Decimal) -> Self {
        self.net_amount = Some(value);
        self
    }

    /// Set the bank_account_id field (optional)
    pub fn bank_account_id(mut self, value: Uuid) -> Self {
        self.bank_account_id = Some(value);
        self
    }

    /// Set the status field (default: `SettlementStatus::default()`)
    pub fn status(mut self, value: SettlementStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the scheduled_at field (optional)
    pub fn scheduled_at(mut self, value: DateTime<Utc>) -> Self {
        self.scheduled_at = Some(value);
        self
    }

    /// Set the processing_started_at field (optional)
    pub fn processing_started_at(mut self, value: DateTime<Utc>) -> Self {
        self.processing_started_at = Some(value);
        self
    }

    /// Set the completed_at field (optional)
    pub fn completed_at(mut self, value: DateTime<Utc>) -> Self {
        self.completed_at = Some(value);
        self
    }

    /// Set the failed_at field (optional)
    pub fn failed_at(mut self, value: DateTime<Utc>) -> Self {
        self.failed_at = Some(value);
        self
    }

    /// Set the failure_reason field (optional)
    pub fn failure_reason(mut self, value: String) -> Self {
        self.failure_reason = Some(value);
        self
    }

    /// Set the transfer_reference field (optional)
    pub fn transfer_reference(mut self, value: String) -> Self {
        self.transfer_reference = Some(value);
        self
    }

    /// Set the transfer_receipt_url field (optional)
    pub fn transfer_receipt_url(mut self, value: String) -> Self {
        self.transfer_receipt_url = Some(value);
        self
    }

    /// Set the is_on_hold field (default: `false`)
    pub fn is_on_hold(mut self, value: bool) -> Self {
        self.is_on_hold = Some(value);
        self
    }

    /// Set the hold_reason field (optional)
    pub fn hold_reason(mut self, value: String) -> Self {
        self.hold_reason = Some(value);
        self
    }

    /// Set the held_by field (optional)
    pub fn held_by(mut self, value: Uuid) -> Self {
        self.held_by = Some(value);
        self
    }

    /// Set the held_at field (optional)
    pub fn held_at(mut self, value: DateTime<Utc>) -> Self {
        self.held_at = Some(value);
        self
    }

    /// Set the released_by field (optional)
    pub fn released_by(mut self, value: Uuid) -> Self {
        self.released_by = Some(value);
        self
    }

    /// Set the released_at field (optional)
    pub fn released_at(mut self, value: DateTime<Utc>) -> Self {
        self.released_at = Some(value);
        self
    }

    /// Set the invoice_id field (optional)
    pub fn invoice_id(mut self, value: Uuid) -> Self {
        self.invoice_id = Some(value);
        self
    }

    /// Set the invoice_generated field (default: `false`)
    pub fn invoice_generated(mut self, value: bool) -> Self {
        self.invoice_generated = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Set the data field (default: `serde_json::json!({"total_orders":0,"total_payments":0,"bank_name":null,"account_number":null,"account_name":null})`)
    pub fn data(mut self, value: serde_json::Value) -> Self {
        self.data = Some(value);
        self
    }

    /// Build the Settlement entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Settlement, String> {
        let settlement_number = self.settlement_number.ok_or_else(|| "settlement_number is required".to_string())?;
        let provider_id = self.provider_id.ok_or_else(|| "provider_id is required".to_string())?;
        let period_start = self.period_start.ok_or_else(|| "period_start is required".to_string())?;
        let period_end = self.period_end.ok_or_else(|| "period_end is required".to_string())?;
        let gross_amount = self.gross_amount.ok_or_else(|| "gross_amount is required".to_string())?;
        let platform_commission_rate = self.platform_commission_rate.ok_or_else(|| "platform_commission_rate is required".to_string())?;
        let net_amount = self.net_amount.ok_or_else(|| "net_amount is required".to_string())?;

        Ok(Settlement {
            id: Uuid::new_v4(),
            settlement_number,
            provider_id,
            period_start,
            period_end,
            frequency: self.frequency.unwrap_or(SettlementFrequency::default()),
            currency: self.currency.unwrap_or("IDR".to_string()),
            gross_amount,
            platform_commission: self.platform_commission.unwrap_or(Decimal::from(0)),
            platform_commission_rate,
            transaction_fees: self.transaction_fees.unwrap_or(Decimal::from(0)),
            payment_gateway_fees: self.payment_gateway_fees.unwrap_or(Decimal::from(0)),
            refunds_amount: self.refunds_amount.unwrap_or(Decimal::from(0)),
            adjustments_amount: self.adjustments_amount.unwrap_or(Decimal::from(0)),
            adjustment_notes: self.adjustment_notes,
            withholding_tax: self.withholding_tax.unwrap_or(Decimal::from(0)),
            withholding_tax_rate: self.withholding_tax_rate,
            net_amount,
            bank_account_id: self.bank_account_id,
            status: self.status.unwrap_or(SettlementStatus::default()),
            scheduled_at: self.scheduled_at,
            processing_started_at: self.processing_started_at,
            completed_at: self.completed_at,
            failed_at: self.failed_at,
            failure_reason: self.failure_reason,
            transfer_reference: self.transfer_reference,
            transfer_receipt_url: self.transfer_receipt_url,
            is_on_hold: self.is_on_hold.unwrap_or(false),
            hold_reason: self.hold_reason,
            held_by: self.held_by,
            held_at: self.held_at,
            released_by: self.released_by,
            released_at: self.released_at,
            invoice_id: self.invoice_id,
            invoice_generated: self.invoice_generated.unwrap_or(false),
            notes: self.notes,
            data: self.data.unwrap_or(serde_json::json!({"total_orders":0,"total_payments":0,"bank_name":null,"account_number":null,"account_name":null})),
            metadata: AuditMetadata::default(),
        })
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::BankAccountType;
use super::BankAccountStatus;
use super::VerificationMethod;
use super::AuditMetadata;

/// Strongly-typed ID for ProviderBankAccount
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderBankAccountId(pub Uuid);

impl ProviderBankAccountId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for ProviderBankAccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ProviderBankAccountId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for ProviderBankAccountId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<ProviderBankAccountId> for Uuid {
    fn from(id: ProviderBankAccountId) -> Self { id.0 }
}

impl AsRef<Uuid> for ProviderBankAccountId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for ProviderBankAccountId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProviderBankAccount {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub bank_code: String,
    pub bank_name: String,
    pub account_number: String,
    pub account_name: String,
    pub account_type: BankAccountType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swift_code: Option<String>,
    pub status: BankAccountStatus,
    pub is_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<VerificationMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_by: Option<Uuid>,
    pub is_primary: bool,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_status: Option<String>,
    pub gateway_metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
    pub total_settlements: i32,
    pub total_amount_received: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_settlement_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_settlement_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl ProviderBankAccount {
    /// Create a builder for ProviderBankAccount
    pub fn builder() -> ProviderBankAccountBuilder {
        ProviderBankAccountBuilder::default()
    }

    /// Create a new ProviderBankAccount with required fields
    pub fn new(provider_id: Uuid, bank_code: String, bank_name: String, account_number: String, account_name: String, account_type: BankAccountType, status: BankAccountStatus, is_verified: bool, is_primary: bool, is_active: bool, gateway_metadata: serde_json::Value, total_settlements: i32, total_amount_received: Decimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            provider_id,
            bank_code,
            bank_name,
            account_number,
            account_name,
            account_type,
            branch_name: None,
            branch_code: None,
            swift_code: None,
            status,
            is_verified,
            verified_at: None,
            verified_by: None,
            verification_method: None,
            verification_notes: None,
            rejection_reason: None,
            rejected_at: None,
            rejected_by: None,
            is_primary,
            is_active,
            gateway_account_id: None,
            gateway_status: None,
            gateway_metadata,
            document_url: None,
            document_type: None,
            total_settlements,
            total_amount_received,
            last_settlement_at: None,
            last_settlement_amount: None,
            notes: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> ProviderBankAccountId {
        ProviderBankAccountId(self.id)
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
    pub fn status(&self) -> &BankAccountStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the branch_name field (chainable)
    pub fn with_branch_name(mut self, value: String) -> Self {
        self.branch_name = Some(value);
        self
    }

    /// Set the branch_code field (chainable)
    pub fn with_branch_code(mut self, value: String) -> Self {
        self.branch_code = Some(value);
        self
    }

    /// Set the swift_code field (chainable)
    pub fn with_swift_code(mut self, value: String) -> Self {
        self.swift_code = Some(value);
        self
    }

    /// Set the verified_at field (chainable)
    pub fn with_verified_at(mut self, value: DateTime<Utc>) -> Self {
        self.verified_at = Some(value);
        self
    }

    /// Set the verified_by field (chainable)
    pub fn with_verified_by(mut self, value: Uuid) -> Self {
        self.verified_by = Some(value);
        self
    }

    /// Set the verification_method field (chainable)
    pub fn with_verification_method(mut self, value: VerificationMethod) -> Self {
        self.verification_method = Some(value);
        self
    }

    /// Set the verification_notes field (chainable)
    pub fn with_verification_notes(mut self, value: String) -> Self {
        self.verification_notes = Some(value);
        self
    }

    /// Set the rejection_reason field (chainable)
    pub fn with_rejection_reason(mut self, value: String) -> Self {
        self.rejection_reason = Some(value);
        self
    }

    /// Set the rejected_at field (chainable)
    pub fn with_rejected_at(mut self, value: DateTime<Utc>) -> Self {
        self.rejected_at = Some(value);
        self
    }

    /// Set the rejected_by field (chainable)
    pub fn with_rejected_by(mut self, value: Uuid) -> Self {
        self.rejected_by = Some(value);
        self
    }

    /// Set the gateway_account_id field (chainable)
    pub fn with_gateway_account_id(mut self, value: String) -> Self {
        self.gateway_account_id = Some(value);
        self
    }

    /// Set the gateway_status field (chainable)
    pub fn with_gateway_status(mut self, value: String) -> Self {
        self.gateway_status = Some(value);
        self
    }

    /// Set the document_url field (chainable)
    pub fn with_document_url(mut self, value: String) -> Self {
        self.document_url = Some(value);
        self
    }

    /// Set the document_type field (chainable)
    pub fn with_document_type(mut self, value: String) -> Self {
        self.document_type = Some(value);
        self
    }

    /// Set the last_settlement_at field (chainable)
    pub fn with_last_settlement_at(mut self, value: DateTime<Utc>) -> Self {
        self.last_settlement_at = Some(value);
        self
    }

    /// Set the last_settlement_amount field (chainable)
    pub fn with_last_settlement_amount(mut self, value: Decimal) -> Self {
        self.last_settlement_amount = Some(value);
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
                "provider_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.provider_id = v; }
                }
                "bank_code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.bank_code = v; }
                }
                "bank_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.bank_name = v; }
                }
                "account_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.account_number = v; }
                }
                "account_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.account_name = v; }
                }
                "account_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.account_type = v; }
                }
                "branch_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.branch_name = v; }
                }
                "branch_code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.branch_code = v; }
                }
                "swift_code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.swift_code = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "is_verified" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_verified = v; }
                }
                "verified_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.verified_at = v; }
                }
                "verified_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.verified_by = v; }
                }
                "verification_method" => {
                    if let Ok(v) = serde_json::from_value(value) { self.verification_method = v; }
                }
                "verification_notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.verification_notes = v; }
                }
                "rejection_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.rejection_reason = v; }
                }
                "rejected_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.rejected_at = v; }
                }
                "rejected_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.rejected_by = v; }
                }
                "is_primary" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_primary = v; }
                }
                "is_active" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_active = v; }
                }
                "gateway_account_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gateway_account_id = v; }
                }
                "gateway_status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gateway_status = v; }
                }
                "gateway_metadata" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gateway_metadata = v; }
                }
                "document_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.document_url = v; }
                }
                "document_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.document_type = v; }
                }
                "total_settlements" => {
                    if let Ok(v) = serde_json::from_value(value) { self.total_settlements = v; }
                }
                "total_amount_received" => {
                    if let Ok(v) = serde_json::from_value(value) { self.total_amount_received = v; }
                }
                "last_settlement_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.last_settlement_at = v; }
                }
                "last_settlement_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.last_settlement_amount = v; }
                }
                "notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.notes = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for ProviderBankAccount {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "ProviderBankAccount"
    }
}

impl backbone_core::PersistentEntity for ProviderBankAccount {
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

impl backbone_orm::EntityRepoMeta for ProviderBankAccount {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("provider_id".to_string(), "uuid".to_string());
        m.insert("account_type".to_string(), "bank_account_type".to_string());
        m.insert("status".to_string(), "bank_account_status".to_string());
        m.insert("verification_method".to_string(), "verification_method".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["bank_code", "bank_name", "account_number", "account_name"]
    }
}

/// Builder for ProviderBankAccount entity
///
/// Provides a fluent API for constructing ProviderBankAccount instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct ProviderBankAccountBuilder {
    provider_id: Option<Uuid>,
    bank_code: Option<String>,
    bank_name: Option<String>,
    account_number: Option<String>,
    account_name: Option<String>,
    account_type: Option<BankAccountType>,
    branch_name: Option<String>,
    branch_code: Option<String>,
    swift_code: Option<String>,
    status: Option<BankAccountStatus>,
    is_verified: Option<bool>,
    verified_at: Option<DateTime<Utc>>,
    verified_by: Option<Uuid>,
    verification_method: Option<VerificationMethod>,
    verification_notes: Option<String>,
    rejection_reason: Option<String>,
    rejected_at: Option<DateTime<Utc>>,
    rejected_by: Option<Uuid>,
    is_primary: Option<bool>,
    is_active: Option<bool>,
    gateway_account_id: Option<String>,
    gateway_status: Option<String>,
    gateway_metadata: Option<serde_json::Value>,
    document_url: Option<String>,
    document_type: Option<String>,
    total_settlements: Option<i32>,
    total_amount_received: Option<Decimal>,
    last_settlement_at: Option<DateTime<Utc>>,
    last_settlement_amount: Option<Decimal>,
    notes: Option<String>,
}

impl ProviderBankAccountBuilder {
    /// Set the provider_id field (required)
    pub fn provider_id(mut self, value: Uuid) -> Self {
        self.provider_id = Some(value);
        self
    }

    /// Set the bank_code field (required)
    pub fn bank_code(mut self, value: String) -> Self {
        self.bank_code = Some(value);
        self
    }

    /// Set the bank_name field (required)
    pub fn bank_name(mut self, value: String) -> Self {
        self.bank_name = Some(value);
        self
    }

    /// Set the account_number field (required)
    pub fn account_number(mut self, value: String) -> Self {
        self.account_number = Some(value);
        self
    }

    /// Set the account_name field (required)
    pub fn account_name(mut self, value: String) -> Self {
        self.account_name = Some(value);
        self
    }

    /// Set the account_type field (default: `BankAccountType::default()`)
    pub fn account_type(mut self, value: BankAccountType) -> Self {
        self.account_type = Some(value);
        self
    }

    /// Set the branch_name field (optional)
    pub fn branch_name(mut self, value: String) -> Self {
        self.branch_name = Some(value);
        self
    }

    /// Set the branch_code field (optional)
    pub fn branch_code(mut self, value: String) -> Self {
        self.branch_code = Some(value);
        self
    }

    /// Set the swift_code field (optional)
    pub fn swift_code(mut self, value: String) -> Self {
        self.swift_code = Some(value);
        self
    }

    /// Set the status field (default: `BankAccountStatus::default()`)
    pub fn status(mut self, value: BankAccountStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the is_verified field (default: `false`)
    pub fn is_verified(mut self, value: bool) -> Self {
        self.is_verified = Some(value);
        self
    }

    /// Set the verified_at field (optional)
    pub fn verified_at(mut self, value: DateTime<Utc>) -> Self {
        self.verified_at = Some(value);
        self
    }

    /// Set the verified_by field (optional)
    pub fn verified_by(mut self, value: Uuid) -> Self {
        self.verified_by = Some(value);
        self
    }

    /// Set the verification_method field (optional)
    pub fn verification_method(mut self, value: VerificationMethod) -> Self {
        self.verification_method = Some(value);
        self
    }

    /// Set the verification_notes field (optional)
    pub fn verification_notes(mut self, value: String) -> Self {
        self.verification_notes = Some(value);
        self
    }

    /// Set the rejection_reason field (optional)
    pub fn rejection_reason(mut self, value: String) -> Self {
        self.rejection_reason = Some(value);
        self
    }

    /// Set the rejected_at field (optional)
    pub fn rejected_at(mut self, value: DateTime<Utc>) -> Self {
        self.rejected_at = Some(value);
        self
    }

    /// Set the rejected_by field (optional)
    pub fn rejected_by(mut self, value: Uuid) -> Self {
        self.rejected_by = Some(value);
        self
    }

    /// Set the is_primary field (default: `false`)
    pub fn is_primary(mut self, value: bool) -> Self {
        self.is_primary = Some(value);
        self
    }

    /// Set the is_active field (default: `true`)
    pub fn is_active(mut self, value: bool) -> Self {
        self.is_active = Some(value);
        self
    }

    /// Set the gateway_account_id field (optional)
    pub fn gateway_account_id(mut self, value: String) -> Self {
        self.gateway_account_id = Some(value);
        self
    }

    /// Set the gateway_status field (optional)
    pub fn gateway_status(mut self, value: String) -> Self {
        self.gateway_status = Some(value);
        self
    }

    /// Set the gateway_metadata field (default: `serde_json::json!({})`)
    pub fn gateway_metadata(mut self, value: serde_json::Value) -> Self {
        self.gateway_metadata = Some(value);
        self
    }

    /// Set the document_url field (optional)
    pub fn document_url(mut self, value: String) -> Self {
        self.document_url = Some(value);
        self
    }

    /// Set the document_type field (optional)
    pub fn document_type(mut self, value: String) -> Self {
        self.document_type = Some(value);
        self
    }

    /// Set the total_settlements field (default: `0`)
    pub fn total_settlements(mut self, value: i32) -> Self {
        self.total_settlements = Some(value);
        self
    }

    /// Set the total_amount_received field (default: `Decimal::from(0)`)
    pub fn total_amount_received(mut self, value: Decimal) -> Self {
        self.total_amount_received = Some(value);
        self
    }

    /// Set the last_settlement_at field (optional)
    pub fn last_settlement_at(mut self, value: DateTime<Utc>) -> Self {
        self.last_settlement_at = Some(value);
        self
    }

    /// Set the last_settlement_amount field (optional)
    pub fn last_settlement_amount(mut self, value: Decimal) -> Self {
        self.last_settlement_amount = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Build the ProviderBankAccount entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<ProviderBankAccount, String> {
        let provider_id = self.provider_id.ok_or_else(|| "provider_id is required".to_string())?;
        let bank_code = self.bank_code.ok_or_else(|| "bank_code is required".to_string())?;
        let bank_name = self.bank_name.ok_or_else(|| "bank_name is required".to_string())?;
        let account_number = self.account_number.ok_or_else(|| "account_number is required".to_string())?;
        let account_name = self.account_name.ok_or_else(|| "account_name is required".to_string())?;

        Ok(ProviderBankAccount {
            id: Uuid::new_v4(),
            provider_id,
            bank_code,
            bank_name,
            account_number,
            account_name,
            account_type: self.account_type.unwrap_or(BankAccountType::default()),
            branch_name: self.branch_name,
            branch_code: self.branch_code,
            swift_code: self.swift_code,
            status: self.status.unwrap_or(BankAccountStatus::default()),
            is_verified: self.is_verified.unwrap_or(false),
            verified_at: self.verified_at,
            verified_by: self.verified_by,
            verification_method: self.verification_method,
            verification_notes: self.verification_notes,
            rejection_reason: self.rejection_reason,
            rejected_at: self.rejected_at,
            rejected_by: self.rejected_by,
            is_primary: self.is_primary.unwrap_or(false),
            is_active: self.is_active.unwrap_or(true),
            gateway_account_id: self.gateway_account_id,
            gateway_status: self.gateway_status,
            gateway_metadata: self.gateway_metadata.unwrap_or(serde_json::json!({})),
            document_url: self.document_url,
            document_type: self.document_type,
            total_settlements: self.total_settlements.unwrap_or(0),
            total_amount_received: self.total_amount_received.unwrap_or(Decimal::from(0)),
            last_settlement_at: self.last_settlement_at,
            last_settlement_amount: self.last_settlement_amount,
            notes: self.notes,
            metadata: AuditMetadata::default(),
        })
    }
}

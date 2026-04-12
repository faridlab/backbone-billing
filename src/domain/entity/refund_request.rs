use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::RefundReason;
use super::RefundStatus;
use super::RefundMethod;
use super::AuditMetadata;

/// Strongly-typed ID for RefundRequest
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RefundRequestId(pub Uuid);

impl RefundRequestId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for RefundRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for RefundRequestId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for RefundRequestId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<RefundRequestId> for Uuid {
    fn from(id: RefundRequestId) -> Self { id.0 }
}

impl AsRef<Uuid> for RefundRequestId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for RefundRequestId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefundRequest {
    pub id: Uuid,
    pub refund_number: String,
    pub payment_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<Uuid>,
    pub requester_type: String,
    pub requester_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester_name: Option<String>,
    pub currency: String,
    pub original_amount: Decimal,
    pub requested_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_amount: Option<Decimal>,
    pub refunded_amount: Decimal,
    pub reason: RefundReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    pub evidence_urls: serde_json::Value,
    pub status: RefundStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_refund_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_method: Option<RefundMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    pub customer_notified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_notified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl RefundRequest {
    /// Create a builder for RefundRequest
    pub fn builder() -> RefundRequestBuilder {
        RefundRequestBuilder::default()
    }

    /// Create a new RefundRequest with required fields
    pub fn new(refund_number: String, payment_id: Uuid, requester_type: String, requester_id: Uuid, currency: String, original_amount: Decimal, requested_amount: Decimal, refunded_amount: Decimal, reason: RefundReason, evidence_urls: serde_json::Value, status: RefundStatus, customer_notified: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            refund_number,
            payment_id,
            order_id: None,
            requester_type,
            requester_id,
            requester_name: None,
            currency,
            original_amount,
            requested_amount,
            approved_amount: None,
            refunded_amount,
            reason,
            reason_detail: None,
            evidence_urls,
            status,
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            approved_by: None,
            approved_at: None,
            approval_notes: None,
            rejected_by: None,
            rejected_at: None,
            rejection_reason: None,
            processing_started_at: None,
            completed_at: None,
            failed_at: None,
            failure_reason: None,
            gateway_refund_id: None,
            gateway_status: None,
            refund_method: None,
            bank_name: None,
            account_number: None,
            account_name: None,
            customer_notified,
            customer_notified_at: None,
            notes: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> RefundRequestId {
        RefundRequestId(self.id)
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
    pub fn status(&self) -> &RefundStatus {
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

    /// Set the requester_name field (chainable)
    pub fn with_requester_name(mut self, value: String) -> Self {
        self.requester_name = Some(value);
        self
    }

    /// Set the approved_amount field (chainable)
    pub fn with_approved_amount(mut self, value: Decimal) -> Self {
        self.approved_amount = Some(value);
        self
    }

    /// Set the reason_detail field (chainable)
    pub fn with_reason_detail(mut self, value: String) -> Self {
        self.reason_detail = Some(value);
        self
    }

    /// Set the reviewed_by field (chainable)
    pub fn with_reviewed_by(mut self, value: Uuid) -> Self {
        self.reviewed_by = Some(value);
        self
    }

    /// Set the reviewed_at field (chainable)
    pub fn with_reviewed_at(mut self, value: DateTime<Utc>) -> Self {
        self.reviewed_at = Some(value);
        self
    }

    /// Set the review_notes field (chainable)
    pub fn with_review_notes(mut self, value: String) -> Self {
        self.review_notes = Some(value);
        self
    }

    /// Set the approved_by field (chainable)
    pub fn with_approved_by(mut self, value: Uuid) -> Self {
        self.approved_by = Some(value);
        self
    }

    /// Set the approved_at field (chainable)
    pub fn with_approved_at(mut self, value: DateTime<Utc>) -> Self {
        self.approved_at = Some(value);
        self
    }

    /// Set the approval_notes field (chainable)
    pub fn with_approval_notes(mut self, value: String) -> Self {
        self.approval_notes = Some(value);
        self
    }

    /// Set the rejected_by field (chainable)
    pub fn with_rejected_by(mut self, value: Uuid) -> Self {
        self.rejected_by = Some(value);
        self
    }

    /// Set the rejected_at field (chainable)
    pub fn with_rejected_at(mut self, value: DateTime<Utc>) -> Self {
        self.rejected_at = Some(value);
        self
    }

    /// Set the rejection_reason field (chainable)
    pub fn with_rejection_reason(mut self, value: String) -> Self {
        self.rejection_reason = Some(value);
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

    /// Set the gateway_refund_id field (chainable)
    pub fn with_gateway_refund_id(mut self, value: String) -> Self {
        self.gateway_refund_id = Some(value);
        self
    }

    /// Set the gateway_status field (chainable)
    pub fn with_gateway_status(mut self, value: String) -> Self {
        self.gateway_status = Some(value);
        self
    }

    /// Set the refund_method field (chainable)
    pub fn with_refund_method(mut self, value: RefundMethod) -> Self {
        self.refund_method = Some(value);
        self
    }

    /// Set the bank_name field (chainable)
    pub fn with_bank_name(mut self, value: String) -> Self {
        self.bank_name = Some(value);
        self
    }

    /// Set the account_number field (chainable)
    pub fn with_account_number(mut self, value: String) -> Self {
        self.account_number = Some(value);
        self
    }

    /// Set the account_name field (chainable)
    pub fn with_account_name(mut self, value: String) -> Self {
        self.account_name = Some(value);
        self
    }

    /// Set the customer_notified_at field (chainable)
    pub fn with_customer_notified_at(mut self, value: DateTime<Utc>) -> Self {
        self.customer_notified_at = Some(value);
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
                "refund_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.refund_number = v; }
                }
                "payment_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payment_id = v; }
                }
                "order_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.order_id = v; }
                }
                "requester_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.requester_type = v; }
                }
                "requester_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.requester_id = v; }
                }
                "requester_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.requester_name = v; }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.currency = v; }
                }
                "original_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.original_amount = v; }
                }
                "requested_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.requested_amount = v; }
                }
                "approved_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.approved_amount = v; }
                }
                "refunded_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.refunded_amount = v; }
                }
                "reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reason = v; }
                }
                "reason_detail" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reason_detail = v; }
                }
                "evidence_urls" => {
                    if let Ok(v) = serde_json::from_value(value) { self.evidence_urls = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "reviewed_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reviewed_by = v; }
                }
                "reviewed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reviewed_at = v; }
                }
                "review_notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.review_notes = v; }
                }
                "approved_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.approved_by = v; }
                }
                "approved_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.approved_at = v; }
                }
                "approval_notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.approval_notes = v; }
                }
                "rejected_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.rejected_by = v; }
                }
                "rejected_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.rejected_at = v; }
                }
                "rejection_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.rejection_reason = v; }
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
                "gateway_refund_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gateway_refund_id = v; }
                }
                "gateway_status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gateway_status = v; }
                }
                "refund_method" => {
                    if let Ok(v) = serde_json::from_value(value) { self.refund_method = v; }
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
                "customer_notified" => {
                    if let Ok(v) = serde_json::from_value(value) { self.customer_notified = v; }
                }
                "customer_notified_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.customer_notified_at = v; }
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

impl super::Entity for RefundRequest {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "RefundRequest"
    }
}

impl backbone_core::PersistentEntity for RefundRequest {
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

impl backbone_orm::EntityRepoMeta for RefundRequest {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("payment_id".to_string(), "uuid".to_string());
        m.insert("order_id".to_string(), "uuid".to_string());
        m.insert("requester_id".to_string(), "uuid".to_string());
        m.insert("reason".to_string(), "refund_reason".to_string());
        m.insert("status".to_string(), "refund_status".to_string());
        m.insert("refund_method".to_string(), "refund_method".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["refund_number", "requester_type", "currency"]
    }
}

/// Builder for RefundRequest entity
///
/// Provides a fluent API for constructing RefundRequest instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct RefundRequestBuilder {
    refund_number: Option<String>,
    payment_id: Option<Uuid>,
    order_id: Option<Uuid>,
    requester_type: Option<String>,
    requester_id: Option<Uuid>,
    requester_name: Option<String>,
    currency: Option<String>,
    original_amount: Option<Decimal>,
    requested_amount: Option<Decimal>,
    approved_amount: Option<Decimal>,
    refunded_amount: Option<Decimal>,
    reason: Option<RefundReason>,
    reason_detail: Option<String>,
    evidence_urls: Option<serde_json::Value>,
    status: Option<RefundStatus>,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    review_notes: Option<String>,
    approved_by: Option<Uuid>,
    approved_at: Option<DateTime<Utc>>,
    approval_notes: Option<String>,
    rejected_by: Option<Uuid>,
    rejected_at: Option<DateTime<Utc>>,
    rejection_reason: Option<String>,
    processing_started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    gateway_refund_id: Option<String>,
    gateway_status: Option<String>,
    refund_method: Option<RefundMethod>,
    bank_name: Option<String>,
    account_number: Option<String>,
    account_name: Option<String>,
    customer_notified: Option<bool>,
    customer_notified_at: Option<DateTime<Utc>>,
    notes: Option<String>,
}

impl RefundRequestBuilder {
    /// Set the refund_number field (required)
    pub fn refund_number(mut self, value: String) -> Self {
        self.refund_number = Some(value);
        self
    }

    /// Set the payment_id field (required)
    pub fn payment_id(mut self, value: Uuid) -> Self {
        self.payment_id = Some(value);
        self
    }

    /// Set the order_id field (optional)
    pub fn order_id(mut self, value: Uuid) -> Self {
        self.order_id = Some(value);
        self
    }

    /// Set the requester_type field (required)
    pub fn requester_type(mut self, value: String) -> Self {
        self.requester_type = Some(value);
        self
    }

    /// Set the requester_id field (required)
    pub fn requester_id(mut self, value: Uuid) -> Self {
        self.requester_id = Some(value);
        self
    }

    /// Set the requester_name field (optional)
    pub fn requester_name(mut self, value: String) -> Self {
        self.requester_name = Some(value);
        self
    }

    /// Set the currency field (default: `"IDR".to_string()`)
    pub fn currency(mut self, value: String) -> Self {
        self.currency = Some(value);
        self
    }

    /// Set the original_amount field (required)
    pub fn original_amount(mut self, value: Decimal) -> Self {
        self.original_amount = Some(value);
        self
    }

    /// Set the requested_amount field (required)
    pub fn requested_amount(mut self, value: Decimal) -> Self {
        self.requested_amount = Some(value);
        self
    }

    /// Set the approved_amount field (optional)
    pub fn approved_amount(mut self, value: Decimal) -> Self {
        self.approved_amount = Some(value);
        self
    }

    /// Set the refunded_amount field (default: `Decimal::from(0)`)
    pub fn refunded_amount(mut self, value: Decimal) -> Self {
        self.refunded_amount = Some(value);
        self
    }

    /// Set the reason field (default: `RefundReason::default()`)
    pub fn reason(mut self, value: RefundReason) -> Self {
        self.reason = Some(value);
        self
    }

    /// Set the reason_detail field (optional)
    pub fn reason_detail(mut self, value: String) -> Self {
        self.reason_detail = Some(value);
        self
    }

    /// Set the evidence_urls field (default: `serde_json::json!([])`)
    pub fn evidence_urls(mut self, value: serde_json::Value) -> Self {
        self.evidence_urls = Some(value);
        self
    }

    /// Set the status field (default: `RefundStatus::default()`)
    pub fn status(mut self, value: RefundStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the reviewed_by field (optional)
    pub fn reviewed_by(mut self, value: Uuid) -> Self {
        self.reviewed_by = Some(value);
        self
    }

    /// Set the reviewed_at field (optional)
    pub fn reviewed_at(mut self, value: DateTime<Utc>) -> Self {
        self.reviewed_at = Some(value);
        self
    }

    /// Set the review_notes field (optional)
    pub fn review_notes(mut self, value: String) -> Self {
        self.review_notes = Some(value);
        self
    }

    /// Set the approved_by field (optional)
    pub fn approved_by(mut self, value: Uuid) -> Self {
        self.approved_by = Some(value);
        self
    }

    /// Set the approved_at field (optional)
    pub fn approved_at(mut self, value: DateTime<Utc>) -> Self {
        self.approved_at = Some(value);
        self
    }

    /// Set the approval_notes field (optional)
    pub fn approval_notes(mut self, value: String) -> Self {
        self.approval_notes = Some(value);
        self
    }

    /// Set the rejected_by field (optional)
    pub fn rejected_by(mut self, value: Uuid) -> Self {
        self.rejected_by = Some(value);
        self
    }

    /// Set the rejected_at field (optional)
    pub fn rejected_at(mut self, value: DateTime<Utc>) -> Self {
        self.rejected_at = Some(value);
        self
    }

    /// Set the rejection_reason field (optional)
    pub fn rejection_reason(mut self, value: String) -> Self {
        self.rejection_reason = Some(value);
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

    /// Set the gateway_refund_id field (optional)
    pub fn gateway_refund_id(mut self, value: String) -> Self {
        self.gateway_refund_id = Some(value);
        self
    }

    /// Set the gateway_status field (optional)
    pub fn gateway_status(mut self, value: String) -> Self {
        self.gateway_status = Some(value);
        self
    }

    /// Set the refund_method field (optional)
    pub fn refund_method(mut self, value: RefundMethod) -> Self {
        self.refund_method = Some(value);
        self
    }

    /// Set the bank_name field (optional)
    pub fn bank_name(mut self, value: String) -> Self {
        self.bank_name = Some(value);
        self
    }

    /// Set the account_number field (optional)
    pub fn account_number(mut self, value: String) -> Self {
        self.account_number = Some(value);
        self
    }

    /// Set the account_name field (optional)
    pub fn account_name(mut self, value: String) -> Self {
        self.account_name = Some(value);
        self
    }

    /// Set the customer_notified field (default: `false`)
    pub fn customer_notified(mut self, value: bool) -> Self {
        self.customer_notified = Some(value);
        self
    }

    /// Set the customer_notified_at field (optional)
    pub fn customer_notified_at(mut self, value: DateTime<Utc>) -> Self {
        self.customer_notified_at = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Build the RefundRequest entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<RefundRequest, String> {
        let refund_number = self.refund_number.ok_or_else(|| "refund_number is required".to_string())?;
        let payment_id = self.payment_id.ok_or_else(|| "payment_id is required".to_string())?;
        let requester_type = self.requester_type.ok_or_else(|| "requester_type is required".to_string())?;
        let requester_id = self.requester_id.ok_or_else(|| "requester_id is required".to_string())?;
        let original_amount = self.original_amount.ok_or_else(|| "original_amount is required".to_string())?;
        let requested_amount = self.requested_amount.ok_or_else(|| "requested_amount is required".to_string())?;

        Ok(RefundRequest {
            id: Uuid::new_v4(),
            refund_number,
            payment_id,
            order_id: self.order_id,
            requester_type,
            requester_id,
            requester_name: self.requester_name,
            currency: self.currency.unwrap_or("IDR".to_string()),
            original_amount,
            requested_amount,
            approved_amount: self.approved_amount,
            refunded_amount: self.refunded_amount.unwrap_or(Decimal::from(0)),
            reason: self.reason.unwrap_or(RefundReason::default()),
            reason_detail: self.reason_detail,
            evidence_urls: self.evidence_urls.unwrap_or(serde_json::json!([])),
            status: self.status.unwrap_or(RefundStatus::default()),
            reviewed_by: self.reviewed_by,
            reviewed_at: self.reviewed_at,
            review_notes: self.review_notes,
            approved_by: self.approved_by,
            approved_at: self.approved_at,
            approval_notes: self.approval_notes,
            rejected_by: self.rejected_by,
            rejected_at: self.rejected_at,
            rejection_reason: self.rejection_reason,
            processing_started_at: self.processing_started_at,
            completed_at: self.completed_at,
            failed_at: self.failed_at,
            failure_reason: self.failure_reason,
            gateway_refund_id: self.gateway_refund_id,
            gateway_status: self.gateway_status,
            refund_method: self.refund_method,
            bank_name: self.bank_name,
            account_number: self.account_number,
            account_name: self.account_name,
            customer_notified: self.customer_notified.unwrap_or(false),
            customer_notified_at: self.customer_notified_at,
            notes: self.notes,
            metadata: AuditMetadata::default(),
        })
    }
}

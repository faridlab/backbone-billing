use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::EarningType;
use super::EarningStatus;
use super::AuditMetadata;

/// Strongly-typed ID for AgentEarning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentEarningId(pub Uuid);

impl AgentEarningId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for AgentEarningId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AgentEarningId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for AgentEarningId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<AgentEarningId> for Uuid {
    fn from(id: AgentEarningId) -> Self { id.0 }
}

impl AsRef<Uuid> for AgentEarningId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for AgentEarningId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentEarning {
    pub id: Uuid,
    pub agent_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<Uuid>,
    pub earning_type: EarningType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub currency: String,
    pub base_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<Decimal>,
    pub distance_bonus: Decimal,
    pub time_bonus: Decimal,
    pub surge_bonus: Decimal,
    pub incentive_bonus: Decimal,
    pub tip_amount: Decimal,
    pub deduction_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deduction_reason: Option<String>,
    pub total_earning: Decimal,
    pub status: EarningStatus,
    pub earned_at: DateTime<Utc>,
    pub earning_date: NaiveDate,
    pub is_paid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payout_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<DateTime<Utc>>,
    pub cod_collected: Decimal,
    pub cod_deposited: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cod_deposited_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl AgentEarning {
    /// Create a builder for AgentEarning
    pub fn builder() -> AgentEarningBuilder {
        AgentEarningBuilder::default()
    }

    /// Create a new AgentEarning with required fields
    pub fn new(agent_id: Uuid, earning_type: EarningType, currency: String, base_amount: Decimal, distance_bonus: Decimal, time_bonus: Decimal, surge_bonus: Decimal, incentive_bonus: Decimal, tip_amount: Decimal, deduction_amount: Decimal, total_earning: Decimal, status: EarningStatus, earned_at: DateTime<Utc>, earning_date: NaiveDate, is_paid: bool, cod_collected: Decimal, cod_deposited: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            task_id: None,
            order_id: None,
            earning_type,
            description: None,
            currency,
            base_amount,
            distance_km: None,
            distance_bonus,
            time_bonus,
            surge_bonus,
            incentive_bonus,
            tip_amount,
            deduction_amount,
            deduction_reason: None,
            total_earning,
            status,
            earned_at,
            earning_date,
            is_paid,
            payout_id: None,
            paid_at: None,
            cod_collected,
            cod_deposited,
            cod_deposited_at: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> AgentEarningId {
        AgentEarningId(self.id)
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
    pub fn status(&self) -> &EarningStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the task_id field (chainable)
    pub fn with_task_id(mut self, value: Uuid) -> Self {
        self.task_id = Some(value);
        self
    }

    /// Set the order_id field (chainable)
    pub fn with_order_id(mut self, value: Uuid) -> Self {
        self.order_id = Some(value);
        self
    }

    /// Set the description field (chainable)
    pub fn with_description(mut self, value: String) -> Self {
        self.description = Some(value);
        self
    }

    /// Set the distance_km field (chainable)
    pub fn with_distance_km(mut self, value: Decimal) -> Self {
        self.distance_km = Some(value);
        self
    }

    /// Set the deduction_reason field (chainable)
    pub fn with_deduction_reason(mut self, value: String) -> Self {
        self.deduction_reason = Some(value);
        self
    }

    /// Set the payout_id field (chainable)
    pub fn with_payout_id(mut self, value: Uuid) -> Self {
        self.payout_id = Some(value);
        self
    }

    /// Set the paid_at field (chainable)
    pub fn with_paid_at(mut self, value: DateTime<Utc>) -> Self {
        self.paid_at = Some(value);
        self
    }

    /// Set the cod_deposited_at field (chainable)
    pub fn with_cod_deposited_at(mut self, value: DateTime<Utc>) -> Self {
        self.cod_deposited_at = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "agent_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.agent_id = v; }
                }
                "task_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.task_id = v; }
                }
                "order_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.order_id = v; }
                }
                "earning_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.earning_type = v; }
                }
                "description" => {
                    if let Ok(v) = serde_json::from_value(value) { self.description = v; }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.currency = v; }
                }
                "base_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.base_amount = v; }
                }
                "distance_km" => {
                    if let Ok(v) = serde_json::from_value(value) { self.distance_km = v; }
                }
                "distance_bonus" => {
                    if let Ok(v) = serde_json::from_value(value) { self.distance_bonus = v; }
                }
                "time_bonus" => {
                    if let Ok(v) = serde_json::from_value(value) { self.time_bonus = v; }
                }
                "surge_bonus" => {
                    if let Ok(v) = serde_json::from_value(value) { self.surge_bonus = v; }
                }
                "incentive_bonus" => {
                    if let Ok(v) = serde_json::from_value(value) { self.incentive_bonus = v; }
                }
                "tip_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.tip_amount = v; }
                }
                "deduction_amount" => {
                    if let Ok(v) = serde_json::from_value(value) { self.deduction_amount = v; }
                }
                "deduction_reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.deduction_reason = v; }
                }
                "total_earning" => {
                    if let Ok(v) = serde_json::from_value(value) { self.total_earning = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "earned_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.earned_at = v; }
                }
                "earning_date" => {
                    if let Ok(v) = serde_json::from_value(value) { self.earning_date = v; }
                }
                "is_paid" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_paid = v; }
                }
                "payout_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.payout_id = v; }
                }
                "paid_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.paid_at = v; }
                }
                "cod_collected" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cod_collected = v; }
                }
                "cod_deposited" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cod_deposited = v; }
                }
                "cod_deposited_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cod_deposited_at = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for AgentEarning {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "AgentEarning"
    }
}

impl backbone_core::PersistentEntity for AgentEarning {
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

impl backbone_orm::EntityRepoMeta for AgentEarning {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("agent_id".to_string(), "uuid".to_string());
        m.insert("task_id".to_string(), "uuid".to_string());
        m.insert("order_id".to_string(), "uuid".to_string());
        m.insert("payout_id".to_string(), "uuid".to_string());
        m.insert("earning_type".to_string(), "earning_type".to_string());
        m.insert("status".to_string(), "earning_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["currency"]
    }
}

/// Builder for AgentEarning entity
///
/// Provides a fluent API for constructing AgentEarning instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct AgentEarningBuilder {
    agent_id: Option<Uuid>,
    task_id: Option<Uuid>,
    order_id: Option<Uuid>,
    earning_type: Option<EarningType>,
    description: Option<String>,
    currency: Option<String>,
    base_amount: Option<Decimal>,
    distance_km: Option<Decimal>,
    distance_bonus: Option<Decimal>,
    time_bonus: Option<Decimal>,
    surge_bonus: Option<Decimal>,
    incentive_bonus: Option<Decimal>,
    tip_amount: Option<Decimal>,
    deduction_amount: Option<Decimal>,
    deduction_reason: Option<String>,
    total_earning: Option<Decimal>,
    status: Option<EarningStatus>,
    earned_at: Option<DateTime<Utc>>,
    earning_date: Option<NaiveDate>,
    is_paid: Option<bool>,
    payout_id: Option<Uuid>,
    paid_at: Option<DateTime<Utc>>,
    cod_collected: Option<Decimal>,
    cod_deposited: Option<bool>,
    cod_deposited_at: Option<DateTime<Utc>>,
}

impl AgentEarningBuilder {
    /// Set the agent_id field (required)
    pub fn agent_id(mut self, value: Uuid) -> Self {
        self.agent_id = Some(value);
        self
    }

    /// Set the task_id field (optional)
    pub fn task_id(mut self, value: Uuid) -> Self {
        self.task_id = Some(value);
        self
    }

    /// Set the order_id field (optional)
    pub fn order_id(mut self, value: Uuid) -> Self {
        self.order_id = Some(value);
        self
    }

    /// Set the earning_type field (required)
    pub fn earning_type(mut self, value: EarningType) -> Self {
        self.earning_type = Some(value);
        self
    }

    /// Set the description field (optional)
    pub fn description(mut self, value: String) -> Self {
        self.description = Some(value);
        self
    }

    /// Set the currency field (default: `"IDR".to_string()`)
    pub fn currency(mut self, value: String) -> Self {
        self.currency = Some(value);
        self
    }

    /// Set the base_amount field (required)
    pub fn base_amount(mut self, value: Decimal) -> Self {
        self.base_amount = Some(value);
        self
    }

    /// Set the distance_km field (optional)
    pub fn distance_km(mut self, value: Decimal) -> Self {
        self.distance_km = Some(value);
        self
    }

    /// Set the distance_bonus field (default: `Decimal::from(0)`)
    pub fn distance_bonus(mut self, value: Decimal) -> Self {
        self.distance_bonus = Some(value);
        self
    }

    /// Set the time_bonus field (default: `Decimal::from(0)`)
    pub fn time_bonus(mut self, value: Decimal) -> Self {
        self.time_bonus = Some(value);
        self
    }

    /// Set the surge_bonus field (default: `Decimal::from(0)`)
    pub fn surge_bonus(mut self, value: Decimal) -> Self {
        self.surge_bonus = Some(value);
        self
    }

    /// Set the incentive_bonus field (default: `Decimal::from(0)`)
    pub fn incentive_bonus(mut self, value: Decimal) -> Self {
        self.incentive_bonus = Some(value);
        self
    }

    /// Set the tip_amount field (default: `Decimal::from(0)`)
    pub fn tip_amount(mut self, value: Decimal) -> Self {
        self.tip_amount = Some(value);
        self
    }

    /// Set the deduction_amount field (default: `Decimal::from(0)`)
    pub fn deduction_amount(mut self, value: Decimal) -> Self {
        self.deduction_amount = Some(value);
        self
    }

    /// Set the deduction_reason field (optional)
    pub fn deduction_reason(mut self, value: String) -> Self {
        self.deduction_reason = Some(value);
        self
    }

    /// Set the total_earning field (required)
    pub fn total_earning(mut self, value: Decimal) -> Self {
        self.total_earning = Some(value);
        self
    }

    /// Set the status field (default: `EarningStatus::default()`)
    pub fn status(mut self, value: EarningStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the earned_at field (required)
    pub fn earned_at(mut self, value: DateTime<Utc>) -> Self {
        self.earned_at = Some(value);
        self
    }

    /// Set the earning_date field (required)
    pub fn earning_date(mut self, value: NaiveDate) -> Self {
        self.earning_date = Some(value);
        self
    }

    /// Set the is_paid field (default: `false`)
    pub fn is_paid(mut self, value: bool) -> Self {
        self.is_paid = Some(value);
        self
    }

    /// Set the payout_id field (optional)
    pub fn payout_id(mut self, value: Uuid) -> Self {
        self.payout_id = Some(value);
        self
    }

    /// Set the paid_at field (optional)
    pub fn paid_at(mut self, value: DateTime<Utc>) -> Self {
        self.paid_at = Some(value);
        self
    }

    /// Set the cod_collected field (default: `Decimal::from(0)`)
    pub fn cod_collected(mut self, value: Decimal) -> Self {
        self.cod_collected = Some(value);
        self
    }

    /// Set the cod_deposited field (default: `false`)
    pub fn cod_deposited(mut self, value: bool) -> Self {
        self.cod_deposited = Some(value);
        self
    }

    /// Set the cod_deposited_at field (optional)
    pub fn cod_deposited_at(mut self, value: DateTime<Utc>) -> Self {
        self.cod_deposited_at = Some(value);
        self
    }

    /// Build the AgentEarning entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<AgentEarning, String> {
        let agent_id = self.agent_id.ok_or_else(|| "agent_id is required".to_string())?;
        let earning_type = self.earning_type.ok_or_else(|| "earning_type is required".to_string())?;
        let base_amount = self.base_amount.ok_or_else(|| "base_amount is required".to_string())?;
        let total_earning = self.total_earning.ok_or_else(|| "total_earning is required".to_string())?;
        let earned_at = self.earned_at.ok_or_else(|| "earned_at is required".to_string())?;
        let earning_date = self.earning_date.ok_or_else(|| "earning_date is required".to_string())?;

        Ok(AgentEarning {
            id: Uuid::new_v4(),
            agent_id,
            task_id: self.task_id,
            order_id: self.order_id,
            earning_type,
            description: self.description,
            currency: self.currency.unwrap_or("IDR".to_string()),
            base_amount,
            distance_km: self.distance_km,
            distance_bonus: self.distance_bonus.unwrap_or(Decimal::from(0)),
            time_bonus: self.time_bonus.unwrap_or(Decimal::from(0)),
            surge_bonus: self.surge_bonus.unwrap_or(Decimal::from(0)),
            incentive_bonus: self.incentive_bonus.unwrap_or(Decimal::from(0)),
            tip_amount: self.tip_amount.unwrap_or(Decimal::from(0)),
            deduction_amount: self.deduction_amount.unwrap_or(Decimal::from(0)),
            deduction_reason: self.deduction_reason,
            total_earning,
            status: self.status.unwrap_or(EarningStatus::default()),
            earned_at,
            earning_date,
            is_paid: self.is_paid.unwrap_or(false),
            payout_id: self.payout_id,
            paid_at: self.paid_at,
            cod_collected: self.cod_collected.unwrap_or(Decimal::from(0)),
            cod_deposited: self.cod_deposited.unwrap_or(false),
            cod_deposited_at: self.cod_deposited_at,
            metadata: AuditMetadata::default(),
        })
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::FinancialPeriodType;
use super::FinancialPeriodStatus;
use super::AuditMetadata;

/// Strongly-typed ID for FinancialPeriod
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FinancialPeriodId(pub Uuid);

impl FinancialPeriodId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for FinancialPeriodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for FinancialPeriodId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for FinancialPeriodId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<FinancialPeriodId> for Uuid {
    fn from(id: FinancialPeriodId) -> Self { id.0 }
}

impl AsRef<Uuid> for FinancialPeriodId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for FinancialPeriodId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FinancialPeriod {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub period_type: FinancialPeriodType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub fiscal_year: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiscal_month: Option<i32>,
    pub gross_revenue: Decimal,
    pub discounts_given: Decimal,
    pub refunds_issued: Decimal,
    pub net_revenue: Decimal,
    pub cogs: Decimal,
    pub labor_cost: Decimal,
    pub material_cost: Decimal,
    pub utility_expense: Decimal,
    pub maintenance_expense: Decimal,
    pub delivery_expense: Decimal,
    pub other_expense: Decimal,
    pub platform_commission: Decimal,
    pub transaction_fees: Decimal,
    pub subscription_fees: Decimal,
    pub gross_profit: Decimal,
    pub operating_profit: Decimal,
    pub net_profit: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_margin_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_margin_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_margin_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_growth_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_growth_percent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_order_value: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_per_kg: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_per_order: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_per_kg: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_per_order: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnl_report_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_generated_at: Option<DateTime<Utc>>,
    pub status: FinancialPeriodStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_by: Option<Uuid>,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub data: serde_json::Value,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl FinancialPeriod {
    /// Create a builder for FinancialPeriod
    pub fn builder() -> FinancialPeriodBuilder {
        FinancialPeriodBuilder::default()
    }

    /// Create a new FinancialPeriod with required fields
    pub fn new(provider_id: Uuid, period_type: FinancialPeriodType, period_start: DateTime<Utc>, period_end: DateTime<Utc>, fiscal_year: i32, gross_revenue: Decimal, discounts_given: Decimal, refunds_issued: Decimal, net_revenue: Decimal, cogs: Decimal, labor_cost: Decimal, material_cost: Decimal, utility_expense: Decimal, maintenance_expense: Decimal, delivery_expense: Decimal, other_expense: Decimal, platform_commission: Decimal, transaction_fees: Decimal, subscription_fees: Decimal, gross_profit: Decimal, operating_profit: Decimal, net_profit: Decimal, status: FinancialPeriodStatus, currency: String, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            provider_id,
            period_type,
            period_start,
            period_end,
            fiscal_year,
            fiscal_month: None,
            gross_revenue,
            discounts_given,
            refunds_issued,
            net_revenue,
            cogs,
            labor_cost,
            material_cost,
            utility_expense,
            maintenance_expense,
            delivery_expense,
            other_expense,
            platform_commission,
            transaction_fees,
            subscription_fees,
            gross_profit,
            operating_profit,
            net_profit,
            gross_margin_percent: None,
            operating_margin_percent: None,
            net_margin_percent: None,
            revenue_growth_percent: None,
            profit_growth_percent: None,
            average_order_value: None,
            cost_per_kg: None,
            cost_per_order: None,
            profit_per_kg: None,
            profit_per_order: None,
            pnl_report_url: None,
            report_generated_at: None,
            status,
            closed_at: None,
            closed_by: None,
            currency,
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
    pub fn typed_id(&self) -> FinancialPeriodId {
        FinancialPeriodId(self.id)
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
    pub fn status(&self) -> &FinancialPeriodStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the fiscal_month field (chainable)
    pub fn with_fiscal_month(mut self, value: i32) -> Self {
        self.fiscal_month = Some(value);
        self
    }

    /// Set the gross_margin_percent field (chainable)
    pub fn with_gross_margin_percent(mut self, value: Decimal) -> Self {
        self.gross_margin_percent = Some(value);
        self
    }

    /// Set the operating_margin_percent field (chainable)
    pub fn with_operating_margin_percent(mut self, value: Decimal) -> Self {
        self.operating_margin_percent = Some(value);
        self
    }

    /// Set the net_margin_percent field (chainable)
    pub fn with_net_margin_percent(mut self, value: Decimal) -> Self {
        self.net_margin_percent = Some(value);
        self
    }

    /// Set the revenue_growth_percent field (chainable)
    pub fn with_revenue_growth_percent(mut self, value: Decimal) -> Self {
        self.revenue_growth_percent = Some(value);
        self
    }

    /// Set the profit_growth_percent field (chainable)
    pub fn with_profit_growth_percent(mut self, value: Decimal) -> Self {
        self.profit_growth_percent = Some(value);
        self
    }

    /// Set the average_order_value field (chainable)
    pub fn with_average_order_value(mut self, value: Decimal) -> Self {
        self.average_order_value = Some(value);
        self
    }

    /// Set the cost_per_kg field (chainable)
    pub fn with_cost_per_kg(mut self, value: Decimal) -> Self {
        self.cost_per_kg = Some(value);
        self
    }

    /// Set the cost_per_order field (chainable)
    pub fn with_cost_per_order(mut self, value: Decimal) -> Self {
        self.cost_per_order = Some(value);
        self
    }

    /// Set the profit_per_kg field (chainable)
    pub fn with_profit_per_kg(mut self, value: Decimal) -> Self {
        self.profit_per_kg = Some(value);
        self
    }

    /// Set the profit_per_order field (chainable)
    pub fn with_profit_per_order(mut self, value: Decimal) -> Self {
        self.profit_per_order = Some(value);
        self
    }

    /// Set the pnl_report_url field (chainable)
    pub fn with_pnl_report_url(mut self, value: String) -> Self {
        self.pnl_report_url = Some(value);
        self
    }

    /// Set the report_generated_at field (chainable)
    pub fn with_report_generated_at(mut self, value: DateTime<Utc>) -> Self {
        self.report_generated_at = Some(value);
        self
    }

    /// Set the closed_at field (chainable)
    pub fn with_closed_at(mut self, value: DateTime<Utc>) -> Self {
        self.closed_at = Some(value);
        self
    }

    /// Set the closed_by field (chainable)
    pub fn with_closed_by(mut self, value: Uuid) -> Self {
        self.closed_by = Some(value);
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
                "period_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.period_type = v; }
                }
                "period_start" => {
                    if let Ok(v) = serde_json::from_value(value) { self.period_start = v; }
                }
                "period_end" => {
                    if let Ok(v) = serde_json::from_value(value) { self.period_end = v; }
                }
                "fiscal_year" => {
                    if let Ok(v) = serde_json::from_value(value) { self.fiscal_year = v; }
                }
                "fiscal_month" => {
                    if let Ok(v) = serde_json::from_value(value) { self.fiscal_month = v; }
                }
                "gross_revenue" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gross_revenue = v; }
                }
                "discounts_given" => {
                    if let Ok(v) = serde_json::from_value(value) { self.discounts_given = v; }
                }
                "refunds_issued" => {
                    if let Ok(v) = serde_json::from_value(value) { self.refunds_issued = v; }
                }
                "net_revenue" => {
                    if let Ok(v) = serde_json::from_value(value) { self.net_revenue = v; }
                }
                "cogs" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cogs = v; }
                }
                "labor_cost" => {
                    if let Ok(v) = serde_json::from_value(value) { self.labor_cost = v; }
                }
                "material_cost" => {
                    if let Ok(v) = serde_json::from_value(value) { self.material_cost = v; }
                }
                "utility_expense" => {
                    if let Ok(v) = serde_json::from_value(value) { self.utility_expense = v; }
                }
                "maintenance_expense" => {
                    if let Ok(v) = serde_json::from_value(value) { self.maintenance_expense = v; }
                }
                "delivery_expense" => {
                    if let Ok(v) = serde_json::from_value(value) { self.delivery_expense = v; }
                }
                "other_expense" => {
                    if let Ok(v) = serde_json::from_value(value) { self.other_expense = v; }
                }
                "platform_commission" => {
                    if let Ok(v) = serde_json::from_value(value) { self.platform_commission = v; }
                }
                "transaction_fees" => {
                    if let Ok(v) = serde_json::from_value(value) { self.transaction_fees = v; }
                }
                "subscription_fees" => {
                    if let Ok(v) = serde_json::from_value(value) { self.subscription_fees = v; }
                }
                "gross_profit" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gross_profit = v; }
                }
                "operating_profit" => {
                    if let Ok(v) = serde_json::from_value(value) { self.operating_profit = v; }
                }
                "net_profit" => {
                    if let Ok(v) = serde_json::from_value(value) { self.net_profit = v; }
                }
                "gross_margin_percent" => {
                    if let Ok(v) = serde_json::from_value(value) { self.gross_margin_percent = v; }
                }
                "operating_margin_percent" => {
                    if let Ok(v) = serde_json::from_value(value) { self.operating_margin_percent = v; }
                }
                "net_margin_percent" => {
                    if let Ok(v) = serde_json::from_value(value) { self.net_margin_percent = v; }
                }
                "revenue_growth_percent" => {
                    if let Ok(v) = serde_json::from_value(value) { self.revenue_growth_percent = v; }
                }
                "profit_growth_percent" => {
                    if let Ok(v) = serde_json::from_value(value) { self.profit_growth_percent = v; }
                }
                "average_order_value" => {
                    if let Ok(v) = serde_json::from_value(value) { self.average_order_value = v; }
                }
                "cost_per_kg" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cost_per_kg = v; }
                }
                "cost_per_order" => {
                    if let Ok(v) = serde_json::from_value(value) { self.cost_per_order = v; }
                }
                "profit_per_kg" => {
                    if let Ok(v) = serde_json::from_value(value) { self.profit_per_kg = v; }
                }
                "profit_per_order" => {
                    if let Ok(v) = serde_json::from_value(value) { self.profit_per_order = v; }
                }
                "pnl_report_url" => {
                    if let Ok(v) = serde_json::from_value(value) { self.pnl_report_url = v; }
                }
                "report_generated_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.report_generated_at = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "closed_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.closed_at = v; }
                }
                "closed_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.closed_by = v; }
                }
                "currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.currency = v; }
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

impl super::Entity for FinancialPeriod {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "FinancialPeriod"
    }
}

impl backbone_core::PersistentEntity for FinancialPeriod {
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

impl backbone_orm::EntityRepoMeta for FinancialPeriod {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("provider_id".to_string(), "uuid".to_string());
        m.insert("period_type".to_string(), "financial_period_type".to_string());
        m.insert("status".to_string(), "financial_period_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["currency"]
    }
}

/// Builder for FinancialPeriod entity
///
/// Provides a fluent API for constructing FinancialPeriod instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct FinancialPeriodBuilder {
    provider_id: Option<Uuid>,
    period_type: Option<FinancialPeriodType>,
    period_start: Option<DateTime<Utc>>,
    period_end: Option<DateTime<Utc>>,
    fiscal_year: Option<i32>,
    fiscal_month: Option<i32>,
    gross_revenue: Option<Decimal>,
    discounts_given: Option<Decimal>,
    refunds_issued: Option<Decimal>,
    net_revenue: Option<Decimal>,
    cogs: Option<Decimal>,
    labor_cost: Option<Decimal>,
    material_cost: Option<Decimal>,
    utility_expense: Option<Decimal>,
    maintenance_expense: Option<Decimal>,
    delivery_expense: Option<Decimal>,
    other_expense: Option<Decimal>,
    platform_commission: Option<Decimal>,
    transaction_fees: Option<Decimal>,
    subscription_fees: Option<Decimal>,
    gross_profit: Option<Decimal>,
    operating_profit: Option<Decimal>,
    net_profit: Option<Decimal>,
    gross_margin_percent: Option<Decimal>,
    operating_margin_percent: Option<Decimal>,
    net_margin_percent: Option<Decimal>,
    revenue_growth_percent: Option<Decimal>,
    profit_growth_percent: Option<Decimal>,
    average_order_value: Option<Decimal>,
    cost_per_kg: Option<Decimal>,
    cost_per_order: Option<Decimal>,
    profit_per_kg: Option<Decimal>,
    profit_per_order: Option<Decimal>,
    pnl_report_url: Option<String>,
    report_generated_at: Option<DateTime<Utc>>,
    status: Option<FinancialPeriodStatus>,
    closed_at: Option<DateTime<Utc>>,
    closed_by: Option<Uuid>,
    currency: Option<String>,
    notes: Option<String>,
    data: Option<serde_json::Value>,
}

impl FinancialPeriodBuilder {
    /// Set the provider_id field (required)
    pub fn provider_id(mut self, value: Uuid) -> Self {
        self.provider_id = Some(value);
        self
    }

    /// Set the period_type field (default: `FinancialPeriodType::default()`)
    pub fn period_type(mut self, value: FinancialPeriodType) -> Self {
        self.period_type = Some(value);
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

    /// Set the fiscal_year field (required)
    pub fn fiscal_year(mut self, value: i32) -> Self {
        self.fiscal_year = Some(value);
        self
    }

    /// Set the fiscal_month field (optional)
    pub fn fiscal_month(mut self, value: i32) -> Self {
        self.fiscal_month = Some(value);
        self
    }

    /// Set the gross_revenue field (default: `Decimal::from(0)`)
    pub fn gross_revenue(mut self, value: Decimal) -> Self {
        self.gross_revenue = Some(value);
        self
    }

    /// Set the discounts_given field (default: `Decimal::from(0)`)
    pub fn discounts_given(mut self, value: Decimal) -> Self {
        self.discounts_given = Some(value);
        self
    }

    /// Set the refunds_issued field (default: `Decimal::from(0)`)
    pub fn refunds_issued(mut self, value: Decimal) -> Self {
        self.refunds_issued = Some(value);
        self
    }

    /// Set the net_revenue field (default: `Decimal::from(0)`)
    pub fn net_revenue(mut self, value: Decimal) -> Self {
        self.net_revenue = Some(value);
        self
    }

    /// Set the cogs field (default: `Decimal::from(0)`)
    pub fn cogs(mut self, value: Decimal) -> Self {
        self.cogs = Some(value);
        self
    }

    /// Set the labor_cost field (default: `Decimal::from(0)`)
    pub fn labor_cost(mut self, value: Decimal) -> Self {
        self.labor_cost = Some(value);
        self
    }

    /// Set the material_cost field (default: `Decimal::from(0)`)
    pub fn material_cost(mut self, value: Decimal) -> Self {
        self.material_cost = Some(value);
        self
    }

    /// Set the utility_expense field (default: `Decimal::from(0)`)
    pub fn utility_expense(mut self, value: Decimal) -> Self {
        self.utility_expense = Some(value);
        self
    }

    /// Set the maintenance_expense field (default: `Decimal::from(0)`)
    pub fn maintenance_expense(mut self, value: Decimal) -> Self {
        self.maintenance_expense = Some(value);
        self
    }

    /// Set the delivery_expense field (default: `Decimal::from(0)`)
    pub fn delivery_expense(mut self, value: Decimal) -> Self {
        self.delivery_expense = Some(value);
        self
    }

    /// Set the other_expense field (default: `Decimal::from(0)`)
    pub fn other_expense(mut self, value: Decimal) -> Self {
        self.other_expense = Some(value);
        self
    }

    /// Set the platform_commission field (default: `Decimal::from(0)`)
    pub fn platform_commission(mut self, value: Decimal) -> Self {
        self.platform_commission = Some(value);
        self
    }

    /// Set the transaction_fees field (default: `Decimal::from(0)`)
    pub fn transaction_fees(mut self, value: Decimal) -> Self {
        self.transaction_fees = Some(value);
        self
    }

    /// Set the subscription_fees field (default: `Decimal::from(0)`)
    pub fn subscription_fees(mut self, value: Decimal) -> Self {
        self.subscription_fees = Some(value);
        self
    }

    /// Set the gross_profit field (default: `Decimal::from(0)`)
    pub fn gross_profit(mut self, value: Decimal) -> Self {
        self.gross_profit = Some(value);
        self
    }

    /// Set the operating_profit field (default: `Decimal::from(0)`)
    pub fn operating_profit(mut self, value: Decimal) -> Self {
        self.operating_profit = Some(value);
        self
    }

    /// Set the net_profit field (default: `Decimal::from(0)`)
    pub fn net_profit(mut self, value: Decimal) -> Self {
        self.net_profit = Some(value);
        self
    }

    /// Set the gross_margin_percent field (optional)
    pub fn gross_margin_percent(mut self, value: Decimal) -> Self {
        self.gross_margin_percent = Some(value);
        self
    }

    /// Set the operating_margin_percent field (optional)
    pub fn operating_margin_percent(mut self, value: Decimal) -> Self {
        self.operating_margin_percent = Some(value);
        self
    }

    /// Set the net_margin_percent field (optional)
    pub fn net_margin_percent(mut self, value: Decimal) -> Self {
        self.net_margin_percent = Some(value);
        self
    }

    /// Set the revenue_growth_percent field (optional)
    pub fn revenue_growth_percent(mut self, value: Decimal) -> Self {
        self.revenue_growth_percent = Some(value);
        self
    }

    /// Set the profit_growth_percent field (optional)
    pub fn profit_growth_percent(mut self, value: Decimal) -> Self {
        self.profit_growth_percent = Some(value);
        self
    }

    /// Set the average_order_value field (optional)
    pub fn average_order_value(mut self, value: Decimal) -> Self {
        self.average_order_value = Some(value);
        self
    }

    /// Set the cost_per_kg field (optional)
    pub fn cost_per_kg(mut self, value: Decimal) -> Self {
        self.cost_per_kg = Some(value);
        self
    }

    /// Set the cost_per_order field (optional)
    pub fn cost_per_order(mut self, value: Decimal) -> Self {
        self.cost_per_order = Some(value);
        self
    }

    /// Set the profit_per_kg field (optional)
    pub fn profit_per_kg(mut self, value: Decimal) -> Self {
        self.profit_per_kg = Some(value);
        self
    }

    /// Set the profit_per_order field (optional)
    pub fn profit_per_order(mut self, value: Decimal) -> Self {
        self.profit_per_order = Some(value);
        self
    }

    /// Set the pnl_report_url field (optional)
    pub fn pnl_report_url(mut self, value: String) -> Self {
        self.pnl_report_url = Some(value);
        self
    }

    /// Set the report_generated_at field (optional)
    pub fn report_generated_at(mut self, value: DateTime<Utc>) -> Self {
        self.report_generated_at = Some(value);
        self
    }

    /// Set the status field (default: `FinancialPeriodStatus::default()`)
    pub fn status(mut self, value: FinancialPeriodStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the closed_at field (optional)
    pub fn closed_at(mut self, value: DateTime<Utc>) -> Self {
        self.closed_at = Some(value);
        self
    }

    /// Set the closed_by field (optional)
    pub fn closed_by(mut self, value: Uuid) -> Self {
        self.closed_by = Some(value);
        self
    }

    /// Set the currency field (default: `"IDR".to_string()`)
    pub fn currency(mut self, value: String) -> Self {
        self.currency = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Set the data field (default: `serde_json::json!({"total_orders":0,"total_revenue":0,"total_weight_kg":0})`)
    pub fn data(mut self, value: serde_json::Value) -> Self {
        self.data = Some(value);
        self
    }

    /// Build the FinancialPeriod entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<FinancialPeriod, String> {
        let provider_id = self.provider_id.ok_or_else(|| "provider_id is required".to_string())?;
        let period_start = self.period_start.ok_or_else(|| "period_start is required".to_string())?;
        let period_end = self.period_end.ok_or_else(|| "period_end is required".to_string())?;
        let fiscal_year = self.fiscal_year.ok_or_else(|| "fiscal_year is required".to_string())?;

        Ok(FinancialPeriod {
            id: Uuid::new_v4(),
            provider_id,
            period_type: self.period_type.unwrap_or(FinancialPeriodType::default()),
            period_start,
            period_end,
            fiscal_year,
            fiscal_month: self.fiscal_month,
            gross_revenue: self.gross_revenue.unwrap_or(Decimal::from(0)),
            discounts_given: self.discounts_given.unwrap_or(Decimal::from(0)),
            refunds_issued: self.refunds_issued.unwrap_or(Decimal::from(0)),
            net_revenue: self.net_revenue.unwrap_or(Decimal::from(0)),
            cogs: self.cogs.unwrap_or(Decimal::from(0)),
            labor_cost: self.labor_cost.unwrap_or(Decimal::from(0)),
            material_cost: self.material_cost.unwrap_or(Decimal::from(0)),
            utility_expense: self.utility_expense.unwrap_or(Decimal::from(0)),
            maintenance_expense: self.maintenance_expense.unwrap_or(Decimal::from(0)),
            delivery_expense: self.delivery_expense.unwrap_or(Decimal::from(0)),
            other_expense: self.other_expense.unwrap_or(Decimal::from(0)),
            platform_commission: self.platform_commission.unwrap_or(Decimal::from(0)),
            transaction_fees: self.transaction_fees.unwrap_or(Decimal::from(0)),
            subscription_fees: self.subscription_fees.unwrap_or(Decimal::from(0)),
            gross_profit: self.gross_profit.unwrap_or(Decimal::from(0)),
            operating_profit: self.operating_profit.unwrap_or(Decimal::from(0)),
            net_profit: self.net_profit.unwrap_or(Decimal::from(0)),
            gross_margin_percent: self.gross_margin_percent,
            operating_margin_percent: self.operating_margin_percent,
            net_margin_percent: self.net_margin_percent,
            revenue_growth_percent: self.revenue_growth_percent,
            profit_growth_percent: self.profit_growth_percent,
            average_order_value: self.average_order_value,
            cost_per_kg: self.cost_per_kg,
            cost_per_order: self.cost_per_order,
            profit_per_kg: self.profit_per_kg,
            profit_per_order: self.profit_per_order,
            pnl_report_url: self.pnl_report_url,
            report_generated_at: self.report_generated_at,
            status: self.status.unwrap_or(FinancialPeriodStatus::default()),
            closed_at: self.closed_at,
            closed_by: self.closed_by,
            currency: self.currency.unwrap_or("IDR".to_string()),
            notes: self.notes,
            data: self.data.unwrap_or(serde_json::json!({"total_orders":0,"total_revenue":0,"total_weight_kg":0})),
            metadata: AuditMetadata::default(),
        })
    }
}

use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "refund_reason", rename_all = "snake_case")]
pub enum RefundReason {
    CustomerRequest,
    OrderCancelled,
    ServiceIssue,
    ItemDamaged,
    ItemLost,
    Overcharge,
    DuplicatePayment,
    ProviderUnable,
    Other,
}

impl std::fmt::Display for RefundReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CustomerRequest => write!(f, "customer_request"),
            Self::OrderCancelled => write!(f, "order_cancelled"),
            Self::ServiceIssue => write!(f, "service_issue"),
            Self::ItemDamaged => write!(f, "item_damaged"),
            Self::ItemLost => write!(f, "item_lost"),
            Self::Overcharge => write!(f, "overcharge"),
            Self::DuplicatePayment => write!(f, "duplicate_payment"),
            Self::ProviderUnable => write!(f, "provider_unable"),
            Self::Other => write!(f, "other"),
        }
    }
}

impl FromStr for RefundReason {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "customer_request" => Ok(Self::CustomerRequest),
            "order_cancelled" => Ok(Self::OrderCancelled),
            "service_issue" => Ok(Self::ServiceIssue),
            "item_damaged" => Ok(Self::ItemDamaged),
            "item_lost" => Ok(Self::ItemLost),
            "overcharge" => Ok(Self::Overcharge),
            "duplicate_payment" => Ok(Self::DuplicatePayment),
            "provider_unable" => Ok(Self::ProviderUnable),
            "other" => Ok(Self::Other),
            _ => Err(format!("Unknown RefundReason variant: {}", s)),
        }
    }
}

impl Default for RefundReason {
    fn default() -> Self {
        Self::CustomerRequest
    }
}

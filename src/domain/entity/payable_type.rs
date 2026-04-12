use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payable_type", rename_all = "snake_case")]
pub enum PayableType {
    Order,
    Subscription,
    Registration,
    CorporateInvoice,
}

impl std::fmt::Display for PayableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order => write!(f, "order"),
            Self::Subscription => write!(f, "subscription"),
            Self::Registration => write!(f, "registration"),
            Self::CorporateInvoice => write!(f, "corporate_invoice"),
        }
    }
}

impl FromStr for PayableType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "order" => Ok(Self::Order),
            "subscription" => Ok(Self::Subscription),
            "registration" => Ok(Self::Registration),
            "corporate_invoice" => Ok(Self::CorporateInvoice),
            _ => Err(format!("Unknown PayableType variant: {}", s)),
        }
    }
}

impl Default for PayableType {
    fn default() -> Self {
        Self::Order
    }
}

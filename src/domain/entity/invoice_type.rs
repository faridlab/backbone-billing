use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "invoice_type", rename_all = "snake_case")]
pub enum InvoiceType {
    Order,
    Subscription,
    Registration,
    Settlement,
    Corporate,
}

impl std::fmt::Display for InvoiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order => write!(f, "order"),
            Self::Subscription => write!(f, "subscription"),
            Self::Registration => write!(f, "registration"),
            Self::Settlement => write!(f, "settlement"),
            Self::Corporate => write!(f, "corporate"),
        }
    }
}

impl FromStr for InvoiceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "order" => Ok(Self::Order),
            "subscription" => Ok(Self::Subscription),
            "registration" => Ok(Self::Registration),
            "settlement" => Ok(Self::Settlement),
            "corporate" => Ok(Self::Corporate),
            _ => Err(format!("Unknown InvoiceType variant: {}", s)),
        }
    }
}

impl Default for InvoiceType {
    fn default() -> Self {
        Self::Order
    }
}

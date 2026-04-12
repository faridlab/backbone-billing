use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "invoiceable_type", rename_all = "snake_case")]
pub enum InvoiceableType {
    Order,
    Settlement,
    Subscription,
    CorporateInvoice,
}

impl std::fmt::Display for InvoiceableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order => write!(f, "order"),
            Self::Settlement => write!(f, "settlement"),
            Self::Subscription => write!(f, "subscription"),
            Self::CorporateInvoice => write!(f, "corporate_invoice"),
        }
    }
}

impl FromStr for InvoiceableType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "order" => Ok(Self::Order),
            "settlement" => Ok(Self::Settlement),
            "subscription" => Ok(Self::Subscription),
            "corporate_invoice" => Ok(Self::CorporateInvoice),
            _ => Err(format!("Unknown InvoiceableType variant: {}", s)),
        }
    }
}

impl Default for InvoiceableType {
    fn default() -> Self {
        Self::Order
    }
}

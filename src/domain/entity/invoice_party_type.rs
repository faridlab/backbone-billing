use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "invoice_party_type", rename_all = "snake_case")]
pub enum InvoicePartyType {
    Platform,
    Provider,
    Customer,
    Corporate,
}

impl std::fmt::Display for InvoicePartyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Platform => write!(f, "platform"),
            Self::Provider => write!(f, "provider"),
            Self::Customer => write!(f, "customer"),
            Self::Corporate => write!(f, "corporate"),
        }
    }
}

impl FromStr for InvoicePartyType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "platform" => Ok(Self::Platform),
            "provider" => Ok(Self::Provider),
            "customer" => Ok(Self::Customer),
            "corporate" => Ok(Self::Corporate),
            _ => Err(format!("Unknown InvoicePartyType variant: {}", s)),
        }
    }
}

impl Default for InvoicePartyType {
    fn default() -> Self {
        Self::Platform
    }
}

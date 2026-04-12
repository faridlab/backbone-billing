use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "invoice_party_role", rename_all = "snake_case")]
pub enum InvoicePartyRole {
    Issuer,
    Recipient,
}

impl std::fmt::Display for InvoicePartyRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Issuer => write!(f, "issuer"),
            Self::Recipient => write!(f, "recipient"),
        }
    }
}

impl FromStr for InvoicePartyRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "issuer" => Ok(Self::Issuer),
            "recipient" => Ok(Self::Recipient),
            _ => Err(format!("Unknown InvoicePartyRole variant: {}", s)),
        }
    }
}

impl Default for InvoicePartyRole {
    fn default() -> Self {
        Self::Issuer
    }
}

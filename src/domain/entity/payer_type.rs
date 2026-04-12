use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payer_type", rename_all = "snake_case")]
pub enum PayerType {
    Customer,
    Corporate,
    Provider,
}

impl std::fmt::Display for PayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Customer => write!(f, "customer"),
            Self::Corporate => write!(f, "corporate"),
            Self::Provider => write!(f, "provider"),
        }
    }
}

impl FromStr for PayerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "customer" => Ok(Self::Customer),
            "corporate" => Ok(Self::Corporate),
            "provider" => Ok(Self::Provider),
            _ => Err(format!("Unknown PayerType variant: {}", s)),
        }
    }
}

impl Default for PayerType {
    fn default() -> Self {
        Self::Customer
    }
}

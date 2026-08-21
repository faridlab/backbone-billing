use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_term_line_value", rename_all = "snake_case")]
pub enum PaymentTermLineValue {
    Balance,
    Percent,
    Fixed,
}

impl std::fmt::Display for PaymentTermLineValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Balance => write!(f, "balance"),
            Self::Percent => write!(f, "percent"),
            Self::Fixed => write!(f, "fixed"),
        }
    }
}

impl FromStr for PaymentTermLineValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "balance" => Ok(Self::Balance),
            "percent" => Ok(Self::Percent),
            "fixed" => Ok(Self::Fixed),
            _ => Err(format!("Unknown PaymentTermLineValue variant: {}", s)),
        }
    }
}

impl Default for PaymentTermLineValue {
    fn default() -> Self {
        Self::Balance
    }
}

use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "refund_method", rename_all = "snake_case")]
pub enum RefundMethod {
    Original,
    BankTransfer,
    Ewallet,
}

impl std::fmt::Display for RefundMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Original => write!(f, "original"),
            Self::BankTransfer => write!(f, "bank_transfer"),
            Self::Ewallet => write!(f, "ewallet"),
        }
    }
}

impl FromStr for RefundMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "original" => Ok(Self::Original),
            "bank_transfer" => Ok(Self::BankTransfer),
            "ewallet" => Ok(Self::Ewallet),
            _ => Err(format!("Unknown RefundMethod variant: {}", s)),
        }
    }
}

impl Default for RefundMethod {
    fn default() -> Self {
        Self::Original
    }
}

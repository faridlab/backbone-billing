use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "bank_account_type", rename_all = "snake_case")]
pub enum BankAccountType {
    Checking,
    Savings,
    Business,
}

impl std::fmt::Display for BankAccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Checking => write!(f, "checking"),
            Self::Savings => write!(f, "savings"),
            Self::Business => write!(f, "business"),
        }
    }
}

impl FromStr for BankAccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "checking" => Ok(Self::Checking),
            "savings" => Ok(Self::Savings),
            "business" => Ok(Self::Business),
            _ => Err(format!("Unknown BankAccountType variant: {}", s)),
        }
    }
}

impl Default for BankAccountType {
    fn default() -> Self {
        Self::Checking
    }
}

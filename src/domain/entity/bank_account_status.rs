use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "bank_account_status", rename_all = "snake_case")]
pub enum BankAccountStatus {
    Pending,
    Verified,
    Rejected,
    Suspended,
    Inactive,
}

impl std::fmt::Display for BankAccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Verified => write!(f, "verified"),
            Self::Rejected => write!(f, "rejected"),
            Self::Suspended => write!(f, "suspended"),
            Self::Inactive => write!(f, "inactive"),
        }
    }
}

impl FromStr for BankAccountStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "verified" => Ok(Self::Verified),
            "rejected" => Ok(Self::Rejected),
            "suspended" => Ok(Self::Suspended),
            "inactive" => Ok(Self::Inactive),
            _ => Err(format!("Unknown BankAccountStatus variant: {}", s)),
        }
    }
}

impl Default for BankAccountStatus {
    fn default() -> Self {
        Self::Pending
    }
}

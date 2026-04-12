use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "earning_status", rename_all = "snake_case")]
pub enum EarningStatus {
    Pending,
    Confirmed,
    Paid,
    Cancelled,
    Disputed,
}

impl std::fmt::Display for EarningStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Confirmed => write!(f, "confirmed"),
            Self::Paid => write!(f, "paid"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Disputed => write!(f, "disputed"),
        }
    }
}

impl FromStr for EarningStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "confirmed" => Ok(Self::Confirmed),
            "paid" => Ok(Self::Paid),
            "cancelled" => Ok(Self::Cancelled),
            "disputed" => Ok(Self::Disputed),
            _ => Err(format!("Unknown EarningStatus variant: {}", s)),
        }
    }
}

impl Default for EarningStatus {
    fn default() -> Self {
        Self::Pending
    }
}

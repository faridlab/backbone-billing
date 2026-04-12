use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "financial_period_status", rename_all = "snake_case")]
pub enum FinancialPeriodStatus {
    Open,
    Closing,
    Closed,
    Locked,
}

impl std::fmt::Display for FinancialPeriodStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Closing => write!(f, "closing"),
            Self::Closed => write!(f, "closed"),
            Self::Locked => write!(f, "locked"),
        }
    }
}

impl FromStr for FinancialPeriodStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "closing" => Ok(Self::Closing),
            "closed" => Ok(Self::Closed),
            "locked" => Ok(Self::Locked),
            _ => Err(format!("Unknown FinancialPeriodStatus variant: {}", s)),
        }
    }
}

impl Default for FinancialPeriodStatus {
    fn default() -> Self {
        Self::Open
    }
}

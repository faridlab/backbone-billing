use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "settlement_frequency", rename_all = "snake_case")]
pub enum SettlementFrequency {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
}

impl std::fmt::Display for SettlementFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daily => write!(f, "daily"),
            Self::Weekly => write!(f, "weekly"),
            Self::Biweekly => write!(f, "biweekly"),
            Self::Monthly => write!(f, "monthly"),
        }
    }
}

impl FromStr for SettlementFrequency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "biweekly" => Ok(Self::Biweekly),
            "monthly" => Ok(Self::Monthly),
            _ => Err(format!("Unknown SettlementFrequency variant: {}", s)),
        }
    }
}

impl Default for SettlementFrequency {
    fn default() -> Self {
        Self::Daily
    }
}

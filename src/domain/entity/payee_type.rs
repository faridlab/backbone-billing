use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payee_type", rename_all = "snake_case")]
pub enum PayeeType {
    Provider,
    Platform,
}

impl std::fmt::Display for PayeeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider => write!(f, "provider"),
            Self::Platform => write!(f, "platform"),
        }
    }
}

impl FromStr for PayeeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "provider" => Ok(Self::Provider),
            "platform" => Ok(Self::Platform),
            _ => Err(format!("Unknown PayeeType variant: {}", s)),
        }
    }
}

impl Default for PayeeType {
    fn default() -> Self {
        Self::Provider
    }
}

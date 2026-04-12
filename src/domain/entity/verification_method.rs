use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "verification_method", rename_all = "snake_case")]
pub enum VerificationMethod {
    MicroDeposit,
    Document,
    Instant,
    Manual,
}

impl std::fmt::Display for VerificationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MicroDeposit => write!(f, "micro_deposit"),
            Self::Document => write!(f, "document"),
            Self::Instant => write!(f, "instant"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

impl FromStr for VerificationMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "micro_deposit" => Ok(Self::MicroDeposit),
            "document" => Ok(Self::Document),
            "instant" => Ok(Self::Instant),
            "manual" => Ok(Self::Manual),
            _ => Err(format!("Unknown VerificationMethod variant: {}", s)),
        }
    }
}

impl Default for VerificationMethod {
    fn default() -> Self {
        Self::MicroDeposit
    }
}

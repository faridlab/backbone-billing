use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_gateway", rename_all = "snake_case")]
pub enum PaymentGateway {
    Xendit,
    Midtrans,
    Doku,
    Stripe,
    Manual,
}

impl std::fmt::Display for PaymentGateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xendit => write!(f, "xendit"),
            Self::Midtrans => write!(f, "midtrans"),
            Self::Doku => write!(f, "doku"),
            Self::Stripe => write!(f, "stripe"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

impl FromStr for PaymentGateway {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xendit" => Ok(Self::Xendit),
            "midtrans" => Ok(Self::Midtrans),
            "doku" => Ok(Self::Doku),
            "stripe" => Ok(Self::Stripe),
            "manual" => Ok(Self::Manual),
            _ => Err(format!("Unknown PaymentGateway variant: {}", s)),
        }
    }
}

impl Default for PaymentGateway {
    fn default() -> Self {
        Self::Xendit
    }
}

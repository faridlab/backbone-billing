use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_fee_type", rename_all = "snake_case")]
pub enum PaymentFeeType {
    ServiceFee,
    GatewayFee,
    Tax,
}

impl std::fmt::Display for PaymentFeeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceFee => write!(f, "service_fee"),
            Self::GatewayFee => write!(f, "gateway_fee"),
            Self::Tax => write!(f, "tax"),
        }
    }
}

impl FromStr for PaymentFeeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "service_fee" => Ok(Self::ServiceFee),
            "gateway_fee" => Ok(Self::GatewayFee),
            "tax" => Ok(Self::Tax),
            _ => Err(format!("Unknown PaymentFeeType variant: {}", s)),
        }
    }
}

impl Default for PaymentFeeType {
    fn default() -> Self {
        Self::ServiceFee
    }
}

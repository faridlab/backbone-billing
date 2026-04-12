use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_transaction_status", rename_all = "snake_case")]
pub enum PaymentTransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
    Cancelled,
    Refunded,
    PartiallyRefunded,
}

impl std::fmt::Display for PaymentTransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Processing => write!(f, "processing"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Expired => write!(f, "expired"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Refunded => write!(f, "refunded"),
            Self::PartiallyRefunded => write!(f, "partially_refunded"),
        }
    }
}

impl FromStr for PaymentTransactionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "expired" => Ok(Self::Expired),
            "cancelled" => Ok(Self::Cancelled),
            "refunded" => Ok(Self::Refunded),
            "partially_refunded" => Ok(Self::PartiallyRefunded),
            _ => Err(format!("Unknown PaymentTransactionStatus variant: {}", s)),
        }
    }
}

impl Default for PaymentTransactionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

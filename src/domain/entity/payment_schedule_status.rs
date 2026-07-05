use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_schedule_status", rename_all = "snake_case")]
pub enum PaymentScheduleStatus {
    Unpaid,
    PartiallyPaid,
    Paid,
    Overdue,
}

impl std::fmt::Display for PaymentScheduleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unpaid => write!(f, "unpaid"),
            Self::PartiallyPaid => write!(f, "partially_paid"),
            Self::Paid => write!(f, "paid"),
            Self::Overdue => write!(f, "overdue"),
        }
    }
}

impl FromStr for PaymentScheduleStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unpaid" => Ok(Self::Unpaid),
            "partially_paid" => Ok(Self::PartiallyPaid),
            "paid" => Ok(Self::Paid),
            "overdue" => Ok(Self::Overdue),
            _ => Err(format!("Unknown PaymentScheduleStatus variant: {}", s)),
        }
    }
}

impl Default for PaymentScheduleStatus {
    fn default() -> Self {
        Self::Unpaid
    }
}

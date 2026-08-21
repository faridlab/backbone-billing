use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_term_delay_type", rename_all = "snake_case")]
pub enum PaymentTermDelayType {
    Days,
    DayFollowingMonth,
    DayCurrentMonth,
}

impl std::fmt::Display for PaymentTermDelayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Days => write!(f, "days"),
            Self::DayFollowingMonth => write!(f, "day_following_month"),
            Self::DayCurrentMonth => write!(f, "day_current_month"),
        }
    }
}

impl FromStr for PaymentTermDelayType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "days" => Ok(Self::Days),
            "day_following_month" => Ok(Self::DayFollowingMonth),
            "day_current_month" => Ok(Self::DayCurrentMonth),
            _ => Err(format!("Unknown PaymentTermDelayType variant: {}", s)),
        }
    }
}

impl Default for PaymentTermDelayType {
    fn default() -> Self {
        Self::Days
    }
}

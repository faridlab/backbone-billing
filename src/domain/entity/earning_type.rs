use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "earning_type", rename_all = "snake_case")]
pub enum EarningType {
    Pickup,
    Delivery,
    PickupDelivery,
    Bonus,
    Tip,
    Incentive,
    Adjustment,
    Referral,
}

impl std::fmt::Display for EarningType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pickup => write!(f, "pickup"),
            Self::Delivery => write!(f, "delivery"),
            Self::PickupDelivery => write!(f, "pickup_delivery"),
            Self::Bonus => write!(f, "bonus"),
            Self::Tip => write!(f, "tip"),
            Self::Incentive => write!(f, "incentive"),
            Self::Adjustment => write!(f, "adjustment"),
            Self::Referral => write!(f, "referral"),
        }
    }
}

impl FromStr for EarningType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pickup" => Ok(Self::Pickup),
            "delivery" => Ok(Self::Delivery),
            "pickup_delivery" => Ok(Self::PickupDelivery),
            "bonus" => Ok(Self::Bonus),
            "tip" => Ok(Self::Tip),
            "incentive" => Ok(Self::Incentive),
            "adjustment" => Ok(Self::Adjustment),
            "referral" => Ok(Self::Referral),
            _ => Err(format!("Unknown EarningType variant: {}", s)),
        }
    }
}

impl Default for EarningType {
    fn default() -> Self {
        Self::PickupDelivery
    }
}

use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "invoice_status", rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Viewed,
    Paid,
    PartiallyPaid,
    Overdue,
    Cancelled,
    Void,
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Sent => write!(f, "sent"),
            Self::Viewed => write!(f, "viewed"),
            Self::Paid => write!(f, "paid"),
            Self::PartiallyPaid => write!(f, "partially_paid"),
            Self::Overdue => write!(f, "overdue"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Void => write!(f, "void"),
        }
    }
}

impl FromStr for InvoiceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "sent" => Ok(Self::Sent),
            "viewed" => Ok(Self::Viewed),
            "paid" => Ok(Self::Paid),
            "partially_paid" => Ok(Self::PartiallyPaid),
            "overdue" => Ok(Self::Overdue),
            "cancelled" => Ok(Self::Cancelled),
            "void" => Ok(Self::Void),
            _ => Err(format!("Unknown InvoiceStatus variant: {}", s)),
        }
    }
}

impl Default for InvoiceStatus {
    fn default() -> Self {
        Self::Draft
    }
}

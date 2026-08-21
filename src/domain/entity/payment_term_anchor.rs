use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_term_anchor", rename_all = "snake_case")]
pub enum PaymentTermAnchor {
    InvoiceDate,
    EndOfInvoiceMonth,
}

impl std::fmt::Display for PaymentTermAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvoiceDate => write!(f, "invoice_date"),
            Self::EndOfInvoiceMonth => write!(f, "end_of_invoice_month"),
        }
    }
}

impl FromStr for PaymentTermAnchor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "invoice_date" => Ok(Self::InvoiceDate),
            "end_of_invoice_month" => Ok(Self::EndOfInvoiceMonth),
            _ => Err(format!("Unknown PaymentTermAnchor variant: {}", s)),
        }
    }
}

impl Default for PaymentTermAnchor {
    fn default() -> Self {
        Self::InvoiceDate
    }
}

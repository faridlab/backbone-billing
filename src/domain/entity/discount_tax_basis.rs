use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "discount_tax_basis", rename_all = "snake_case")]
pub enum DiscountTaxBasis {
    Included,
    ReducedPrice,
}

impl std::fmt::Display for DiscountTaxBasis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Included => write!(f, "included"),
            Self::ReducedPrice => write!(f, "reduced_price"),
        }
    }
}

impl FromStr for DiscountTaxBasis {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "included" => Ok(Self::Included),
            "reduced_price" => Ok(Self::ReducedPrice),
            _ => Err(format!("Unknown DiscountTaxBasis variant: {}", s)),
        }
    }
}

impl Default for DiscountTaxBasis {
    fn default() -> Self {
        Self::Included
    }
}

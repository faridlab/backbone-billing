use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "tax_basis", rename_all = "snake_case")]
pub enum TaxBasis {
    Output,
    Input,
    Withholding,
}

impl std::fmt::Display for TaxBasis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output => write!(f, "output"),
            Self::Input => write!(f, "input"),
            Self::Withholding => write!(f, "withholding"),
        }
    }
}

impl FromStr for TaxBasis {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "output" => Ok(Self::Output),
            "input" => Ok(Self::Input),
            "withholding" => Ok(Self::Withholding),
            _ => Err(format!("Unknown TaxBasis variant: {}", s)),
        }
    }
}

impl Default for TaxBasis {
    fn default() -> Self {
        Self::Output
    }
}

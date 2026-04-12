use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "payment_channel", rename_all = "snake_case")]
pub enum PaymentChannel {
    Qris,
    VaBca,
    VaBni,
    VaBri,
    VaMandiri,
    VaPermata,
    VaCimb,
    Ovo,
    Gopay,
    Dana,
    Shopeepay,
    Linkaja,
    CreditCard,
    DebitCard,
    Cash,
    BankTransfer,
}

impl std::fmt::Display for PaymentChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Qris => write!(f, "qris"),
            Self::VaBca => write!(f, "va_bca"),
            Self::VaBni => write!(f, "va_bni"),
            Self::VaBri => write!(f, "va_bri"),
            Self::VaMandiri => write!(f, "va_mandiri"),
            Self::VaPermata => write!(f, "va_permata"),
            Self::VaCimb => write!(f, "va_cimb"),
            Self::Ovo => write!(f, "ovo"),
            Self::Gopay => write!(f, "gopay"),
            Self::Dana => write!(f, "dana"),
            Self::Shopeepay => write!(f, "shopeepay"),
            Self::Linkaja => write!(f, "linkaja"),
            Self::CreditCard => write!(f, "credit_card"),
            Self::DebitCard => write!(f, "debit_card"),
            Self::Cash => write!(f, "cash"),
            Self::BankTransfer => write!(f, "bank_transfer"),
        }
    }
}

impl FromStr for PaymentChannel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qris" => Ok(Self::Qris),
            "va_bca" => Ok(Self::VaBca),
            "va_bni" => Ok(Self::VaBni),
            "va_bri" => Ok(Self::VaBri),
            "va_mandiri" => Ok(Self::VaMandiri),
            "va_permata" => Ok(Self::VaPermata),
            "va_cimb" => Ok(Self::VaCimb),
            "ovo" => Ok(Self::Ovo),
            "gopay" => Ok(Self::Gopay),
            "dana" => Ok(Self::Dana),
            "shopeepay" => Ok(Self::Shopeepay),
            "linkaja" => Ok(Self::Linkaja),
            "credit_card" => Ok(Self::CreditCard),
            "debit_card" => Ok(Self::DebitCard),
            "cash" => Ok(Self::Cash),
            "bank_transfer" => Ok(Self::BankTransfer),
            _ => Err(format!("Unknown PaymentChannel variant: {}", s)),
        }
    }
}

impl Default for PaymentChannel {
    fn default() -> Self {
        Self::Qris
    }
}

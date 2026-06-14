use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ProductTier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NatalVariant {
    Simplified,
    Full,
}

impl NatalVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simplified => "simplified",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NatalProductCode {
    NatalSimplifiedFree,
    NatalSimplifiedBasic,
    NatalSimplifiedPremium,
    NatalFullFree,
    NatalFullBasic,
    NatalFullPremium,
}

impl NatalProductCode {
    pub fn from_parts(variant: NatalVariant, tier: ProductTier) -> Self {
        match (variant, tier) {
            (NatalVariant::Simplified, ProductTier::Free) => Self::NatalSimplifiedFree,
            (NatalVariant::Simplified, ProductTier::Basic) => Self::NatalSimplifiedBasic,
            (NatalVariant::Simplified, ProductTier::Premium) => Self::NatalSimplifiedPremium,
            (NatalVariant::Full, ProductTier::Free) => Self::NatalFullFree,
            (NatalVariant::Full, ProductTier::Basic) => Self::NatalFullBasic,
            (NatalVariant::Full, ProductTier::Premium) => Self::NatalFullPremium,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NatalSimplifiedFree => "natal_simplified_free",
            Self::NatalSimplifiedBasic => "natal_simplified_basic",
            Self::NatalSimplifiedPremium => "natal_simplified_premium",
            Self::NatalFullFree => "natal_full_free",
            Self::NatalFullBasic => "natal_full_basic",
            Self::NatalFullPremium => "natal_full_premium",
        }
    }
}

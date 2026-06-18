//! Module astral_calculator\src\features\simplified\request.rs du moteur astral_calculator.

use serde::{Deserialize, Serialize};

pub const SIMPLIFIED_REQUEST_CONTRACT_VERSION: &str = "astro_simplified_natal_request_v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Structure AstroSimplifiedNatalRequest.
pub struct AstroSimplifiedNatalRequest {
    pub request_contract_version: String,
    #[serde(default)]
    pub request_id: Option<String>,
    pub birth: SimplifiedBirthRequest,
    #[serde(default)]
    pub input_metadata: Option<SimplifiedInputMetadata>,
    #[serde(default)]
    pub calculation: SimplifiedCalculationRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Structure SimplifiedBirthRequest.
pub struct SimplifiedBirthRequest {
    pub date: String,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub location: Option<SimplifiedLocationRequest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Structure SimplifiedLocationRequest.
pub struct SimplifiedLocationRequest {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Structure SimplifiedInputMetadata.
pub struct SimplifiedInputMetadata {
    #[serde(default)]
    pub location_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Structure SimplifiedCalculationRequest.
pub struct SimplifiedCalculationRequest {
    #[serde(default = "default_zodiac")]
    pub zodiacal_reference_system: String,
    #[serde(default = "default_house_system")]
    pub house_system: String,
}

impl Default for SimplifiedCalculationRequest {
    /// Fonction default.
    fn default() -> Self {
        Self {
            zodiacal_reference_system: default_zodiac(),
            house_system: default_house_system(),
        }
    }
}

/// Fonction default_zodiac.
fn default_zodiac() -> String {
    "tropical".to_string()
}

/// Fonction default_house_system.
fn default_house_system() -> String {
    "placidus".to_string()
}

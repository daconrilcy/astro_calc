//! Module astral_calculator\src\domain\natal_input.rs du moteur astral_calculator.

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure NatalChartInput.
pub struct NatalChartInput {
    pub subject_label: Option<String>,
    pub birth_datetime_utc: DateTime<Utc>,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: Option<f64>,
    pub reference_version_id: i32,
    pub calculation_profile_id: Option<i32>,
    pub zodiacal_reference_system_id: i32,
    pub coordinate_reference_system_id: i32,
    pub house_system_id: i32,
    pub product_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_idempotency_key: Option<String>,
}

impl NatalChartInput {
    /// Fonction product_code.
    pub fn product_code(&self) -> &str {
        self.product_code.as_deref().unwrap_or("basic")
    }
}

#[derive(Debug, Clone)]
/// Structure RuntimeOptions.
pub struct RuntimeOptions {
    pub engine_version: String,
    pub ephemeris_version: String,
    pub stale_after_seconds: i32,
}

impl Default for RuntimeOptions {
    /// Fonction default.
    fn default() -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            ephemeris_version: "se-2026a".to_string(),
            stale_after_seconds: 900,
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

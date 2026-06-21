//! Contrats applicatifs pour la construction des requetes horoscope.

use async_trait::async_trait;

use crate::shared::error::RuntimeError;

#[derive(Debug, Clone)]
pub struct HoroscopeServiceProfile {
    pub service_code: String,
    pub house_system_code: Option<String>,
    pub period_profile_code: Option<String>,
    pub scan_profile_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HoroscopeTimeSlotProfile {
    pub service_code: String,
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HoroscopePeriodProfile {
    pub period_profile_code: String,
    pub resolution_strategy: String,
    pub duration_days: Option<i32>,
    pub week_offset: Option<i32>,
    pub included_days: Option<Vec<String>>,
    pub is_enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct HoroscopeScanProfileDefinition {
    pub scan_profile_code: String,
    pub granularity: String,
    pub reference_time_local: String,
    pub expected_snapshots_per_day: i32,
    pub is_enabled: bool,
}

#[async_trait]
pub trait HoroscopeBuilderCatalog: Send + Sync {
    async fn horoscope_service_profiles(
        &self,
    ) -> Result<Vec<HoroscopeServiceProfile>, RuntimeError>;
    async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfile>, RuntimeError>;
    async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<HoroscopePeriodProfile>, RuntimeError>;
    async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileDefinition>, RuntimeError>;
}

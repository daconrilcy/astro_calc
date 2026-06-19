//! Module astral_calculator\src\infra\db\horoscope_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use async_trait::async_trait;

use super::models::{
    AstralTimePeriodProfileRow, HoroscopeOrbWeightBandRow, HoroscopeScanProfileRow,
    HoroscopeServiceRow, HoroscopeSignalThemeMappingRow, HoroscopeSupportedObjectRow,
    HoroscopeTimeSlotProfileRow,
};
use super::runtime_queries::RuntimeQueries;
use crate::application::ports::{
    HoroscopeBuilderCatalog, HoroscopeCatalog, HoroscopeOrbWeightBand, HoroscopePeriodProfile,
    HoroscopeScanProfileDefinition, HoroscopeServiceProfile, HoroscopeTimeSlotProfile,
};
use crate::features::horoscope::{HoroscopeSignalThemeMapping, HoroscopeSupportedObject};
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure HoroscopeRepository.
pub struct HoroscopeRepository {
    inner: RuntimeQueries,
}

#[async_trait]
impl HoroscopeCatalog for HoroscopeRepository {
    async fn horoscope_orb_weight_bands(
        &self,
    ) -> Result<Vec<HoroscopeOrbWeightBand>, RuntimeError> {
        Ok(HoroscopeRepository::horoscope_orb_weight_bands(self)
            .await?
            .into_iter()
            .map(|row| HoroscopeOrbWeightBand {
                max_orb_deg: row.max_orb_deg,
            })
            .collect())
    }

    async fn horoscope_supported_objects(
        &self,
    ) -> Result<Vec<HoroscopeSupportedObject>, RuntimeError> {
        Ok(HoroscopeRepository::horoscope_supported_objects(self)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn horoscope_signal_theme_mappings(
        &self,
    ) -> Result<Vec<HoroscopeSignalThemeMapping>, RuntimeError> {
        Ok(HoroscopeRepository::horoscope_signal_theme_mappings(self)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}

#[async_trait]
impl HoroscopeBuilderCatalog for HoroscopeRepository {
    async fn horoscope_service_profiles(
        &self,
    ) -> Result<Vec<HoroscopeServiceProfile>, RuntimeError> {
        Ok(HoroscopeRepository::horoscope_services(self)
            .await?
            .into_iter()
            .map(|row| HoroscopeServiceProfile {
                service_code: row.service_code,
                house_system_code: row.house_system_code,
                period_profile_code: row.period_profile_code,
                scan_profile_code: row.scan_profile_code,
            })
            .collect())
    }

    async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfile>, RuntimeError> {
        Ok(HoroscopeRepository::horoscope_time_slot_profiles(self)
            .await?
            .into_iter()
            .map(|row| HoroscopeTimeSlotProfile {
                service_code: row.service_code,
                slot_code: row.slot_code,
                start_local_time: row.start_local_time,
                end_local_time: row.end_local_time,
                reference_local_time: row.reference_local_time,
                sort_order: row.sort_order,
            })
            .collect())
    }

    async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<HoroscopePeriodProfile>, RuntimeError> {
        Ok(HoroscopeRepository::astral_time_period_profiles(self)
            .await?
            .into_iter()
            .map(|row| HoroscopePeriodProfile {
                period_profile_code: row.period_profile_code,
                resolution_strategy: row.resolution_strategy,
                duration_days: row.duration_days,
                week_offset: row.week_offset,
                included_days: row.included_days,
                is_enabled: row.is_enabled,
                sort_order: row.sort_order,
            })
            .collect())
    }

    async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileDefinition>, RuntimeError> {
        Ok(HoroscopeRepository::horoscope_scan_profiles(self)
            .await?
            .into_iter()
            .map(|row| HoroscopeScanProfileDefinition {
                scan_profile_code: row.scan_profile_code,
                granularity: row.granularity,
                reference_time_local: row.reference_time_local,
                expected_snapshots_per_day: row.expected_snapshots_per_day,
                is_enabled: row.is_enabled,
            })
            .collect())
    }
}

impl HoroscopeRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeQueries::new(pool),
        }
    }

    /// Fonction horoscope_services.
    pub async fn horoscope_services(&self) -> Result<Vec<HoroscopeServiceRow>, RuntimeError> {
        self.inner.horoscope_services().await
    }

    /// Fonction horoscope_time_slot_profiles.
    pub async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfileRow>, RuntimeError> {
        self.inner.horoscope_time_slot_profiles().await
    }

    /// Fonction astral_time_period_profiles.
    pub async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<AstralTimePeriodProfileRow>, RuntimeError> {
        self.inner.astral_time_period_profiles().await
    }

    /// Fonction horoscope_scan_profiles.
    pub async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileRow>, RuntimeError> {
        self.inner.horoscope_scan_profiles().await
    }

    /// Fonction horoscope_orb_weight_bands.
    pub async fn horoscope_orb_weight_bands(
        &self,
    ) -> Result<Vec<HoroscopeOrbWeightBandRow>, RuntimeError> {
        self.inner.horoscope_orb_weight_bands().await
    }

    /// Fonction horoscope_supported_objects.
    pub async fn horoscope_supported_objects(
        &self,
    ) -> Result<Vec<HoroscopeSupportedObjectRow>, RuntimeError> {
        self.inner.horoscope_supported_objects().await
    }

    /// Fonction horoscope_signal_theme_mappings.
    pub async fn horoscope_signal_theme_mappings(
        &self,
    ) -> Result<Vec<HoroscopeSignalThemeMappingRow>, RuntimeError> {
        self.inner.horoscope_signal_theme_mappings().await
    }
}

impl From<HoroscopeSupportedObjectRow> for HoroscopeSupportedObject {
    fn from(row: HoroscopeSupportedObjectRow) -> Self {
        Self {
            object_code: row.object_code,
            weight: row.weight,
        }
    }
}

impl From<HoroscopeSignalThemeMappingRow> for HoroscopeSignalThemeMapping {
    fn from(row: HoroscopeSignalThemeMappingRow) -> Self {
        Self {
            match_object: row.match_object,
            match_aspect: row.match_aspect,
            match_natal_target: row.match_natal_target,
            theme_code: row.theme_code,
        }
    }
}

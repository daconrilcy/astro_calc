//! Module astral_calculator\src\infra\db\horoscope_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use async_trait::async_trait;

use super::models::{
    AstralTimePeriodProfileRow, HoroscopeOrbWeightBandRow, HoroscopeScanProfileRow,
    HoroscopeServiceRow, HoroscopeSignalThemeMappingRow, HoroscopeSupportedObjectRow,
    HoroscopeTimeSlotProfileRow,
};
use super::runtime_queries::RuntimeQueries;
use crate::application::ports::{HoroscopeCatalog, HoroscopeOrbWeightBand};
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

//! Module astral_calculator\src\infra\db\horoscope_repository.rs du moteur astral_calculator.

use sqlx::PgPool;

use super::models::{
    AstralTimePeriodProfileRow, HoroscopeOrbWeightBandRow, HoroscopeScanProfileRow,
    HoroscopeServiceRow, HoroscopeTimeSlotProfileRow,
};
use super::runtime_repository::RuntimeRepository;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
/// Structure HoroscopeRepository.
pub struct HoroscopeRepository {
    inner: RuntimeRepository,
}

impl HoroscopeRepository {
    /// Fonction new.
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
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
}

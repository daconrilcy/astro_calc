use sqlx::PgPool;

use super::models::{
    AstralTimePeriodProfileRow, HoroscopeOrbWeightBandRow, HoroscopeScanProfileRow,
    HoroscopeServiceRow, HoroscopeTimeSlotProfileRow,
};
use super::runtime_repository::RuntimeRepository;
use crate::shared::error::RuntimeError;

#[derive(Clone)]
pub struct HoroscopeRepository {
    inner: RuntimeRepository,
}

impl HoroscopeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: RuntimeRepository::new(pool),
        }
    }

    pub async fn horoscope_services(&self) -> Result<Vec<HoroscopeServiceRow>, RuntimeError> {
        self.inner.horoscope_services().await
    }

    pub async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfileRow>, RuntimeError> {
        self.inner.horoscope_time_slot_profiles().await
    }

    pub async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<AstralTimePeriodProfileRow>, RuntimeError> {
        self.inner.astral_time_period_profiles().await
    }

    pub async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileRow>, RuntimeError> {
        self.inner.horoscope_scan_profiles().await
    }

    pub async fn horoscope_orb_weight_bands(
        &self,
    ) -> Result<Vec<HoroscopeOrbWeightBandRow>, RuntimeError> {
        self.inner.horoscope_orb_weight_bands().await
    }
}

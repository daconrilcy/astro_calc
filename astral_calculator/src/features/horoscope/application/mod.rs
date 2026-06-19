//! Module astral_calculator\src\features\horoscope\application\mod.rs du moteur astral_calculator.

use async_trait::async_trait;

use crate::features::horoscope::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopePeriodCalculationRequest,
    HoroscopePeriodCalculationResponse,
};
use crate::shared::error::RuntimeError;

pub mod horoscope_service;

pub use horoscope_service::HoroscopeService;

#[async_trait]
pub trait HoroscopeCapability: Send + Sync {
    async fn calculate_daily(
        &self,
        request: HoroscopeCalculationRequest,
    ) -> Result<HoroscopeCalculationResponse, RuntimeError>;
    async fn calculate_period(
        &self,
        request: HoroscopePeriodCalculationRequest,
    ) -> Result<HoroscopePeriodCalculationResponse, RuntimeError>;
}

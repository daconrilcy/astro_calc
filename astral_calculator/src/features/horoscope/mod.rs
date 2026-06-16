// Horoscope domain facade.
// This module keeps the public horoscope API stable while delegating
// implementation details to focused submodules.
mod builders;
mod contracts;
mod daily;
mod period;

pub use astral_contracts::{
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
};
pub use builders::{
    build_horoscope_daily_calculation_request_from_public,
    build_horoscope_period_calculation_request_from_public,
};
pub use contracts::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopeCalculationSlot,
    HoroscopeCalculationSlotRequest, HoroscopeLocation, HoroscopePeriod,
    HoroscopePeriodCalculationRequest, HoroscopePeriodCalculationResponse, HoroscopePeriodSnapshot,
    HoroscopeScanPlan, HoroscopeSnapshotRequest, HoroscopeTransitFact,
};
pub use daily::calculate_horoscope_daily_natal;
pub use period::{
    calculate_horoscope_period_natal, calculate_horoscope_period_natal_from_positions,
    calculate_horoscope_period_natal_from_transits, normalize_horoscope_period_request_utc,
};

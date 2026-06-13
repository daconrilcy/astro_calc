use super::*;
pub const HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";
pub const HOROSCOPE_FREE_DAILY_SERVICE_CODE: &str = "horoscope_free_daily";
pub const HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE: &str =
    "horoscope_premium_daily_local_2h_slots";
pub const HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str =
    "horoscope_basic_next_7_days_natal";
pub const HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str = "horoscope_free_next_7_days_natal";
pub const HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str =
    "horoscope_premium_next_7_days_natal";
pub const HOROSCOPE_SERVICE_CODE: &str = HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE;
pub(crate) const HOROSCOPE_PRODUCT_CODE: &str = "horoscope";
pub(crate) const PERIOD_V2_QUALITY_MAX_RETRIES: usize = 2;
pub(crate) const PERIOD_V2_MAX_OUTPUT_TOKENS: u32 = 16_000;
pub(crate) const PERIOD_V2_OBJECTIVE_TEXT_REPLACEMENTS: &[(&str, &str)] = &[
    ("demie-journée", "demi-journée"),
    ("demie journée", "demi-journée"),
    ("reorganiser", "réorganiser"),
    ("Reorganiser", "Réorganiser"),
];
pub(crate) fn service_code_from_value(value: &Value) -> Result<&str, GenerationError> {
    let service_code = value
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    validate_supported_service_code(service_code)?;
    Ok(service_code)
}
pub(crate) fn validate_supported_service_code(service_code: &str) -> Result<(), GenerationError> {
    if matches!(
        service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
            | HOROSCOPE_FREE_DAILY_SERVICE_CODE
            | HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
    ) {
        return Ok(());
    }
    Err(horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))
}
pub(crate) fn validate_period_service_code(service_code: &str) -> Result<(), GenerationError> {
    if matches!(
        service_code,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
    ) {
        return Ok(());
    }
    Err(horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))
}

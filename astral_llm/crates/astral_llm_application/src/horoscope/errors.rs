use super::*;
pub(crate) fn horoscope_error(code: &str) -> GenerationError {
    GenerationError::with_details(GenerationErrorCode::InvalidInput, code, Value::Null)
}
pub(crate) fn quality_error(code: &str, details: Value) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::PostSafetyValidationFailed,
        code,
        details,
    )
}

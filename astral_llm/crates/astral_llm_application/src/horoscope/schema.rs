use super::*;
pub(crate) const INTERPRETATION_REQUEST_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_interpretation_request.schema.json");
pub(crate) const RESPONSE_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_response.schema.json");
pub(crate) const PERIOD_WRITER_REQUEST_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_period_writer_request.schema.json");
pub(crate) const PERIOD_RESPONSE_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_period_response.schema.json");
pub fn validate_interpretation_request_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        interpretation_request_schema,
        "HOROSCOPE_RESPONSE_INVALID",
        value,
    )
}
pub fn validate_horoscope_response_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(response_schema, "HOROSCOPE_RESPONSE_INVALID", value)
}
pub(crate) fn validate_schema(
    schema: fn() -> &'static JSONSchema,
    code: &str,
    value: &Value,
) -> Result<(), GenerationError> {
    schema().validate(value).map_err(|errors| {
        let errors = errors.map(|err| err.to_string()).collect::<Vec<_>>();
        let message = if errors.is_empty() {
            code.to_string()
        } else {
            format!("{code}: {}", errors.join("; "))
        };
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            message,
            json!({ "errors": errors }),
        )
    })
}
pub(crate) fn interpretation_request_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(INTERPRETATION_REQUEST_SCHEMA_JSON))
}
pub(crate) fn response_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(RESPONSE_SCHEMA_JSON))
}
pub(crate) fn period_writer_request_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(PERIOD_WRITER_REQUEST_SCHEMA_JSON))
}
pub(crate) fn period_response_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(PERIOD_RESPONSE_SCHEMA_JSON))
}
pub(crate) fn compile_schema(raw: &str) -> JSONSchema {
    let schema: Value = serde_json::from_str(raw).expect("horoscope schema json is valid");
    JSONSchema::compile(&schema).expect("horoscope schema compiles")
}

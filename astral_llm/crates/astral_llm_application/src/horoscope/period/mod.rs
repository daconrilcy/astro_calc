use super::*;

pub(crate) mod calculation_request;
pub(crate) mod contract_validators;
pub(crate) mod evidence;
pub(crate) mod free_validators;
pub(crate) mod period_metadata;
pub(crate) mod postprocess;
pub(crate) mod public_request;
pub(crate) mod public_text_validation;
pub(crate) mod quality;
pub(crate) mod response_repair;
pub(crate) mod response_sanitize;
pub(crate) mod runtime_helpers;
pub(crate) mod semantic_brief;
pub(crate) mod semantic_brief_validation;
pub(crate) mod validators;
pub(crate) mod writer;

pub use calculation_request::{
    build_period_calculation_request, build_period_calculation_request_for_service,
    validate_scan_plan,
};
pub use contract_validators::validate_period_response_contract_gates;
pub use free_validators::validate_period_provider_public_payload;
pub use period_metadata::{
    period_style_editor_max_output_tokens, period_writer_max_output_tokens,
    period_writer_reasoning_effort, validate_period_public_word_count,
};
pub use postprocess::{postprocess_period_provider_response, reprocess_horoscope_period_payload};
pub use public_request::validate_period_public_request;
pub use quality::{
    period_editorial_audit, period_quality_audit, validate_period_response_quality_gates,
};
pub use response_repair::repair_period_response_shape;
pub use semantic_brief::build_period_writer_request;
pub use validators::{
    validate_period_response_contract, validate_period_response_evidence,
    validate_period_response_schema, validate_period_writer_request_schema,
};
pub use writer::{
    fake_period_writer_response, period_response_provider_schema, period_writer_messages,
    period_writer_response_with_quality_loop,
};

pub(crate) use evidence::*;
pub(crate) use free_validators::*;
pub(crate) use period_metadata::*;
pub(crate) use postprocess::*;
pub(crate) use public_text_validation::*;
pub(crate) use quality::*;
pub(crate) use response_repair::*;
pub(crate) use runtime_helpers::*;
pub(crate) use semantic_brief::*;
pub(crate) use semantic_brief_validation::*;
pub(crate) use validators::*;
pub(crate) use writer::*;

pub fn build_period_interpretation_request(
    public: &HoroscopePeriodPublicRequest,
    calculation: &Value,
) -> Result<Value, GenerationError> {
    build_period_writer_request(public, calculation)
}

pub fn validate_period_interpretation_request_schema(value: &Value) -> Result<(), GenerationError> {
    validate_period_writer_request_schema(value)
}

pub fn validate_semantic_brief_is_atomic(value: &Value) -> Result<(), GenerationError> {
    validate_semantic_brief_references_only(value)
}

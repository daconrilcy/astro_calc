use super::*;

pub(crate) mod calculation_request;
pub(crate) mod contract_validators;
pub(crate) mod evidence;
pub(crate) mod free_validators;
pub(crate) mod legacy_v1;
pub(crate) mod period_metadata;
pub(crate) mod postprocess;
pub(crate) mod public_request;
pub(crate) mod public_text_validation;
pub(crate) mod quality;
pub(crate) mod response_length;
pub(crate) mod response_repair;
pub(crate) mod response_sanitize;
pub(crate) mod scoring;
pub(crate) mod semantic_brief;
pub(crate) mod semantic_brief_validation;
pub(crate) mod validators;
pub(crate) mod writer;

pub use calculation_request::{
    build_period_calculation_request, build_period_calculation_request_for_service,
    validate_scan_plan,
};
pub use contract_validators::validate_period_response_contract_gates_v2;
pub use free_validators::validate_period_provider_public_payload;
pub use legacy_v1::build_period_interpretation_request;
pub use period_metadata::{
    period_writer_max_output_tokens, period_writer_reasoning_effort,
    validate_period_public_word_count,
};
pub use postprocess::{
    postprocess_period_provider_response, postprocess_period_provider_response_v2,
    reprocess_horoscope_period_payload,
};
pub use public_request::validate_period_public_request;
pub use quality::{
    period_v2_editorial_audit, period_v2_quality_audit, validate_period_response_quality_gates_v2,
};
pub use response_repair::{
    prune_period_response_variant_fields, repair_period_response_shape,
    repair_period_response_shape_v2,
};
pub use semantic_brief::build_period_writer_request_v2;
pub use semantic_brief_validation::validate_semantic_brief_is_atomic;
pub use validators::{
    validate_period_interpretation_request_schema, validate_period_response_evidence,
    validate_period_response_schema, validate_period_writer_request_v2_schema,
};
pub use writer::{
    fake_period_writer_response, fake_period_writer_response_v2, period_response_provider_schema,
    period_writer_messages,
};

pub(crate) use evidence::*;
pub(crate) use free_validators::*;
pub(crate) use period_metadata::*;
pub(crate) use postprocess::*;
pub(crate) use public_text_validation::*;
pub(crate) use quality::*;
pub(crate) use response_length::*;
pub(crate) use response_repair::*;
pub(crate) use response_sanitize::*;
pub(crate) use scoring::*;
pub(crate) use semantic_brief::*;
pub(crate) use validators::*;
pub(crate) use writer::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeriodGenerationMode {
    LegacyV1,
    SemanticBriefV2,
}

impl PeriodGenerationMode {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, GenerationError> {
        match value.unwrap_or("legacy_v1") {
            "legacy_v1" => Ok(Self::LegacyV1),
            "semantic_brief_v2" => Ok(Self::SemanticBriefV2),
            _ => Err(horoscope_error(
                "HOROSCOPE_PERIOD_GENERATION_MODE_UNSUPPORTED",
            )),
        }
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::LegacyV1 => "legacy_v1",
            Self::SemanticBriefV2 => "semantic_brief_v2",
        }
    }
}

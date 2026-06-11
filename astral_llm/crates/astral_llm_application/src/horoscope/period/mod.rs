use super::{horoscope_error, GenerationError};

pub(crate) mod calculation_request;
pub(crate) mod evidence;
pub(crate) mod legacy_v1;
pub(crate) mod postprocess;
pub(crate) mod public_request;
pub(crate) mod quality;
pub(crate) mod response_repair;
pub(crate) mod scoring;
pub(crate) mod semantic_brief;
pub(crate) mod validators;
pub(crate) mod writer;
pub(crate) mod writer_request;

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

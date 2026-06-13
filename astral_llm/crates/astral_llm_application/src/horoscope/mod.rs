use crate::french_typography::{
    french_elision_violations, french_glued_compound_violations, restore_french_glued_compounds,
};
use crate::generate_reading_use_case::GenerateReadingUseCase;
use crate::text_reprocessing_service_adapter::{
    reprocess_horoscope_daily, reprocess_horoscope_period,
};
use astral_llm_domain::{
    model_usage_tier::ModelRouteContext, EngineDefaults, GenerationError, GenerationErrorCode,
    ProviderKind, ReasoningEffort, SafetyMode,
};
use astral_llm_providers::{
    GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest,
};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use jsonschema::JSONSchema;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Duration as StdDuration;
pub(crate) mod daily;
pub(crate) mod errors;
pub(crate) mod orchestrators;
pub(crate) mod period;
pub(crate) mod reference_data;
pub(crate) mod schema;
pub(crate) mod service_codes;
pub(crate) mod text;
pub(crate) mod types;
pub(crate) mod writer_engine;
pub(crate) use daily::*;
pub use daily::{
    aggregate_themes, build_calculation_request, build_calculation_request_for_service,
    build_interpretation_request, daily_response_provider_schema, daily_writer_messages,
    score_calculation, validate_public_request, validate_response_evidence,
};
pub(crate) use errors::*;
pub use orchestrators::{
    HoroscopeBasicDailyNatalOrchestrator, HoroscopeDailyNatalOrchestrator,
    HoroscopeFreeDailyOrchestrator, HoroscopePeriodNatalOrchestrator,
    HoroscopePremiumDailyLocalOrchestrator,
};
pub(crate) use period::*;
pub use period::{
    build_period_calculation_request, build_period_calculation_request_for_service,
    build_period_interpretation_request, build_period_writer_request_v2,
    fake_period_writer_response, fake_period_writer_response_v2, period_response_provider_schema,
    period_v2_editorial_audit, period_v2_quality_audit, period_writer_max_output_tokens,
    period_writer_messages, period_writer_reasoning_effort, postprocess_period_provider_response,
    postprocess_period_provider_response_v2, prune_period_response_variant_fields,
    repair_period_response_shape, repair_period_response_shape_v2,
    reprocess_horoscope_period_payload, validate_period_interpretation_request_schema,
    validate_period_provider_public_payload, validate_period_public_request,
    validate_period_public_word_count, validate_period_response_contract_gates_v2,
    validate_period_response_evidence, validate_period_response_quality_gates_v2,
    validate_period_response_schema, validate_period_writer_request_v2_schema, validate_scan_plan,
    validate_semantic_brief_is_atomic,
};
pub use reference_data::public_watch_point_for_theme;
pub(crate) use reference_data::*;
pub(crate) use schema::*;
pub use schema::{validate_horoscope_response_schema, validate_interpretation_request_schema};
pub(crate) use service_codes::*;
pub use service_codes::{
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_SERVICE_CODE,
};
pub(crate) use text::*;
pub(crate) use types::*;
pub use types::{
    AstrologerPersona, HoroscopeLocation, HoroscopePeriodPublicRequest, HoroscopePublicRequest,
    ScoredSignal, SlotInterpretationPlan, SlotProfile, TargetLanguageCode,
};
pub(crate) use writer_engine::*;

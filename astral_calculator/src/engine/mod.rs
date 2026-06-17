pub mod application;
#[allow(dead_code)]
pub mod calculation_refs;
pub mod env;
pub mod projection;
mod request;
mod resolve;
mod response;

pub use calculation_refs::{
    coordinate_reference_system_id_from_env, coordinate_reference_system_key_from_env,
    house_system_code_from_env, house_system_id_from_env, zodiacal_reference_system_id_from_env,
    zodiacal_reference_system_key_from_env,
};
pub use env::{birth_datetime_utc_from_env, engine_request_from_env};
pub use request::{
    AstroEngineRequest, EngineBirthLocation, EngineBirthRequest, EngineCalculationRequest,
    EngineProjectionRequest, LLM_PROJECTION_CONTRACT_VERSION, REQUEST_CONTRACT_VERSION,
    RESPONSE_CONTRACT_VERSION,
};
pub use resolve::{
    local_birth_to_utc, validate_and_resolve_request, validate_request_early, ResolvedEngineRequest,
};
pub use response::{
    AstroEngineResponse, EngineAuditPayload, EngineCalculationResult, EngineEchoLocation,
    EngineRequestEcho,
};

use crate::domain::{BasicPayload, RuntimeOptions};
use crate::engine::projection::{build_llm_projection_natal_v1, LlmProjectionBuildContext};
use crate::shared::error::RuntimeError;

#[allow(clippy::too_many_arguments)]
pub fn build_engine_response(
    resolved: &ResolvedEngineRequest,
    audit: BasicPayload,
    options: &RuntimeOptions,
    zodiac_label: &str,
    coordinate_label: &str,
    house_system_label: &str,
    house_axes: &[crate::domain::HouseAxisReference],
    profile: &crate::engine::projection::LlmProjectionProfile,
) -> Result<AstroEngineResponse, RuntimeError> {
    let raw_contract = audit
        .chart_context
        .payload_contract
        .contract_version
        .clone();

    let llm_payload = build_llm_projection_natal_v1(
        &audit,
        profile,
        &LlmProjectionBuildContext {
            birth_location_label: &resolved.location_label,
            zodiac_label,
            coordinate_label,
            house_system_label,
            house_axes,
        },
    );

    Ok(AstroEngineResponse {
        response_contract_version: RESPONSE_CONTRACT_VERSION.to_string(),
        request_echo: EngineRequestEcho {
            calculation_type: resolved.calculation_type.clone(),
            birth_datetime_local: resolved.birth_datetime_local.clone(),
            birth_timezone: resolved.birth_timezone.clone(),
            birth_datetime_utc: resolved.birth_datetime_utc.to_rfc3339(),
            location: EngineEchoLocation {
                label: Some(resolved.location_label.clone()),
                latitude: resolved.natal_input.latitude_deg,
                longitude: resolved.natal_input.longitude_deg,
            },
            projection_level: resolved.projection_level.clone(),
        },
        calculation_result: EngineCalculationResult {
            status: "completed".to_string(),
            chart_calculation_id: audit.chart_calculation_id,
            engine_version: options.engine_version.clone(),
            ephemeris_version: options.ephemeris_version.clone(),
            raw_payload_contract_version: raw_contract,
            llm_projection_contract_version: LLM_PROJECTION_CONTRACT_VERSION.to_string(),
        },
        audit_payload: EngineAuditPayload {
            contract_version: audit
                .chart_context
                .payload_contract
                .contract_version
                .clone(),
            payload: audit,
        },
        llm_payload,
    })
}

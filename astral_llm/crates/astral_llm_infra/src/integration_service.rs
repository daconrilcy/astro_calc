use std::collections::HashMap;

use astral_contracts::OrchestrationMode;
use astral_llm_domain::integration::{CalculationMode, IntegrationService, ServiceAvailability};
use sqlx::PgPool;

pub async fn load_integration_services(pool: &PgPool) -> Vec<IntegrationService> {
    let rows = match sqlx::query_as::<_, IntegrationServiceRow>(
        "SELECT service_code, profile_code, product_code, label_fr, description_fr, \
         orchestration_mode, calculation_mode, service_request_contract, payload_contract, \
         service_response_contract, calculation_output_contract, reading_output_contract, \
         sync_endpoint, async_endpoint, supports_async, supports_sync_legacy, supports_mercure, \
         availability, example_request_json, sort_order \
         FROM llm_integration_services \
         ORDER BY sort_order, service_code",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(error = %err, "failed to load integration services");
            return Vec::new();
        }
    };

    rows.into_iter()
        .filter_map(|row| row.into_domain())
        .collect()
}

pub fn integration_services_map(
    services: &[IntegrationService],
) -> HashMap<String, IntegrationService> {
    services
        .iter()
        .map(|s| (s.service_code.clone(), s.clone()))
        .collect()
}

#[derive(Debug, sqlx::FromRow)]
struct IntegrationServiceRow {
    service_code: String,
    profile_code: String,
    product_code: String,
    label_fr: String,
    description_fr: String,
    orchestration_mode: String,
    calculation_mode: String,
    service_request_contract: String,
    payload_contract: String,
    service_response_contract: String,
    calculation_output_contract: Option<String>,
    reading_output_contract: String,
    sync_endpoint: Option<String>,
    async_endpoint: String,
    supports_async: bool,
    supports_sync_legacy: bool,
    supports_mercure: bool,
    availability: String,
    example_request_json: Option<serde_json::Value>,
    sort_order: i32,
}

impl IntegrationServiceRow {
    fn into_domain(self) -> Option<IntegrationService> {
        let calculation_mode = CalculationMode::parse(&self.calculation_mode)?;
        let availability = ServiceAvailability::parse(&self.availability)?;
        let orchestration_mode_typed = parse_orchestration_mode(&self.orchestration_mode);
        let service_request_contract = self.service_request_contract.clone();
        let payload_contract = self.payload_contract.clone();
        let service_response_contract = self.service_response_contract.clone();
        let calculator_request_contract = infer_calculator_request_contract(
            orchestration_mode_typed,
            &payload_contract,
            calculation_mode,
        );
        let llm_request_contract =
            infer_llm_request_contract(orchestration_mode_typed, &payload_contract);
        Some(IntegrationService {
            service_code: self.service_code,
            profile_code: self.profile_code,
            product_code: self.product_code,
            label_fr: self.label_fr,
            description_fr: self.description_fr,
            orchestration_mode_typed: Some(orchestration_mode_typed),
            orchestration_mode: self.orchestration_mode,
            calculation_mode,
            service_request_contract: service_request_contract.clone(),
            payload_contract: payload_contract.clone(),
            service_response_contract: service_response_contract.clone(),
            public_request_contract: Some(service_request_contract.clone()),
            calculator_request_contract,
            llm_request_contract,
            public_response_contract: Some(service_response_contract.clone()),
            calculation_output_contract: self.calculation_output_contract,
            reading_output_contract: self.reading_output_contract,
            sync_endpoint: self.sync_endpoint,
            async_endpoint: self.async_endpoint,
            supports_async: self.supports_async,
            supports_sync_legacy: self.supports_sync_legacy,
            supports_mercure: self.supports_mercure,
            availability,
            example_request_json: self.example_request_json,
            sort_order: self.sort_order.try_into().ok()?,
        })
    }
}

fn parse_orchestration_mode(raw: &str) -> OrchestrationMode {
    match raw {
        "calculator_only" => OrchestrationMode::CalculatorOnly,
        "llm_only" => OrchestrationMode::LlmOnly,
        "public_gateway" => OrchestrationMode::PublicGateway,
        "legacy_unified" => OrchestrationMode::LegacyUnified,
        _ => OrchestrationMode::CalculatorThenLlm,
    }
}

fn infer_calculator_request_contract(
    orchestration_mode: OrchestrationMode,
    payload_contract: &str,
    calculation_mode: CalculationMode,
) -> Option<String> {
    if matches!(
        orchestration_mode,
        OrchestrationMode::CalculatorOnly | OrchestrationMode::CalculatorThenLlm
    ) || !matches!(calculation_mode, CalculationMode::None)
    {
        Some(payload_contract.to_string())
    } else {
        None
    }
}

fn infer_llm_request_contract(
    orchestration_mode: OrchestrationMode,
    payload_contract: &str,
) -> Option<String> {
    match orchestration_mode {
        OrchestrationMode::LlmOnly => Some(payload_contract.to_string()),
        _ => None,
    }
}

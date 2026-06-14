use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationMode {
    CalculatorOnly,
    LlmOnly,
    CalculatorThenLlm,
    PublicGateway,
    LegacyUnified,
}

impl OrchestrationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CalculatorOnly => "calculator_only",
            Self::LlmOnly => "llm_only",
            Self::CalculatorThenLlm => "calculator_then_llm",
            Self::PublicGateway => "public_gateway",
            Self::LegacyUnified => "legacy_unified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ServiceContractSet {
    pub public_request_contract: &'static str,
    pub calculator_request_contract: &'static str,
    pub llm_request_contract: &'static str,
    pub public_response_contract: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ServiceDescriptor {
    pub service_code: &'static str,
    pub orchestration_mode: OrchestrationMode,
    pub contracts: ServiceContractSet,
}

pub const HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";
pub const HOROSCOPE_FREE_DAILY_SERVICE_CODE: &str = "horoscope_free_daily";
pub const HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE: &str =
    "horoscope_premium_daily_local_2h_slots";
pub const HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str =
    "horoscope_basic_next_7_days_natal";
pub const HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str = "horoscope_free_next_7_days_natal";
pub const HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE: &str =
    "horoscope_premium_next_7_days_natal";

pub const HOROSCOPE_SERVICE_DESCRIPTORS: &[ServiceDescriptor] = &[
    ServiceDescriptor {
        service_code: HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        orchestration_mode: OrchestrationMode::CalculatorThenLlm,
        contracts: ServiceContractSet {
            public_request_contract: "horoscope_daily_request_v2",
            calculator_request_contract: "horoscope_calculation_request",
            llm_request_contract: "horoscope_interpretation_request",
            public_response_contract: "horoscope_response",
        },
    },
    ServiceDescriptor {
        service_code: HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        orchestration_mode: OrchestrationMode::CalculatorThenLlm,
        contracts: ServiceContractSet {
            public_request_contract: "horoscope_daily_request_v2",
            calculator_request_contract: "horoscope_calculation_request",
            llm_request_contract: "horoscope_interpretation_request",
            public_response_contract: "horoscope_response",
        },
    },
    ServiceDescriptor {
        service_code: HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        orchestration_mode: OrchestrationMode::CalculatorThenLlm,
        contracts: ServiceContractSet {
            public_request_contract: "horoscope_daily_request_v2",
            calculator_request_contract: "horoscope_calculation_request",
            llm_request_contract: "horoscope_interpretation_request",
            public_response_contract: "horoscope_response",
        },
    },
    ServiceDescriptor {
        service_code: HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        orchestration_mode: OrchestrationMode::CalculatorThenLlm,
        contracts: ServiceContractSet {
            public_request_contract: "horoscope_period_request_v2",
            calculator_request_contract: "horoscope_period_calculation_request",
            llm_request_contract: "horoscope_period_writer_request",
            public_response_contract: "horoscope_period_response",
        },
    },
    ServiceDescriptor {
        service_code: HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        orchestration_mode: OrchestrationMode::CalculatorThenLlm,
        contracts: ServiceContractSet {
            public_request_contract: "horoscope_period_request_v2",
            calculator_request_contract: "horoscope_period_calculation_request",
            llm_request_contract: "horoscope_period_writer_request",
            public_response_contract: "horoscope_period_response",
        },
    },
    ServiceDescriptor {
        service_code: HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        orchestration_mode: OrchestrationMode::CalculatorThenLlm,
        contracts: ServiceContractSet {
            public_request_contract: "horoscope_period_request_v2",
            calculator_request_contract: "horoscope_period_calculation_request",
            llm_request_contract: "horoscope_period_writer_request",
            public_response_contract: "horoscope_period_response",
        },
    },
];

pub fn horoscope_service_descriptor(service_code: &str) -> Option<&'static ServiceDescriptor> {
    HOROSCOPE_SERVICE_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.service_code == service_code)
}

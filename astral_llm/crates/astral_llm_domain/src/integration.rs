//! Contrats integration API (catalogue services, jobs async).

use astral_contracts::OrchestrationMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    SafetyRejected,
    Cancelled,
    Expired,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::SafetyRejected => "safety_rejected",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "safety_rejected" => Some(Self::SafetyRejected),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::SafetyRejected | Self::Cancelled | Self::Expired
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceAvailability {
    Active,
    Beta,
    Planned,
    Deprecated,
    Disabled,
}

impl ServiceAvailability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Beta => "beta",
            Self::Planned => "planned",
            Self::Deprecated => "deprecated",
            Self::Disabled => "disabled",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "active" => Some(Self::Active),
            "beta" => Some(Self::Beta),
            "planned" => Some(Self::Planned),
            "deprecated" => Some(Self::Deprecated),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    pub fn is_submittable(&self) -> bool {
        matches!(self, Self::Active | Self::Beta)
    }

    pub fn is_public_listed(&self, include_planned: bool) -> bool {
        match self {
            Self::Active | Self::Beta => true,
            Self::Planned if include_planned => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalculationMode {
    None,
    SimplifiedNatal,
    FullNatal,
}

impl CalculationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SimplifiedNatal => "simplified_natal",
            Self::FullNatal => "full_natal",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "none" => Some(Self::None),
            "simplified_natal" => Some(Self::SimplifiedNatal),
            "full_natal" => Some(Self::FullNatal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationService {
    pub service_code: String,
    pub profile_code: String,
    pub product_code: String,
    pub label_fr: String,
    pub description_fr: String,
    pub orchestration_mode: String,
    #[serde(default)]
    pub orchestration_mode_typed: Option<OrchestrationMode>,
    pub calculation_mode: CalculationMode,
    pub service_request_contract: String,
    pub payload_contract: String,
    pub service_response_contract: String,
    #[serde(default)]
    pub public_request_contract: Option<String>,
    #[serde(default)]
    pub calculator_request_contract: Option<String>,
    #[serde(default)]
    pub llm_request_contract: Option<String>,
    #[serde(default)]
    pub public_response_contract: Option<String>,
    pub calculation_output_contract: Option<String>,
    pub reading_output_contract: String,
    pub sync_endpoint: Option<String>,
    pub async_endpoint: String,
    pub supports_async: bool,
    pub supports_sync_legacy: bool,
    pub supports_mercure: bool,
    pub availability: ServiceAvailability,
    pub example_request_json: Option<serde_json::Value>,
    pub sort_order: i16,
}

impl IntegrationService {
    pub fn is_from_payload(&self) -> bool {
        self.service_code.ends_with("_from_payload")
    }

    pub fn resolved_orchestration_mode(&self) -> OrchestrationMode {
        self.orchestration_mode_typed
            .unwrap_or_else(|| match self.orchestration_mode.as_str() {
                "calculator_only" => OrchestrationMode::CalculatorOnly,
                "llm_only" => OrchestrationMode::LlmOnly,
                "public_gateway" => OrchestrationMode::PublicGateway,
                "legacy_unified" => OrchestrationMode::LegacyUnified,
                _ => OrchestrationMode::CalculatorThenLlm,
            })
    }
}

//! Tests du catalogue providers/modeles (validation registry + mapping SQL).

use astral_llm_application::ModelCapabilityRegistry;
use astral_llm_domain::{
    model_capability::{ModelCapability, StructuredOutputAdapterKind},
    model_usage_tier::{ModelRouteContext, ModelUsageTierPolicy},
    provider::{ProviderKind, ReasoningEffort, StructuredOutputMode},
    GenerationErrorCode,
};
use astral_llm_infra::provider_catalog::row_to_capability;

#[test]
fn registry_rejects_oracle_on_primary_without_flag() {
    let mut registry = ModelCapabilityRegistry::from_db_catalog(vec!["openai".into()], vec![]);
    registry.register(oracle_gpt55_pro());
    let err = registry
        .validate_engine_for_context(
            ModelRouteContext::PrimaryReading,
            &ProviderKind::OpenAi,
            "gpt-5.5-pro",
            false,
        )
        .expect_err("oracle_only blocked on primary");
    assert_eq!(
        err.detail().code,
        GenerationErrorCode::UnsupportedCapability
    );
}

#[test]
fn registry_accepts_production_openai_models() {
    let registry = ModelCapabilityRegistry::from_db_catalog(
        vec!["openai".into()],
        vec![
            production_openai_gpt41(),
            ModelCapability {
                provider: ProviderKind::OpenAi,
                model: "gpt-5.4".into(),
                supports_json_schema_strict: true,
                supports_json_object: true,
                supports_reasoning_effort: true,
                supports_streaming: true,
                supports_native_safety_prompt: false,
                max_input_tokens: 1_050_000,
                max_output_tokens: 128_000,
                structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
                structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
                storage_disable_supported: true,
                is_active: true,
                supports_temperature: true,
                reasoning_output_reserve_min: Some(4096),
                reasoning_effort_subtask: Some(ReasoningEffort::None),
                reasoning_effort_primary: Some(ReasoningEffort::Low),
                reasoning_effort_oracle: Some(ReasoningEffort::Medium),
                usage_tier_code: Some("production_candidate".into()),
                tier_policy: production_tier(),
            },
        ],
    );
    for model in ["gpt-4.1", "gpt-5.4"] {
        registry
            .validate_engine_for_context(
                ModelRouteContext::PrimaryReading,
                &ProviderKind::OpenAi,
                model,
                false,
            )
            .unwrap_or_else(|e| panic!("{model} should be allowed: {e:?}"));
    }
}

#[test]
fn subtask_tier_allowed_for_summary_context() {
    let registry = ModelCapabilityRegistry::from_db_catalog(
        vec!["openai".into()],
        vec![ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-5-nano".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: true,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 400_000,
            max_output_tokens: 128_000,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
            supports_temperature: false,
            reasoning_output_reserve_min: Some(4096),
            reasoning_effort_subtask: Some(ReasoningEffort::None),
            reasoning_effort_primary: Some(ReasoningEffort::Low),
            reasoning_effort_oracle: Some(ReasoningEffort::Medium),
            usage_tier_code: Some("subtask_candidate".into()),
            tier_policy: ModelUsageTierPolicy {
                allows_primary_reading: false,
                allows_subtask: true,
                allows_oracle_benchmark: false,
            },
        }],
    );
    assert!(registry
        .validate_engine_for_context(
            ModelRouteContext::Subtask,
            &ProviderKind::OpenAi,
            "gpt-5-nano",
            false,
        )
        .is_ok());
    assert!(registry
        .validate_engine_for_context(
            ModelRouteContext::PrimaryReading,
            &ProviderKind::OpenAi,
            "gpt-5-nano",
            false,
        )
        .is_err());
    registry
        .validate_request_capabilities(
            ModelRouteContext::Subtask,
            &ProviderKind::OpenAi,
            "gpt-5-nano",
            None,
            true,
        )
        .expect("subtask route must accept nano for summary");
    assert!(registry
        .validate_request_capabilities(
            ModelRouteContext::PrimaryReading,
            &ProviderKind::OpenAi,
            "gpt-5-nano",
            None,
            true,
        )
        .is_err());
}

#[test]
fn row_mapping_sets_anthropic_native_safety() {
    use astral_llm_infra::provider_catalog::LlmProviderModelRow;

    let row = LlmProviderModelRow {
        id: 1,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-20250514".into(),
        catalog_notes: None,
        supports_json_schema_strict: true,
        supports_json_object: true,
        supports_reasoning_effort: false,
        supports_streaming: true,
        max_input_tokens: 200_000,
        max_output_tokens: 8192,
        structured_output_adapter: "anthropic_output_config_format".into(),
        storage_disable_supported: false,
        is_active: true,
        supports_temperature: true,
        reasoning_output_reserve_min: None,
        reasoning_effort_subtask: None,
        reasoning_effort_primary: None,
        reasoning_effort_oracle: None,
        usage_tier_code: Some("production_candidate".into()),
        allows_primary_reading: true,
        allows_subtask: true,
        allows_oracle_benchmark: false,
    };
    let cap = row_to_capability(&row).expect("mapped");
    assert_eq!(cap.reasoning_output_reserve(), 0);
    assert!(cap.supports_native_safety_prompt);
    assert_eq!(cap.usage_tier_code.as_deref(), Some("production_candidate"));
}

fn production_tier() -> ModelUsageTierPolicy {
    ModelUsageTierPolicy {
        allows_primary_reading: true,
        allows_subtask: true,
        allows_oracle_benchmark: false,
    }
}

fn production_openai_gpt41() -> ModelCapability {
    ModelCapability {
        provider: ProviderKind::OpenAi,
        model: "gpt-4.1".into(),
        supports_json_schema_strict: true,
        supports_json_object: true,
        supports_reasoning_effort: false,
        supports_streaming: true,
        supports_native_safety_prompt: false,
        max_input_tokens: 1_000_000,
        max_output_tokens: 32_000,
        structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
        structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
        storage_disable_supported: true,
        is_active: true,
        supports_temperature: true,
        reasoning_output_reserve_min: None,
        reasoning_effort_subtask: None,
        reasoning_effort_primary: None,
        reasoning_effort_oracle: None,
        usage_tier_code: Some("baseline".into()),
        tier_policy: production_tier(),
    }
}

fn oracle_gpt55_pro() -> ModelCapability {
    ModelCapability {
        provider: ProviderKind::OpenAi,
        model: "gpt-5.5-pro".into(),
        supports_json_schema_strict: true,
        supports_json_object: true,
        supports_reasoning_effort: true,
        supports_streaming: false,
        supports_native_safety_prompt: false,
        max_input_tokens: 1_050_000,
        max_output_tokens: 128_000,
        structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
        structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
        storage_disable_supported: true,
        is_active: true,
        supports_temperature: false,
        reasoning_output_reserve_min: Some(4096),
        reasoning_effort_subtask: Some(ReasoningEffort::None),
        reasoning_effort_primary: Some(ReasoningEffort::Low),
        reasoning_effort_oracle: Some(ReasoningEffort::Medium),
        usage_tier_code: Some("oracle_only".into()),
        tier_policy: ModelUsageTierPolicy {
            allows_primary_reading: false,
            allows_subtask: false,
            allows_oracle_benchmark: true,
        },
    }
}

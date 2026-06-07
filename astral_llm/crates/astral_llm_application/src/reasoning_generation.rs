//! Parametres de generation pour modeles a raisonnement (`supports_reasoning_effort`).

use astral_llm_domain::{
    model_capability::ModelCapability, model_usage_tier::ModelRouteContext,
    ProductGenerationPolicy, ReasoningEffort,
};

/// Budget de base pour sous-taches (summary, repair) hors contrat chapitre.
pub const SUBTASK_BASE_OUTPUT_TOKENS: u32 = 600;

/// Effort de raisonnement envoye au provider (toujours explicite si le modele le supporte).
pub fn resolve_reasoning_effort(
    cap: &ModelCapability,
    policy: &ProductGenerationPolicy,
    requested: Option<ReasoningEffort>,
    route: ModelRouteContext,
) -> Option<ReasoningEffort> {
    if !cap.supports_reasoning_effort {
        return None;
    }

    let effort = requested.unwrap_or_else(|| default_reasoning_effort(cap, route));
    Some(policy.caps_reasoning(Some(effort)))
}

fn default_reasoning_effort(cap: &ModelCapability, route: ModelRouteContext) -> ReasoningEffort {
    match route {
        ModelRouteContext::Subtask => cap.reasoning_effort_subtask.unwrap_or(ReasoningEffort::Low),
        ModelRouteContext::PrimaryReading => {
            cap.reasoning_effort_primary.unwrap_or(ReasoningEffort::Low)
        }
        ModelRouteContext::OracleBenchmark => cap
            .reasoning_effort_oracle
            .unwrap_or(ReasoningEffort::Medium),
    }
}

/// Temperature uniquement si le catalogue indique `supports_temperature`.
pub fn effective_temperature(cap: &ModelCapability, temperature: Option<f32>) -> Option<f32> {
    if cap.supports_temperature {
        temperature
    } else {
        None
    }
}

/// Applique le plancher canonique `reasoning_output_reserve_min` au budget demande.
pub fn apply_reasoning_output_reserve(cap: &ModelCapability, requested_tokens: u32) -> u32 {
    let mut tokens = requested_tokens;
    let reserve = cap.reasoning_output_reserve();
    if reserve > 0 {
        tokens = tokens.max(reserve);
    }
    tokens.min(cap.max_output_tokens.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        model_capability::{ModelCapability, StructuredOutputAdapterKind},
        model_usage_tier::ModelUsageTierPolicy,
        provider::{ProviderKind, StructuredOutputMode},
    };

    fn gpt5_mini_cap() -> ModelCapability {
        ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-5-mini".into(),
            supports_reasoning_effort: true,
            reasoning_effort_subtask: Some(ReasoningEffort::Minimal),
            reasoning_effort_primary: Some(ReasoningEffort::Low),
            reasoning_effort_oracle: Some(ReasoningEffort::Medium),
            reasoning_output_reserve_min: Some(4096),
            max_output_tokens: 16_000,
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 400_000,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
            supports_temperature: false,
            usage_tier_code: None,
            tier_policy: ModelUsageTierPolicy::unrestricted(),
        }
    }

    #[test]
    fn subtask_defaults_to_minimal_effort() {
        let cap = gpt5_mini_cap();
        let policy = ProductGenerationPolicy::bootstrap_natal_prompter();
        let effort = resolve_reasoning_effort(&cap, &policy, None, ModelRouteContext::Subtask);
        assert_eq!(effort, Some(ReasoningEffort::Minimal));
    }

    fn gpt54_cap() -> ModelCapability {
        ModelCapability {
            model: "gpt-5.4".into(),
            reasoning_effort_subtask: Some(ReasoningEffort::None),
            reasoning_effort_primary: Some(ReasoningEffort::Low),
            reasoning_effort_oracle: Some(ReasoningEffort::Medium),
            ..gpt5_mini_cap()
        }
    }

    #[test]
    fn frontier_subtask_defaults_to_none_effort() {
        let cap = gpt54_cap();
        let policy = ProductGenerationPolicy::bootstrap_natal_prompter();
        let effort = resolve_reasoning_effort(&cap, &policy, None, ModelRouteContext::Subtask);
        assert_eq!(effort, Some(ReasoningEffort::None));
    }

    #[test]
    fn primary_defaults_to_low_effort() {
        let cap = gpt5_mini_cap();
        let policy = ProductGenerationPolicy::bootstrap_natal_prompter();
        let effort =
            resolve_reasoning_effort(&cap, &policy, None, ModelRouteContext::PrimaryReading);
        assert_eq!(effort, Some(ReasoningEffort::Low));
    }

    #[test]
    fn reserve_raises_subtask_budget() {
        let cap = gpt5_mini_cap();
        assert_eq!(
            apply_reasoning_output_reserve(&cap, SUBTASK_BASE_OUTPUT_TOKENS),
            4096
        );
    }

    #[test]
    fn non_reasoning_model_unchanged() {
        let cap = ModelCapability {
            reasoning_output_reserve_min: None,
            supports_reasoning_effort: false,
            max_output_tokens: 32_000,
            ..gpt5_mini_cap()
        };
        assert_eq!(
            apply_reasoning_output_reserve(&cap, SUBTASK_BASE_OUTPUT_TOKENS),
            SUBTASK_BASE_OUTPUT_TOKENS
        );
    }
}

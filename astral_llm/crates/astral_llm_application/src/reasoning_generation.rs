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

use astral_llm_domain::{
    GenerateReadingRequest, GenerationError, GenerationErrorCode, NatalReadingResponse,
    ProductGenerationPolicy, SafetyPolicy,
};

use crate::execution_audit::ExecutionAudit;
use crate::generate_reading_use_case::GenerateReadingUseCase;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::reading_script_guard::violations_are_script_only;
use crate::safety_guard::SafetyGuard;
use crate::simplified_reading::{SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_PROFILE};
use crate::simplified_reading_guard::violations_are_ambiguous_core_only;
use crate::simplified_reading_postprocess::{
    apply_simplified_body_fallback, post_process_single_pass_reading, SCRIPT_REPAIR_INSTRUCTION,
};

impl GenerateReadingUseCase {
    pub(super) async fn generate_single_pass_hardened(
        &self,
        request: &GenerateReadingRequest,
        engine: &crate::engine_defaults::ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        astro_facts: &astral_llm_domain::NormalizedAstroFacts,
        domains: &[String],
        product_policy: &ProductGenerationPolicy,
        interpretation: Option<&ResolvedInterpretationContext>,
        run_id: &str,
        audit: &mut ExecutionAudit,
    ) -> Result<NatalReadingResponse, GenerationError> {
        let is_simplified = request
            .product_context
            .interpretation_profile_code
            .as_deref()
            == Some(SIMPLIFIED_PROFILE);
        let max_attempts = max_script_generation_attempts(interpretation, is_simplified);
        let chapter_code = request
            .response_contract
            .chapters
            .first()
            .map(|c| c.code.as_str())
            .unwrap_or("identity");

        let mut repair_instruction: Option<&str> = None;
        let mut last_violations: Vec<String> = Vec::new();

        for attempt in 0..max_attempts {
            let single_pass = self
                .generate_single_pass(
                    request,
                    engine,
                    safety_policy,
                    astro_facts,
                    domains,
                    product_policy,
                    interpretation,
                    run_id,
                    repair_instruction,
                )
                .await?;
            audit.record_chapter_step(
                chapter_code,
                &single_pass.used_provider,
                &single_pass.used_model,
                if attempt > 0 {
                    astral_llm_domain::ChapterGenerationStatus::Repaired
                } else {
                    astral_llm_domain::ChapterGenerationStatus::Generated
                },
                single_pass.token_usage.clone(),
                single_pass.latency_ms,
                None,
                Some("single_pass_generate"),
            );
            let mut reading = single_pass.reading;

            let post_audit =
                post_process_single_pass_reading(&mut reading, request, interpretation);
            if !post_audit.dash_normalized_fields.is_empty() {
                audit.push_step(astral_llm_domain::GenerationStepRecord {
                    step_type: "dash_normalize".into(),
                    chapter_code: Some(chapter_code.to_string()),
                    provider: reading.quality.used_provider.clone(),
                    model: reading.quality.used_model.clone(),
                    status: astral_llm_domain::ChapterGenerationStatus::Repaired,
                    token_usage: None,
                    input_tokens: None,
                    output_tokens: None,
                    latency_ms: None,
                    error_code: Some(format!(
                        "dash_normalized_fields={}",
                        post_audit.dash_normalized_fields.join(",")
                    )),
                });
            }
            if !post_audit.sanitized_fields.is_empty() {
                audit.push_step(astral_llm_domain::GenerationStepRecord {
                    step_type: "script_sanitize".into(),
                    chapter_code: Some(chapter_code.to_string()),
                    provider: reading.quality.used_provider.clone(),
                    model: reading.quality.used_model.clone(),
                    status: astral_llm_domain::ChapterGenerationStatus::Repaired,
                    token_usage: None,
                    input_tokens: None,
                    output_tokens: None,
                    latency_ms: None,
                    error_code: Some(format!(
                        "sanitized_fields={}",
                        post_audit.sanitized_fields.join(",")
                    )),
                });
            }
            if post_audit.ambiguous_core_hardening.any_applied() {
                let h = &post_audit.ambiguous_core_hardening;
                audit.push_step(astral_llm_domain::GenerationStepRecord {
                    step_type: "ambiguous_core_hardening".into(),
                    chapter_code: Some(chapter_code.to_string()),
                    provider: reading.quality.used_provider.clone(),
                    model: reading.quality.used_model.clone(),
                    status: astral_llm_domain::ChapterGenerationStatus::Repaired,
                    token_usage: None,
                    input_tokens: None,
                    output_tokens: None,
                    latency_ms: None,
                    error_code: Some(format!(
                        "code_corrected={} confidence_clamped={} basis_pruned={} prefix={}",
                        h.chapter_code_corrected,
                        h.confidence_clamped,
                        h.basis_pruned,
                        h.uncertainty_prefix_applied
                    )),
                });
            }

            match self.validate_single_pass_output(request, &reading, safety_policy, is_simplified)
            {
                Ok(()) => {
                    if attempt > 0 {
                        audit.push_step(astral_llm_domain::GenerationStepRecord {
                            step_type: "script_repair".into(),
                            chapter_code: Some(chapter_code.to_string()),
                            provider: reading.quality.used_provider.clone(),
                            model: reading.quality.used_model.clone(),
                            status: astral_llm_domain::ChapterGenerationStatus::Repaired,
                            token_usage: None,
                            input_tokens: None,
                            output_tokens: None,
                            latency_ms: None,
                            error_code: Some(format!("attempt={}", attempt + 1)),
                        });
                    }
                    return Ok(reading);
                }
                Err(violations)
                    if violations_are_script_only(&violations) && attempt + 1 < max_attempts =>
                {
                    last_violations = violations;
                    repair_instruction = Some(SCRIPT_REPAIR_INSTRUCTION);
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts,
                        "script contamination — retrying single_pass with repair instruction"
                    );
                }
                Err(violations) if violations_are_script_only(&violations) && is_simplified => {
                    apply_simplified_body_fallback(&mut reading, chapter_code);
                    let _ = post_process_single_pass_reading(&mut reading, request, interpretation);
                    if self
                        .validate_single_pass_output(request, &reading, safety_policy, true)
                        .is_ok()
                    {
                        reading.quality.fallback_used = true;
                        audit.push_step(astral_llm_domain::GenerationStepRecord {
                            step_type: "script_body_fallback".into(),
                            chapter_code: Some(chapter_code.to_string()),
                            provider: reading.quality.used_provider.clone(),
                            model: reading.quality.used_model.clone(),
                            status: astral_llm_domain::ChapterGenerationStatus::Repaired,
                            token_usage: None,
                            input_tokens: None,
                            output_tokens: None,
                            latency_ms: None,
                            error_code: None,
                        });
                        return Ok(reading);
                    }
                    last_violations = violations;
                    break;
                }
                Err(violations)
                    if violations_are_ambiguous_core_only(&violations) && is_simplified =>
                {
                    apply_simplified_body_fallback(&mut reading, SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
                    let _ = post_process_single_pass_reading(&mut reading, request, interpretation);
                    if self
                        .validate_single_pass_output(request, &reading, safety_policy, true)
                        .is_ok()
                    {
                        reading.quality.fallback_used = true;
                        audit.push_step(astral_llm_domain::GenerationStepRecord {
                            step_type: "ambiguous_core_body_fallback".into(),
                            chapter_code: Some(SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE.to_string()),
                            provider: reading.quality.used_provider.clone(),
                            model: reading.quality.used_model.clone(),
                            status: astral_llm_domain::ChapterGenerationStatus::Repaired,
                            token_usage: None,
                            input_tokens: None,
                            output_tokens: None,
                            latency_ms: None,
                            error_code: None,
                        });
                        return Ok(reading);
                    }
                    last_violations = violations;
                    break;
                }
                Err(violations) => {
                    return Err(safety_validation_error(&violations));
                }
            }
        }

        let fallback_violations = vec!["generated content failed safety validation".to_string()];
        let violations_ref = if last_violations.is_empty() {
            &fallback_violations
        } else {
            &last_violations
        };
        Err(safety_validation_error(violations_ref))
    }

    fn validate_single_pass_output(
        &self,
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
        safety_policy: &SafetyPolicy,
        is_simplified: bool,
    ) -> Result<(), Vec<String>> {
        if is_simplified {
            self.validate_simplified_reading(request, reading)
                .map_err(|err| extract_violations(&err))?;
        }

        SafetyGuard::validate_response(
            reading,
            safety_policy,
            &request.astrologer_profile.forbidden_wording,
            self.catalog.shared_catalog(),
        )
        .map_err(|mut violations| {
            if is_simplified {
                violations.retain(|v| !v.contains("legal.disclaimer"));
            }
            violations
        })
    }
}

fn max_script_generation_attempts(
    interpretation: Option<&ResolvedInterpretationContext>,
    is_simplified: bool,
) -> usize {
    interpretation
        .map(|ctx| usize::from(ctx.profile.max_script_repair_attempts()))
        .unwrap_or(if is_simplified { 2 } else { 1 })
}

fn extract_violations(err: &GenerationError) -> Vec<String> {
    err.detail()
        .details
        .as_ref()
        .and_then(|d| d.get("violations"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| vec![err.detail().message.clone()])
}

fn safety_validation_error(violations: &[String]) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::PostSafetyValidationFailed,
        "generated content failed safety validation",
        serde_json::json!({ "violations": violations }),
    )
}

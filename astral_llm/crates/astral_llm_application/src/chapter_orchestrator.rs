use std::time::{Duration, Instant};

use astral_llm_domain::{
    chapter_orchestration::{ChapterGenerationStatus, ReadingPlan, ReadingPlanChapter},
    interpretation_profile::SYNTHESIS_CHAPTER_CODE,
    generation_response::{
        ChapterProviderResponse, LegalBlock, NatalReadingResponse, QualityMetadata,
        ReadingChapter,
    },
    output_contract::GenerationMode,
    GenerateReadingRequest, GenerationError, GenerationErrorCode, NormalizedAstroFacts,
    ProductGenerationPolicy, SafetyPolicy, SafetyMode,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};
use uuid::Uuid;

use astral_llm_domain::default_legal_disclaimer;

use crate::astro_basis_validator::AstroBasisValidator;
use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::chapter_evidence_basis_enricher::ChapterEvidenceBasisEnricher;
use crate::chapter_evidence_coherence::ChapterEvidenceCoherence;
use crate::chapter_writing_guidance::ChapterWritingGuidance;
use crate::chapter_evidence_planner::{pack_for_chapter, ChapterEvidencePlanner};
use crate::evidence_diversity_validator::{compute_evidence_metrics, EvidenceDiversityValidator};
use crate::reading_opening_diversity_validator::ReadingOpeningDiversityValidator;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::interpretive_evidence_builder::{evidence_enabled_for_request, InterpretiveEvidenceBuilder};
use crate::chapter_quality_repair::{
    append_repair_instructions, is_min_words_violation, length_repair_from_error,
    maybe_repair_repetition, retry_chapter_on_min_words, ChapterRepairKind,
};
use crate::domain_resolver::DomainResolver;
use crate::engine_defaults::{
    drop_unsupported_reasoning, drop_unsupported_temperature, resolve_subtask_engine,
    ResolvedEngineParams,
};
use crate::execution_audit::ExecutionAudit;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::prompt_trace;
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reading_plan::ReadingPlanBuilder;
use crate::reading_quality_validator::PremiumQualityThresholds;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::final_synthesis_synthesizer::FinalSynthesisSynthesizer;
use crate::summary_synthesizer::SummarySynthesizer;
use crate::reasoning_generation::{effective_temperature, resolve_reasoning_effort};
use crate::token_budget::TokenBudget;
use astral_llm_domain::ServiceLimits;

pub struct ChapterOrchestrator<'a> {
    router: &'a ProviderRouter,
    compiler: &'a PromptCompiler,
    validator: &'a ResponseValidator,
    catalog: &'a SharedCanonicalCatalog,
    limits: &'a ServiceLimits,
}

pub struct OrchestratedResult {
    pub reading: NatalReadingResponse,
    pub plan: ReadingPlan,
    pub chapter_packs: Vec<astral_llm_domain::ChapterEvidencePack>,
    pub evidence_metrics: Option<astral_llm_domain::EvidenceMetrics>,
}

impl<'a> ChapterOrchestrator<'a> {
    pub fn new(
        router: &'a ProviderRouter,
        compiler: &'a PromptCompiler,
        validator: &'a ResponseValidator,
        catalog: &'a SharedCanonicalCatalog,
        limits: &'a ServiceLimits,
    ) -> Self {
        Self {
            router,
            compiler,
            validator,
            catalog,
            limits,
        }
    }

    pub async fn generate(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        astro_facts: &NormalizedAstroFacts,
        product_policy: &ProductGenerationPolicy,
        interpretation: Option<&ResolvedInterpretationContext>,
        run_id: &str,
        audit: &mut ExecutionAudit,
    ) -> Result<OrchestratedResult, GenerationError> {
        let domains = DomainResolver::resolve(
            request,
            self.catalog,
            self.limits,
            product_policy,
            interpretation,
        );
        audit.selected_domains = domains.clone();

        let plan = ReadingPlanBuilder::build(request, &domains, interpretation);
        ReadingPlanBuilder::validate(&plan, interpretation)?;

        let contracts = ReadingPlanBuilder::to_chapter_contracts(&plan);
        let quality_thresholds =
            crate::reading_quality_validator::thresholds_for_request(request, interpretation);
        let writing_locale =
            AstroLabelHumanizer::locale_key(&request.product_context.user_language);

        let pool = InterpretiveEvidenceBuilder::build(astro_facts, &self.catalog.evidence)?;
        let evidence_enabled = evidence_enabled_for_request(interpretation);
        let evidence_policy = interpretation
            .and_then(|ctx| ctx.profile.to_premium_evidence_policy())
            .unwrap_or_else(|| self.catalog.evidence.premium_policy.clone());

        let mut requirement_audit = Vec::new();
        let chapter_packs = if evidence_enabled {
            let packs = ChapterEvidencePlanner::plan_all(
                &pool,
                &plan,
                &self.catalog.evidence,
                &evidence_policy,
            )?;
            requirement_audit = EvidenceDiversityValidator::validate_packs_planned(
                evidence_enabled,
                &pool,
                &packs,
                &self.catalog.evidence,
                &evidence_policy,
            )?;
            packs
        } else {
            Vec::new()
        };

        let mut generated = Vec::new();
        let mut last_bundle = None;
        let mut used_provider = engine.provider.as_str().to_string();
        let mut used_model = engine.model.clone();
        let mut fallback_used = false;

        for (chapter, contract) in plan
            .chapters
            .iter()
            .zip(contracts.iter())
            .filter(|(ch, _)| ch.code != SYNTHESIS_CHAPTER_CODE)
        {
            let chapter_pack = pack_for_chapter(&chapter_packs, &chapter.code);
            let started = Instant::now();
            match self
                .generate_one_chapter(
                    request,
                    engine,
                    safety_policy,
                    astro_facts,
                    chapter_pack,
                    chapter,
                    contract,
                    &generated,
                    run_id,
                    product_policy,
                    interpretation,
                    &[],
                )
                .await
            {
                Ok((reading_chapter, bundle, route_meta)) => {
                    let outcome = maybe_repair_repetition(
                        chapter,
                        reading_chapter,
                        bundle,
                        route_meta,
                        &quality_thresholds,
                        writing_locale,
                        run_id,
                        engine,
                        started,
                        audit,
                        |repair| async {
                            let repair_buf: Vec<ChapterRepairKind> = repair.into_iter().collect();
                            self.generate_one_chapter(
                                request,
                                engine,
                                safety_policy,
                                astro_facts,
                                chapter_pack,
                                chapter,
                                contract,
                                &generated,
                                run_id,
                                product_policy,
                                interpretation,
                                &repair_buf,
                            )
                            .await
                        },
                    )
                    .await?;
                    last_bundle = Some(outcome.bundle);
                    used_provider = outcome.route_meta.0.clone();
                    used_model = outcome.route_meta.1.clone();
                    fallback_used |= outcome.route_meta.2;
                    audit.record_chapter_step(
                        &chapter.code,
                        &used_provider,
                        &used_model,
                        outcome.status,
                        outcome.route_meta.3,
                        outcome.route_meta.4,
                        started.elapsed().as_millis() as u64,
                        None,
                    );
                    generated.push(outcome.reading_chapter);
                }
                Err(err) if is_evidence_coherence_violation(&err) && chapter_pack.is_some() =>
                {
                    let repair_kind = evidence_repair_from_error(&err).unwrap_or(
                        ChapterRepairKind::EvidenceCoherence {
                            missing_pack_fact_ids: vec![],
                            orphan_object_codes: vec![],
                        },
                    );
                    match self
                        .generate_one_chapter(
                            request,
                            engine,
                            safety_policy,
                            astro_facts,
                            chapter_pack,
                            chapter,
                            contract,
                            &generated,
                            run_id,
                            product_policy,
                            interpretation,
                            std::slice::from_ref(&repair_kind),
                        )
                        .await
                    {
                        Ok((reading_chapter, bundle, route_meta)) => {
                            let outcome = maybe_repair_repetition(
                                chapter,
                                reading_chapter,
                                bundle,
                                route_meta,
                                &quality_thresholds,
                                writing_locale,
                                run_id,
                                engine,
                                started,
                                audit,
                                |repair| async {
                                    let repair_buf: Vec<ChapterRepairKind> =
                                        repair.into_iter().collect();
                                    self.generate_one_chapter(
                                        request,
                                        engine,
                                        safety_policy,
                                        astro_facts,
                                        chapter_pack,
                                        chapter,
                                        contract,
                                        &generated,
                                        run_id,
                                        product_policy,
                                        interpretation,
                                        &repair_buf,
                                    )
                                    .await
                                },
                            )
                            .await?;
                            last_bundle = Some(outcome.bundle);
                            used_provider = outcome.route_meta.0.clone();
                            used_model = outcome.route_meta.1.clone();
                            fallback_used |= outcome.route_meta.2;
                            audit.record_chapter_step(
                                &chapter.code,
                                &used_provider,
                                &used_model,
                                ChapterGenerationStatus::Repaired,
                                outcome.route_meta.3,
                                outcome.route_meta.4,
                                started.elapsed().as_millis() as u64,
                                None,
                            );
                            generated.push(outcome.reading_chapter);
                        }
                        Err(repair_err) => {
                            audit.record_chapter_step(
                                &chapter.code,
                                engine.provider.as_str(),
                                &engine.model,
                                ChapterGenerationStatus::AstroBasisInvalid,
                                None,
                                None,
                                started.elapsed().as_millis() as u64,
                                Some(repair_err.detail().code.as_str().to_string()),
                            );
                            return Err(repair_err);
                        }
                    }
                }
                Err(err) if is_min_words_violation(&err) => {
                    let (reading_chapter, bundle, route_meta) = retry_chapter_on_min_words(
                        chapter,
                        err,
                        run_id,
                        engine,
                        started,
                        audit,
                        |repair| async {
                            let repair_buf: Vec<ChapterRepairKind> = repair.into_iter().collect();
                            self.generate_one_chapter(
                                request,
                                engine,
                                safety_policy,
                                astro_facts,
                                chapter_pack,
                                chapter,
                                contract,
                                &generated,
                                run_id,
                                product_policy,
                                interpretation,
                                &repair_buf,
                            )
                            .await
                        },
                    )
                    .await?;
                    let outcome = maybe_repair_repetition(
                        chapter,
                        reading_chapter,
                        bundle,
                        route_meta,
                        &quality_thresholds,
                        writing_locale,
                        run_id,
                        engine,
                        started,
                        audit,
                        |repair| async {
                            let repair_buf: Vec<ChapterRepairKind> = repair.into_iter().collect();
                            self.generate_one_chapter(
                                request,
                                engine,
                                safety_policy,
                                astro_facts,
                                chapter_pack,
                                chapter,
                                contract,
                                &generated,
                                run_id,
                                product_policy,
                                interpretation,
                                &repair_buf,
                            )
                            .await
                        },
                    )
                    .await?;
                    last_bundle = Some(outcome.bundle);
                    used_provider = outcome.route_meta.0.clone();
                    used_model = outcome.route_meta.1.clone();
                    fallback_used |= outcome.route_meta.2;
                    audit.record_chapter_step(
                        &chapter.code,
                        &used_provider,
                        &used_model,
                        ChapterGenerationStatus::Repaired,
                        outcome.route_meta.3,
                        outcome.route_meta.4,
                        started.elapsed().as_millis() as u64,
                        None,
                    );
                    generated.push(outcome.reading_chapter);
                }
                Err(err) => {
                    let status = if matches!(
                        err.detail().code,
                        GenerationErrorCode::PostSafetyValidationFailed
                            | GenerationErrorCode::SafetyRejected
                    ) {
                        ChapterGenerationStatus::SafetyRejected
                    } else if matches!(err.detail().code, GenerationErrorCode::SchemaValidationFailed) {
                        ChapterGenerationStatus::AstroBasisInvalid
                    } else {
                        ChapterGenerationStatus::Failed
                    };
                    audit.record_chapter_step(
                        &chapter.code,
                        engine.provider.as_str(),
                        &engine.model,
                        status,
                        None,
                        None,
                        started.elapsed().as_millis() as u64,
                        Some(err.detail().code.as_str().to_string()),
                    );
                    return Err(err);
                }
            }
        }

        let bundle = last_bundle.expect("at least one chapter");

        let summary_started = Instant::now();
        let mut summary_engine = resolve_subtask_engine(
            engine,
            &request.engine,
            Some(product_policy),
        );
        let registry = self.router.capability_registry();
        drop_unsupported_reasoning(&mut summary_engine, registry);
        drop_unsupported_temperature(&mut summary_engine, registry);
        let synthesizer =
            SummarySynthesizer::new(self.router, self.validator, self.catalog);
        let summary_result = synthesizer
            .synthesize(request, &generated, &summary_engine, safety_policy, run_id)
            .await?;
        audit.record_chapter_step(
            "summary",
            summary_engine.provider.as_str(),
            &summary_engine.model,
            ChapterGenerationStatus::Generated,
            summary_result.input_tokens,
            summary_result.output_tokens,
            summary_started.elapsed().as_millis() as u64,
            None,
        );

        if interpretation.is_some_and(|ctx| ctx.profile.has_final_synthesis_chapter()) {
            if let Some((synthesis_chapter, synthesis_contract)) = plan
                .chapters
                .iter()
                .zip(contracts.iter())
                .find(|(ch, _)| ch.code == SYNTHESIS_CHAPTER_CODE)
            {
                let synthesis_pack = pack_for_chapter(&chapter_packs, SYNTHESIS_CHAPTER_CODE);
                let synthesis_started = Instant::now();
                let final_synthesizer =
                    FinalSynthesisSynthesizer::new(self.router, self.validator, self.catalog);
                const MAX_SYNTHESIS_ATTEMPTS: usize = 2;
                let mut synthesis_repair: Option<crate::chapter_quality_repair::ChapterRepairKind> =
                    None;
                let mut synthesis_result = None;
                for attempt in 0..MAX_SYNTHESIS_ATTEMPTS {
                    match final_synthesizer
                        .synthesize(
                            request,
                            &generated,
                            synthesis_chapter,
                            synthesis_contract,
                            synthesis_pack,
                            astro_facts,
                            engine,
                            safety_policy,
                            product_policy,
                            interpretation,
                            run_id,
                            synthesis_repair.clone(),
                        )
                        .await
                    {
                        Ok(result) => {
                            let words = result.chapter.body.split_whitespace().count() as u32;
                            if words >= synthesis_chapter.min_words {
                                synthesis_result = Some(result);
                                break;
                            }
                            if attempt + 1 >= MAX_SYNTHESIS_ATTEMPTS {
                                return Err(GenerationError::with_details(
                                    GenerationErrorCode::ReadingQualityFailed,
                                    format!(
                                        "synthesis chapter too short ({words} words, min {})",
                                        synthesis_chapter.min_words
                                    ),
                                    serde_json::json!({
                                        "chapter": SYNTHESIS_CHAPTER_CODE,
                                        "words": words,
                                        "min_words": synthesis_chapter.min_words,
                                    }),
                                ));
                            }
                            synthesis_repair = Some(
                                crate::chapter_quality_repair::ChapterRepairKind::TooShort {
                                    words,
                                    min_words: synthesis_chapter.min_words,
                                    max_words: synthesis_chapter.max_words,
                                },
                            );
                        }
                        Err(err) => {
                            audit.record_chapter_step(
                                SYNTHESIS_CHAPTER_CODE,
                                engine.provider.as_str(),
                                &engine.model,
                                ChapterGenerationStatus::Failed,
                                None,
                                None,
                                synthesis_started.elapsed().as_millis() as u64,
                                Some(err.detail().code.as_str().to_string()),
                            );
                            return Err(err);
                        }
                    }
                }
                let synthesis_result = synthesis_result.expect("synthesis generated");
                audit.record_chapter_step(
                    SYNTHESIS_CHAPTER_CODE,
                    engine.provider.as_str(),
                    &engine.model,
                    if synthesis_repair.is_some() {
                        ChapterGenerationStatus::Repaired
                    } else {
                        ChapterGenerationStatus::Generated
                    },
                    synthesis_result.input_tokens,
                    synthesis_result.output_tokens,
                    synthesis_started.elapsed().as_millis() as u64,
                    None,
                );
                generated.push(synthesis_result.chapter);
            }
        }

        if evidence_enabled {
            self.repair_opening_duplicates(
                request,
                engine,
                safety_policy,
                astro_facts,
                &chapter_packs,
                &plan,
                &contracts,
                &quality_thresholds,
                writing_locale,
                run_id,
                product_policy,
                interpretation,
                &mut generated,
                audit,
            )
            .await?;

            EvidenceDiversityValidator::validate_reading(
                evidence_enabled,
                &pool,
                &generated,
                &chapter_packs,
            )?;
            ReadingOpeningDiversityValidator::validate(&generated, writing_locale)?;
        }

        let reading = NatalReadingResponse {
            schema_version: request.response_contract.output_schema_version.clone(),
            language: request.product_context.user_language.clone(),
            reading_type: request.product_context.product_code.clone(),
            summary: summary_result.summary,
            chapters: generated,
            legal: LegalBlock {
                disclaimer: default_legal_disclaimer(
                    &request.product_context.user_language,
                    request.response_contract.include_legal_disclaimer,
                ),
            },
            quality: QualityMetadata {
                used_provider,
                used_model,
                generation_mode: GenerationMode::ChapterOrchestrated,
                prompt_family: bundle.prompt_family,
                prompt_version: bundle.prompt_version,
                astro_contract_version: request.astro_result.contract_version.clone(),
                fallback_used,
            },
        };

        let evidence_metrics = if evidence_enabled {
            Some(compute_evidence_metrics(
                &chapter_packs,
                &reading.chapters,
                requirement_audit,
            ))
        } else {
            None
        };

        Ok(OrchestratedResult {
            reading,
            plan,
            chapter_packs,
            evidence_metrics,
        })
    }

    async fn repair_opening_duplicates(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        astro_facts: &NormalizedAstroFacts,
        chapter_packs: &[astral_llm_domain::ChapterEvidencePack],
        plan: &ReadingPlan,
        contracts: &[astral_llm_domain::output_contract::ChapterContract],
        _quality_thresholds: &PremiumQualityThresholds,
        locale: &str,
        run_id: &str,
        product_policy: &ProductGenerationPolicy,
        interpretation: Option<&ResolvedInterpretationContext>,
        generated: &mut Vec<ReadingChapter>,
        audit: &mut ExecutionAudit,
    ) -> Result<(), GenerationError> {
        const MAX_ROUNDS: usize = 8;

        for round in 0..MAX_ROUNDS {
            let violations = ReadingOpeningDiversityValidator::detect(generated, locale);
            if violations.is_empty() {
                return Ok(());
            }

            let Some(first_violation) = violations
                .iter()
                .find(|v| v.kind.contains("duplicate"))
            else {
                break;
            };
            let target_code = first_violation.chapter_code.clone();

            let mut any_repaired = false;
            {
                let Some(idx) = generated.iter().position(|c| c.code == target_code) else {
                    continue;
                };
                let Some(chapter) = plan.chapters.iter().find(|c| c.code == target_code) else {
                    continue;
                };
                let Some(contract) = contracts.iter().find(|c| c.code == target_code) else {
                    continue;
                };
                let prior: Vec<ReadingChapter> = generated[..idx].to_vec();
                let pack = pack_for_chapter(chapter_packs, &target_code);
                let chapter_violations: Vec<_> = violations
                    .iter()
                    .filter(|v| v.chapter_code == target_code && v.kind.contains("duplicate"))
                    .cloned()
                    .collect();
                let started = Instant::now();
                let mut repairs = vec![ChapterRepairKind::OpeningDiversity {
                    violations: chapter_violations,
                }];
                for opening_attempt in 0..2 {
                    match self
                        .generate_one_chapter(
                            request,
                            engine,
                            safety_policy,
                            astro_facts,
                            pack,
                            chapter,
                            contract,
                            &prior,
                            run_id,
                            product_policy,
                            interpretation,
                            &repairs,
                        )
                        .await
                    {
                        Ok((repaired, _bundle, meta)) => {
                            audit.record_chapter_step(
                                &target_code,
                                &meta.0,
                                &meta.1,
                                ChapterGenerationStatus::Repaired,
                                meta.3,
                                meta.4,
                                started.elapsed().as_millis() as u64,
                                None,
                            );
                            generated[idx] = repaired;
                            any_repaired = true;
                            break;
                        }
                        Err(err)
                            if is_min_words_violation(&err)
                                && opening_attempt == 0
                                && !repairs.iter().any(|r| {
                                    matches!(r, ChapterRepairKind::TooShort { .. })
                                }) =>
                        {
                            repairs.push(length_repair_from_error(&err, chapter));
                            tracing::info!(
                                run_id,
                                round,
                                chapter = %target_code,
                                "opening diversity repair too short; retrying with min_words expansion"
                            );
                        }
                        Err(err) => {
                            tracing::warn!(
                                run_id,
                                round,
                                chapter = %target_code,
                                error = %err.detail().message,
                                "opening diversity repair failed"
                            );
                            break;
                        }
                    }
                }
            }

            if !any_repaired {
                break;
            }
        }

        Ok(())
    }

    async fn generate_one_chapter(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        astro_facts: &NormalizedAstroFacts,
        chapter_pack: Option<&astral_llm_domain::ChapterEvidencePack>,
        chapter: &ReadingPlanChapter,
        contract: &astral_llm_domain::output_contract::ChapterContract,
        prior_chapters: &[ReadingChapter],
        run_id: &str,
        product_policy: &ProductGenerationPolicy,
        interpretation: Option<&ResolvedInterpretationContext>,
        repairs: &[ChapterRepairKind],
    ) -> Result<
        (
            ReadingChapter,
            crate::prompt_compiler::PromptBundle,
            (String, String, bool, Option<u32>, Option<u32>),
        ),
        GenerationError,
    > {
        ProductPolicyValidator::validate_against_policy(
            request,
            product_policy,
            &engine.provider,
            &engine.model,
        )?;

        let mut bundle = self
            .compiler
            .compile(PromptCompilationInput {
                request,
                safety_policy,
                astro_facts,
                selected_domains: &[chapter.code.clone()],
                chapter_code: Some(&chapter.code),
                chapter_evidence_pack: chapter_pack,
                catalog: self.catalog,
                interpretation,
            })
            .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;

        ChapterWritingGuidance::append_upstream_directives(
            &mut bundle,
            chapter,
            prior_chapters,
            chapter_pack,
            &request.product_context.user_language,
            interpretation,
        );

        for repair_kind in repairs {
            append_repair_instructions(&mut bundle, chapter, repair_kind.clone());
            if let ChapterRepairKind::OpeningDiversity { violations } = repair_kind {
                ReadingOpeningDiversityValidator::append_opening_repair_directives(
                    &mut bundle,
                    chapter,
                    prior_chapters,
                    &AstroLabelHumanizer::locale_key(&request.product_context.user_language),
                    violations,
                );
            }
        }

        let messages = self.compiler.to_provider_messages(&bundle);
        let attempt = if repairs.is_empty() {
            "primary"
        } else if repairs.iter().any(|r| matches!(r, ChapterRepairKind::OpeningDiversity { .. }))
            && repairs.iter().any(|r| matches!(r, ChapterRepairKind::TooShort { .. }))
        {
            "repair_opening_too_short"
        } else if let Some(primary) = repairs.last() {
            match primary {
                ChapterRepairKind::TooShort { .. } => "repair_too_short",
                ChapterRepairKind::Repetition { .. } => "repair_repetition",
                ChapterRepairKind::EvidenceCoherence { .. } => "repair_evidence",
                ChapterRepairKind::OpeningDiversity { .. } => "repair_opening",
            }
        } else {
            "primary"
        };
        prompt_trace::log_prompt_bundle(
            run_id,
            Some(&chapter.code),
            &bundle,
            self.compiler,
            Some(attempt),
        );
        let route_context = if repairs.is_empty() {
            astral_llm_domain::ModelRouteContext::PrimaryReading
        } else {
            astral_llm_domain::ModelRouteContext::Subtask
        };
        self.router.capability_registry().validate_engine_for_context(
            route_context,
            &engine.provider,
            &engine.model,
            engine.allow_oracle_benchmark,
        )?;

        let canonical_schema = self
            .validator
            .schema_registry()
            .provider_schema("chapter_provider_v1")
            .cloned();
        let model_cap = self
            .router
            .capability_registry()
            .require(&engine.provider, &engine.model)?;
        let schema = canonical_schema
            .as_ref()
            .map(|s| {
                let mut provider_schema = s.clone();
                crate::provider_schema_compiler::pin_chapter_code(&mut provider_schema, &chapter.code);
                ProviderSchemaCompiler::compile(&provider_schema, model_cap)
            })
            .transpose()?;

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: schema,
            reasoning_effort: resolve_reasoning_effort(
                model_cap,
                product_policy,
                engine.reasoning_effort,
                route_context,
            ),
            temperature: effective_temperature(model_cap, engine.temperature),
            max_output_tokens: Some(TokenBudget::chapter_max_tokens(
                contract,
                request
                    .engine
                    .max_output_tokens
                    .or(request.response_contract.global_max_tokens),
                model_cap,
            )),
            safety_mode: resolve_safety_mode(&engine.provider),
            timeout: Duration::from_millis(engine.timeout_ms.unwrap_or(120_000)),
            metadata: GenerationMetadata {
                run_id: run_id.to_string(),
                request_id: request.request_id.clone(),
                product_code: request.product_context.product_code.clone(),
                chapter_code: Some(chapter.code.clone()),
            },
        };

        let route = self
            .router
            .generate(
                provider_request,
                engine.provider.clone(),
                &engine.model,
                engine.allow_fallback,
                true,
                route_context,
            )
            .await?;

        let input_tokens = route.response.usage.as_ref().map(|u| u.input_tokens);
        let output_tokens = route.response.usage.as_ref().map(|u| u.output_tokens);

        let json = route.response.parsed_json.ok_or_else(|| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidJsonOutput,
                "provider returned no JSON for chapter",
                serde_json::json!({
                    "chapter": chapter.code,
                    "raw_text_preview": route.response.raw_text.chars().take(800).collect::<String>(),
                    "raw_text_len": route.response.raw_text.len(),
                }),
            )
        })?;

        self.validator.validate_chapter(&json)?;

        let mut chapter_reading: ChapterProviderResponse = serde_json::from_value(json).map_err(|e| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                format!("chapter deserialization failed: {e}"),
            )
        })?;

        if chapter_reading.code != chapter.code {
            if let Some(normalized) = normalize_chapter_code(&chapter_reading.code, &chapter.code) {
                tracing::warn!(
                    expected = %chapter.code,
                    received = %chapter_reading.code,
                    normalized = %normalized,
                    "chapter code normalized after provider drift"
                );
                chapter_reading.code = normalized;
            } else {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::SchemaValidationFailed,
                    "chapter code mismatch",
                    serde_json::json!({
                        "expected": chapter.code,
                        "received": chapter_reading.code
                    }),
                ));
            }
        }

        let mut reading_chapter = ReadingChapter {
            code: chapter_reading.code.clone(),
            title: chapter_reading.title,
            body: chapter_reading.body.clone(),
            astro_basis: chapter_reading.astro_basis,
            confidence: chapter_reading.confidence,
            safety_flags: vec![],
        };

        crate::astro_basis_role_normalizer::AstroBasisRoleNormalizer::normalize_chapter(
            &mut reading_chapter,
            chapter_pack,
        );

        if let Some(pack) = chapter_pack {
            ChapterEvidenceBasisEnricher::enrich_missing_pack_slots(&mut reading_chapter, pack);
            crate::astro_basis_role_normalizer::AstroBasisRoleNormalizer::normalize_chapter(
                &mut reading_chapter,
                chapter_pack,
            );
        }

        AstroLabelHumanizer::new(self.catalog).enrich_chapter_astro_basis(
            &mut reading_chapter.astro_basis,
            astro_facts,
            &request.product_context.user_language,
        );

        crate::evidence_fact_parse::normalize_chapter_astro_basis_fact_ids(
            &mut reading_chapter,
            astro_facts,
        );

        AstroBasisValidator::validate_chapter_with_pack(
            &reading_chapter,
            astro_facts,
            chapter_pack,
            product_policy,
        )?;

        if let Some(pack) = chapter_pack {
            ChapterEvidenceCoherence::validate_premium(
                &reading_chapter,
                pack,
                self.catalog.as_ref(),
                &request.product_context.user_language,
            )?;
        }

        SafetyGuard::validate_chapter_text(
            &reading_chapter.body,
            safety_policy,
            &request.astrologer_profile.forbidden_wording,
            self.catalog,
        )
        .map_err(|violations| {
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "chapter failed safety validation",
                serde_json::json!({ "violations": violations, "chapter": chapter.code }),
            )
        })?;

        TokenBudget::validate_chapter_lengths(
            &[(reading_chapter.code.clone(), reading_chapter.body.clone())],
            std::slice::from_ref(contract),
        )?;

        let meta = (
            route.used_provider.as_str().to_string(),
            route.response.model_used,
            route.fallback_used,
            input_tokens,
            output_tokens,
        );

        Ok((reading_chapter, bundle, meta))
    }
}

fn is_evidence_coherence_violation(err: &GenerationError) -> bool {
    matches!(err.detail().code, GenerationErrorCode::AstroBasisInvalid)
        && err.detail().message.contains("evidence coherence")
}

fn evidence_repair_from_error(err: &GenerationError) -> Option<ChapterRepairKind> {
    let details = err.detail().details.as_ref()?;
    let missing = details
        .get("missing_pack_fact_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let orphans = details
        .get("orphan_object_codes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Some(ChapterRepairKind::EvidenceCoherence {
        missing_pack_fact_ids: missing,
        orphan_object_codes: orphans,
    })
}

/// Corrige les derivees de code renvoyees par certains modeles (ex. `emotional_life_natal_premium_v1`).
fn normalize_chapter_code(received: &str, expected: &str) -> Option<String> {
    if received == expected {
        return Some(expected.to_string());
    }
    if received.starts_with(expected) {
        let suffix = &received[expected.len()..];
        if suffix.is_empty() || suffix.starts_with('_') {
            return Some(expected.to_string());
        }
    }
    None
}

fn resolve_safety_mode(provider: &astral_llm_domain::ProviderKind) -> SafetyMode {
    if matches!(provider, astral_llm_domain::ProviderKind::Mistral) {
        SafetyMode::PlatformAndNative
    } else {
        SafetyMode::PlatformRulesOnly
    }
}

#[cfg(test)]
mod chapter_code_tests {
    use super::normalize_chapter_code;

    #[test]
    fn normalizes_product_suffix_drift() {
        assert_eq!(
            normalize_chapter_code("emotional_life_natal_premium_v1", "emotional_life").as_deref(),
            Some("emotional_life")
        );
    }

    #[test]
    fn rejects_unrelated_code() {
        assert!(normalize_chapter_code("career", "emotional_life").is_none());
    }
}

pub fn new_run_id() -> String {
    Uuid::new_v4().to_string()
}

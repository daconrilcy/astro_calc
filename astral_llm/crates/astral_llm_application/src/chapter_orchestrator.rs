use std::time::{Duration, Instant};

use astral_llm_domain::{
    chapter_orchestration::{ChapterGenerationStatus, ReadingPlan, ReadingPlanChapter},
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
use crate::interpretive_evidence_builder::{is_premium_product, InterpretiveEvidenceBuilder};
use crate::chapter_quality_repair::{
    append_repair_instructions, is_min_words_violation, maybe_repair_repetition,
    retry_chapter_on_min_words, ChapterRepairKind,
};
use crate::domain_resolver::DomainResolver;
use crate::engine_defaults::ResolvedEngineParams;
use crate::execution_audit::ExecutionAudit;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::prompt_trace;
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reading_plan::ReadingPlanBuilder;
use crate::reading_quality_validator::ReadingQualityValidator;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
use crate::summary_synthesizer::SummarySynthesizer;
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
        run_id: &str,
        audit: &mut ExecutionAudit,
    ) -> Result<OrchestratedResult, GenerationError> {
        let domains = DomainResolver::resolve(request, self.catalog, self.limits, product_policy);
        audit.selected_domains = domains.clone();

        let plan = ReadingPlanBuilder::build(request, &domains);
        ReadingPlanBuilder::validate(&plan)?;

        let contracts = ReadingPlanBuilder::to_chapter_contracts(&plan);
        let quality_thresholds = ReadingQualityValidator::thresholds_for_request(request);
        let writing_locale =
            AstroLabelHumanizer::locale_key(&request.product_context.user_language);

        let pool = InterpretiveEvidenceBuilder::build(astro_facts, &self.catalog.evidence)?;
        let premium = is_premium_product(&request.product_context.product_code);
        let evidence_policy = &self.catalog.evidence.premium_policy;

        let chapter_packs = if premium {
            let packs = ChapterEvidencePlanner::plan_all(&pool, &plan, &self.catalog.evidence, evidence_policy)?;
            EvidenceDiversityValidator::validate_packs_planned(
                &request.product_context.product_code,
                &pool,
                &packs,
                &self.catalog.evidence,
                evidence_policy,
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

        for (chapter, contract) in plan.chapters.iter().zip(contracts.iter()) {
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
                    None,
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
                        |repair| {
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
                                repair,
                            )
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
                            Some(repair_kind),
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
                                |repair| {
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
                                        repair,
                                    )
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
                        |repair| {
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
                                repair,
                            )
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
                        |repair| {
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
                                repair,
                            )
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

        if premium {
            EvidenceDiversityValidator::validate_reading(
                &request.product_context.product_code,
                &pool,
                &generated,
                &chapter_packs,
            )?;
        }

        let summary_started = Instant::now();
        let synthesizer =
            SummarySynthesizer::new(self.router, self.validator, self.catalog);
        let summary_result = synthesizer
            .synthesize(request, &generated, engine, safety_policy, run_id)
            .await?;
        audit.record_chapter_step(
            "summary",
            &used_provider,
            &used_model,
            ChapterGenerationStatus::Generated,
            summary_result.input_tokens,
            summary_result.output_tokens,
            summary_started.elapsed().as_millis() as u64,
            None,
        );

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

        let evidence_metrics = if premium {
            Some(compute_evidence_metrics(&chapter_packs, &reading.chapters))
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
        repair: Option<ChapterRepairKind>,
    ) -> Result<
        (
            ReadingChapter,
            crate::prompt_compiler::PromptBundle,
            (String, String, bool, Option<u32>, Option<u32>),
        ),
        GenerationError,
    > {
        let _ = ProductPolicyValidator::validate(
            request,
            self.catalog,
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
            })
            .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;

        ChapterWritingGuidance::append_upstream_directives(
            &mut bundle,
            chapter,
            prior_chapters,
            chapter_pack,
            &request.product_context.user_language,
        );

        if let Some(ref repair_kind) = repair {
            append_repair_instructions(&mut bundle, chapter, repair_kind.clone());
        }

        let messages = self.compiler.to_provider_messages(&bundle);
        let attempt = match &repair {
            None => "primary",
            Some(crate::chapter_quality_repair::ChapterRepairKind::TooShort { .. }) => {
                "repair_too_short"
            }
            Some(crate::chapter_quality_repair::ChapterRepairKind::Repetition { .. }) => {
                "repair_repetition"
            }
            Some(crate::chapter_quality_repair::ChapterRepairKind::EvidenceCoherence { .. }) => {
                "repair_evidence"
            }
        };
        prompt_trace::log_prompt_bundle(
            run_id,
            Some(&chapter.code),
            &bundle,
            self.compiler,
            Some(attempt),
        );
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
            .map(|s| ProviderSchemaCompiler::compile(s, model_cap))
            .transpose()?;

        let provider_request = ProviderGenerationRequest {
            model: engine.model.clone(),
            messages,
            structured_schema: schema,
            reasoning_effort: engine.reasoning_effort,
            temperature: engine.temperature,
            max_output_tokens: Some(TokenBudget::chapter_max_tokens(
                contract,
                request.engine.max_output_tokens.or(request.response_contract.global_max_tokens),
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

        let chapter_reading: ChapterProviderResponse = serde_json::from_value(json).map_err(|e| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                format!("chapter deserialization failed: {e}"),
            )
        })?;

        if chapter_reading.code != chapter.code {
            return Err(GenerationError::with_details(
                GenerationErrorCode::SchemaValidationFailed,
                "chapter code mismatch",
                serde_json::json!({
                    "expected": chapter.code,
                    "received": chapter_reading.code
                }),
            ));
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

fn resolve_safety_mode(provider: &astral_llm_domain::ProviderKind) -> SafetyMode {
    if matches!(provider, astral_llm_domain::ProviderKind::Mistral) {
        SafetyMode::PlatformAndNative
    } else {
        SafetyMode::PlatformRulesOnly
    }
}

pub fn new_run_id() -> String {
    Uuid::new_v4().to_string()
}

use std::time::{Duration, Instant};

use astral_llm_domain::{
    chapter_orchestration::{ChapterGenerationStatus, ReadingPlan, ReadingPlanChapter},
    generation_response::{
        ChapterProviderResponse, LegalBlock, NatalReadingResponse, QualityMetadata,
        ReadingChapter, ReadingSummary,
    },
    output_contract::GenerationMode,
    GenerateReadingRequest, GenerationError, GenerationErrorCode, NormalizedAstroFacts,
    ProductGenerationPolicy, SafetyPolicy, SafetyMode,
};
use astral_llm_infra::SharedCanonicalCatalog;
use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest};
use uuid::Uuid;

use crate::astro_basis_validator::AstroBasisValidator;
use crate::chapter_quality_repair::{
    append_repair_instructions, maybe_repair_repetition, ChapterRepairKind,
};
use crate::domain_resolver::DomainResolver;
use crate::engine_defaults::ResolvedEngineParams;
use crate::execution_audit::ExecutionAudit;
use crate::product_policy_validator::ProductPolicyValidator;
use crate::prompt_compiler::{PromptCompilationInput, PromptCompiler};
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reading_plan::ReadingPlanBuilder;
use crate::reading_quality_validator::ReadingQualityValidator;
use crate::response_validator::ResponseValidator;
use crate::safety_guard::SafetyGuard;
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
        let min_astro_refs = product_policy.min_astro_basis_refs_per_chapter;
        let quality_thresholds = ReadingQualityValidator::thresholds_for_request(request);

        let mut generated = Vec::new();
        let mut last_bundle = None;
        let mut used_provider = engine.provider.as_str().to_string();
        let mut used_model = engine.model.clone();
        let mut fallback_used = false;

        for (chapter, contract) in plan.chapters.iter().zip(contracts.iter()) {
            let started = Instant::now();
            match self
                .generate_one_chapter(
                    request,
                    engine,
                    safety_policy,
                    astro_facts,
                    chapter,
                    contract,
                    run_id,
                    min_astro_refs,
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
                                chapter,
                                contract,
                                run_id,
                                min_astro_refs,
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
                Err(err) if is_length_violation(&err) => {
                    match self
                        .generate_one_chapter(
                            request,
                            engine,
                            safety_policy,
                            astro_facts,
                            chapter,
                            contract,
                            run_id,
                            min_astro_refs,
                            Some(ChapterRepairKind::Length),
                        )
                        .await
                    {
                        Ok((reading_chapter, bundle, route_meta)) => {
                            last_bundle = Some(bundle);
                            used_provider = route_meta.0;
                            used_model = route_meta.1;
                            fallback_used |= route_meta.2;
                            audit.record_chapter_step(
                                &chapter.code,
                                &used_provider,
                                &used_model,
                                ChapterGenerationStatus::Repaired,
                                route_meta.3,
                                route_meta.4,
                                started.elapsed().as_millis() as u64,
                                None,
                            );
                            generated.push(reading_chapter);
                        }
                        Err(repair_err) => {
                            audit.record_chapter_step(
                                &chapter.code,
                                engine.provider.as_str(),
                                &engine.model,
                                ChapterGenerationStatus::Failed,
                                None,
                                None,
                                started.elapsed().as_millis() as u64,
                                Some(repair_err.detail().code.as_str().to_string()),
                            );
                            return Err(repair_err);
                        }
                    }
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
        let reading = NatalReadingResponse {
            schema_version: request.response_contract.output_schema_version.clone(),
            language: request.product_context.user_language.clone(),
            reading_type: request.product_context.product_code.clone(),
            summary: ReadingSummary {
                title: format!("Lecture {} — synthese", request.product_context.product_code),
                short_text: "Synthese produite par generation chapitre par chapitre.".into(),
            },
            chapters: generated,
            legal: LegalBlock {
                disclaimer: default_disclaimer(
                    request.response_contract.include_legal_disclaimer,
                    &request.product_context.user_language,
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

        Ok(OrchestratedResult { reading, plan })
    }

    async fn generate_one_chapter(
        &self,
        request: &GenerateReadingRequest,
        engine: &ResolvedEngineParams,
        safety_policy: &SafetyPolicy,
        astro_facts: &NormalizedAstroFacts,
        chapter: &ReadingPlanChapter,
        contract: &astral_llm_domain::output_contract::ChapterContract,
        run_id: &str,
        min_astro_refs: u8,
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
                catalog: self.catalog,
            })
            .map_err(|e| GenerationError::new(GenerationErrorCode::InvalidInput, e))?;

        if let Some(repair) = repair {
            append_repair_instructions(&mut bundle, chapter, repair);
        }

        let messages = self.compiler.to_provider_messages(&bundle);
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
                request.response_contract.global_max_tokens,
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
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                "provider returned no JSON for chapter",
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

        let reading_chapter = ReadingChapter {
            code: chapter_reading.code.clone(),
            title: chapter_reading.title,
            body: chapter_reading.body.clone(),
            astro_basis: chapter_reading.astro_basis,
            confidence: chapter_reading.confidence,
            safety_flags: vec![],
        };

        AstroBasisValidator::validate_chapter_with_min_refs(
            &reading_chapter,
            astro_facts,
            min_astro_refs,
        )?;

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

fn is_length_violation(err: &GenerationError) -> bool {
    matches!(
        err.detail().code,
        GenerationErrorCode::SchemaValidationFailed
    ) && (err.detail().message.contains("min_words")
        || err.detail().message.contains("max_words"))
}

fn resolve_safety_mode(provider: &astral_llm_domain::ProviderKind) -> SafetyMode {
    if matches!(provider, astral_llm_domain::ProviderKind::Mistral) {
        SafetyMode::PlatformAndNative
    } else {
        SafetyMode::PlatformRulesOnly
    }
}

fn default_disclaimer(include: bool, _language: &str) -> String {
    if include {
        "Cette lecture est une interpretation symbolique et ne remplace aucun avis medical, \
         psychologique, juridique ou financier."
            .into()
    } else {
        String::new()
    }
}

pub fn new_run_id() -> String {
    Uuid::new_v4().to_string()
}

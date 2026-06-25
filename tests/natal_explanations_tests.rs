use std::sync::Arc;

use astral_llm_application::{
    build_provider_map,
    reading_catalog::ReadingCatalog,
    reading_persistence::{
        ExplanationCacheKeyRecord, ExplanationCacheRecord, PersistedGenerationRunRecord,
        PersistedPromptTraceRecord, PersistedTokenUsageRecord, ReadingPersistence,
        ReadingPersistenceError,
    },
    select_major_explanation_candidates, ExplanationPreparationRequest, GenerateReadingUseCase,
    ModelCapabilityRegistry, PromptCompiler, ProviderCircuitBreaker, ProviderRouter,
    ResponseValidator, SchemaRegistry,
};
use astral_llm_domain::{
    astro_fact::{AstroFactKind, AstroFactUsage, NormalizedAstroFact},
    chapter_orchestration::GenerationStepRecord,
    provider::ProviderKind,
    AstroCalculationPayload, EngineDefaults, FallbackPolicy, FallbackReason, PrivacyPolicy,
    ServiceLimits,
};
use astral_llm_infra::{
    bootstrap_astro_basis_roles, bootstrap_astro_object_labels, bootstrap_domains,
    bootstrap_interpretation_profiles, bootstrap_product_policies, bootstrap_zodiac_sign_labels,
    CanonicalCatalog,
};
use astral_llm_providers::FakeProvider;
use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

fn fact(
    id: &str,
    kind: AstroFactKind,
    kind_code: &str,
    label: &str,
    value: serde_json::Value,
) -> NormalizedAstroFact {
    NormalizedAstroFact {
        id: id.into(),
        kind,
        kind_code: kind_code.into(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: label.into(),
        value,
        interpretive_weight: None,
        domains: vec![],
    }
}

#[test]
fn natal_explanations_select_major_candidates_in_deterministic_order() {
    let facts = vec![
        fact(
            "aspect:mars_trine_uranus",
            AstroFactKind::Aspect,
            "aspect",
            "Mars en harmonie avec Uranus",
            serde_json::json!({ "aspect": "Mars trine Uranus" }),
        ),
        fact(
            "placement:moon:capricorn:house:6",
            AstroFactKind::PlanetPosition,
            "placement",
            "Lune en Capricorne maison 6",
            serde_json::json!({ "object": "moon", "sign": "capricorn", "house": 6 }),
        ),
        fact(
            "placement:sun:taurus:house:10",
            AstroFactKind::PlanetPosition,
            "placement",
            "Soleil en Taureau maison 10",
            serde_json::json!({ "object": "sun", "sign": "taurus", "house": 10 }),
        ),
        fact(
            "angle:ascendant:cancer",
            AstroFactKind::Angle,
            "angle",
            "Ascendant en Cancer",
            serde_json::json!({ "sign": "cancer" }),
        ),
    ];

    let catalog = test_catalog();
    let candidates = select_major_explanation_candidates(&facts, &catalog, "fr", 10);

    let ids = candidates
        .iter()
        .map(|candidate| candidate.fact_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "placement:sun:taurus:house:10",
            "placement:moon:capricorn:house:6",
            "angle:ascendant:cancer",
            "aspect:mars_trine_uranus"
        ]
    );
    assert_eq!(candidates[0].cache_key.key_json["object"], "sun");
    assert_eq!(candidates[0].cache_key.key_json["house"], 10);
}

#[test]
fn natal_explanations_cache_key_is_stable_for_same_combination() {
    let first = fact(
        "placement:sun:taurus:house:10",
        AstroFactKind::PlanetPosition,
        "placement",
        "Soleil en Taureau maison 10",
        serde_json::json!({ "object": "Sun", "sign": "Taurus", "house": 10 }),
    );
    let second = fact(
        "placement:sun:taurus:house:10",
        AstroFactKind::PlanetPosition,
        "placement",
        "Soleil en Taureau maison 10",
        serde_json::json!({ "object": "sun", "sign": "taurus", "house": 10 }),
    );

    let catalog = test_catalog();
    let a = select_major_explanation_candidates(&[first], &catalog, "fr", 1);
    let b = select_major_explanation_candidates(&[second], &catalog, "fr", 1);

    assert_eq!(a[0].cache_key.key_hash, b[0].cache_key.key_hash);
}

#[test]
fn natal_explanations_candidates_are_localized_by_language() {
    let catalog = test_catalog();
    let fact = fact(
        "placement:sun:taurus:house:10",
        AstroFactKind::PlanetPosition,
        "placement",
        "Sun in Taurus",
        serde_json::json!({ "object": "sun", "sign": "taurus", "house": 10 }),
    );

    let fr = select_major_explanation_candidates(&[fact.clone()], &catalog, "fr", 1);
    assert_eq!(fr[0].title, "Soleil en Taureau en maison 10");
    assert_eq!(fr[0].expression_primary.as_deref(), Some("Maison 10"));

    let en = select_major_explanation_candidates(&[fact], &catalog, "en", 1);
    assert_eq!(en[0].title, "Sun in Taurus in house 10");
    assert_eq!(en[0].expression_primary.as_deref(), Some("House 10"));
}

#[test]
fn natal_explanations_house_axis_cache_key_includes_justification_signature() {
    let catalog = test_catalog();
    let base = serde_json::json!({
        "axis_code": "resources_sharing",
        "houses": [2, 8],
        "theme_codes": ["resources", "shared_resources"],
        "primary_house": 2,
        "secondary_house": 8,
        "polarity_balance": "primary_house_dominant",
        "axis_score": 0.91,
        "reason_details": [
            { "reason_code": "object_in_house", "object_code": "sun", "house_number": 2, "theme_code": "resources" }
        ]
    });
    let changed = serde_json::json!({
        "axis_code": "resources_sharing",
        "houses": [2, 8],
        "theme_codes": ["resources", "shared_resources"],
        "primary_house": 2,
        "secondary_house": 8,
        "polarity_balance": "primary_house_dominant",
        "axis_score": 0.91,
        "reason_details": [
            { "reason_code": "object_in_house", "object_code": "moon", "house_number": 2, "theme_code": "resources" }
        ]
    });

    let first = select_major_explanation_candidates(
        &[fact(
            "house_axis:resources_sharing",
            AstroFactKind::HousePlacement,
            "house_axis",
            "Axe maison : resources_sharing",
            base,
        )],
        &catalog,
        "fr",
        1,
    );
    let second = select_major_explanation_candidates(
        &[fact(
            "house_axis:resources_sharing",
            AstroFactKind::HousePlacement,
            "house_axis",
            "Axe maison : resources_sharing",
            changed,
        )],
        &catalog,
        "fr",
        1,
    );

    assert_ne!(
        first[0].cache_key.key_hash, second[0].cache_key.key_hash,
        "house_axis cache must vary when concrete astrological justification changes"
    );
    assert_eq!(
        first[0].cache_key.key_json["justification_signature"]["houses"],
        serde_json::json!([2, 8])
    );
}

#[test]
fn natal_explanations_house_axis_signature_contains_prompt_context() {
    let catalog = test_catalog();
    let candidates = select_major_explanation_candidates(
        &[fact(
            "house_axis:resources_sharing",
            AstroFactKind::HousePlacement,
            "house_axis",
            "Axe maison : resources_sharing",
            serde_json::json!({
                "axis_code": "resources_sharing",
                "houses": [2, 8],
                "theme_codes": ["resources", "shared_resources"],
                "primary_house": 2,
                "secondary_house": 8,
                "polarity_balance": "primary_house_dominant",
                "axis_score": 1.0,
                "interpretive_hint": "Resources and Sharing is activated mainly through house 2.",
                "reason_details": [
                    { "reason_code": "dominant_house" },
                    { "reason_code": "object_in_house", "object_code": "sun", "house_number": 2, "theme_code": "resources" },
                    { "reason_code": "cross_axis_aspect", "signal_key": "aspect:jupiter:uranus:opposition" }
                ]
            }),
        )],
        &catalog,
        "fr",
        1,
    );

    let context = &candidates[0].cache_key.key_json["justification_signature"];

    assert_eq!(candidates[0].kind_code, "house_axis");
    assert_eq!(context["axis_code"], "resources_sharing");
    assert_eq!(context["houses"], serde_json::json!([2, 8]));
    assert_eq!(context["polarity_balance"], "primary_house_dominant");
    assert_eq!(
        context["interpretive_hint"],
        "Resources and Sharing is activated mainly through house 2."
    );
    assert!(context["supporting_factors"]
        .as_array()
        .is_some_and(|factors| factors.iter().any(|factor| factor["object_code"] == "sun")));
}

#[test]
fn natal_explanations_house_axis_signature_uses_projection_supporting_factors() {
    let catalog = test_catalog();
    let candidates = select_major_explanation_candidates(
        &[fact(
            "house_axis:resources_and_sharing",
            AstroFactKind::HousePlacement,
            "house_axis",
            "Axe maison : Resources and Sharing",
            serde_json::json!({
                "axis": "Resources and Sharing",
                "houses": [
                    { "number": 2, "theme": "Resources" },
                    { "number": 8, "theme": "Transformation" }
                ],
                "balance": "Mainly house 2",
                "summary": "Resources and Sharing is activated mainly through house 2.",
                "supporting_factors": [
                    "Dominant house emphasis",
                    "Sun in house",
                    "A major aspect connects both sides of this house axis"
                ]
            }),
        )],
        &catalog,
        "fr",
        1,
    );

    let signature = &candidates[0].cache_key.key_json["justification_signature"];

    assert_eq!(signature["axis_code"], "resources_and_sharing");
    assert_eq!(signature["houses"], serde_json::json!([2, 8]));
    assert_eq!(
        signature["supporting_factors"],
        serde_json::json!([
            "Dominant house emphasis",
            "Sun in house",
            "A major aspect connects both sides of this house axis"
        ])
    );
}

#[test]
fn natal_explanations_house_axes_rank_after_angles_before_house_emphasis() {
    let catalog = test_catalog();
    let facts = vec![
        fact(
            "house_emphasis:house:2",
            AstroFactKind::HousePlacement,
            "house_emphasis",
            "Emphase maison resources",
            serde_json::json!({ "house_number": 2, "theme_code": "resources" }),
        ),
        fact(
            "house_axis:resources_sharing",
            AstroFactKind::HousePlacement,
            "house_axis",
            "Axe maison : resources_sharing",
            serde_json::json!({
                "axis_code": "resources_sharing",
                "houses": [2, 8],
                "reason_details": [{ "reason_code": "dominant_house" }]
            }),
        ),
        fact(
            "angle:mc:aries",
            AstroFactKind::Angle,
            "angle",
            "MC en Bélier",
            serde_json::json!({ "sign": "aries" }),
        ),
    ];

    let candidates = select_major_explanation_candidates(&facts, &catalog, "fr", 10);
    let ids = candidates
        .iter()
        .map(|candidate| candidate.fact_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            "angle:mc:aries",
            "house_axis:resources_sharing",
            "house_emphasis:house:2"
        ]
    );
}

fn test_catalog() -> ReadingCatalog {
    ReadingCatalog::new(Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        astro_basis_roles: bootstrap_astro_basis_roles(),
        astro_object_labels: bootstrap_astro_object_labels(),
        zodiac_sign_labels: bootstrap_zodiac_sign_labels(),
        product_generation_policies: bootstrap_product_policies(),
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    }))
}

#[derive(Default)]
struct MemoryExplanationPersistence {
    records: Mutex<Vec<ExplanationCacheRecord>>,
}

#[async_trait]
impl ReadingPersistence for MemoryExplanationPersistence {
    async fn upsert_run(
        &self,
        _record: &PersistedGenerationRunRecord,
    ) -> Result<(), ReadingPersistenceError> {
        Ok(())
    }

    async fn insert_prompt_trace(
        &self,
        _record: &PersistedPromptTraceRecord,
    ) -> Result<(), ReadingPersistenceError> {
        Ok(())
    }

    async fn insert_steps(
        &self,
        _run_id: Uuid,
        _steps: &[GenerationStepRecord],
    ) -> Result<Vec<Uuid>, ReadingPersistenceError> {
        Ok(Vec::new())
    }

    async fn replace_run_token_usages(
        &self,
        _run_id: Uuid,
        _usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError> {
        Ok(())
    }

    async fn replace_step_token_usages(
        &self,
        _step_id: Uuid,
        _usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError> {
        Ok(())
    }

    async fn lookup_natal_explanations(
        &self,
        keys: &[ExplanationCacheKeyRecord],
    ) -> Result<Vec<ExplanationCacheRecord>, ReadingPersistenceError> {
        let records = self.records.lock().await;
        Ok(records
            .iter()
            .filter(|record| {
                keys.iter().any(|key| {
                    key.language_code == record.language_code && key.key_hash == record.key_hash
                })
            })
            .cloned()
            .collect())
    }

    async fn upsert_natal_explanations(
        &self,
        records: &[ExplanationCacheRecord],
    ) -> Result<(), ReadingPersistenceError> {
        let mut stored = self.records.lock().await;
        for record in records {
            if let Some(existing) = stored.iter_mut().find(|existing| {
                existing.language_code == record.language_code
                    && existing.key_hash == record.key_hash
            }) {
                *existing = record.clone();
            } else {
                stored.push(record.clone());
            }
        }
        Ok(())
    }
}

fn use_case_with_fake_fallback(
    persistence: Option<Arc<dyn ReadingPersistence>>,
) -> GenerateReadingUseCase {
    let router = ProviderRouter::new(
        build_provider_map(vec![Arc::new(FakeProvider)]),
        FallbackPolicy {
            enabled: true,
            chain: vec![ProviderKind::Fake],
            fallback_on: vec![
                FallbackReason::Timeout,
                FallbackReason::RateLimited,
                FallbackReason::ProviderUnavailable,
            ],
            require_same_structured_output_level: true,
            allow_cross_vendor_data_transfer: true,
            max_retries_per_provider: 1,
        },
        Arc::new(ModelCapabilityRegistry::bootstrap()),
        PrivacyPolicy {
            allow_cross_provider_fallback: true,
            ..PrivacyPolicy::default()
        },
        Arc::new(ProviderCircuitBreaker::new(5, 60)),
        None,
    );
    let prompts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(prompts),
        ResponseValidator::new(Arc::new(SchemaRegistry::new())),
        EngineDefaults {
            provider: ProviderKind::Fake,
            model: "fake-model".into(),
        },
        ServiceLimits::default(),
        test_catalog(),
        PrivacyPolicy {
            allow_cross_provider_fallback: true,
            ..PrivacyPolicy::default()
        },
        true,
        persistence,
    )
}

#[tokio::test]
async fn natal_explanations_prepare_generates_prompt_safe_block() {
    let use_case = use_case_with_fake_fallback(None);

    let response = use_case
        .prepare_natal_explanations(ExplanationPreparationRequest {
            run_id: Some("run-test".into()),
            user_language: "fr".into(),
            interpretation_profile_code: Some("natal_basic".into()),
            astro_result: AstroCalculationPayload {
                contract_version: "natal_structured_v14".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({
                    "planets": {
                        "sun": { "house": 10, "sign": "taurus" },
                        "moon": { "house": 6, "sign": "capricorn" }
                    }
                }),
            },
        })
        .await;

    assert_eq!(
        response.explanations.status, "complete",
        "errors: {:?}",
        response.explanations.errors
    );
    assert!(!response.explanations.items.is_empty());
    assert_eq!(
        response.neutral_explanations["_type"],
        "neutral_natal_explanations"
    );
}

#[tokio::test]
async fn natal_explanations_prepare_persists_house_axis_context_signature() {
    let persistence = Arc::new(MemoryExplanationPersistence::default());
    let use_case = use_case_with_fake_fallback(Some(persistence.clone()));

    let response = use_case
        .prepare_natal_explanations(ExplanationPreparationRequest {
            run_id: Some("run-axis".into()),
            user_language: "fr".into(),
            interpretation_profile_code: Some("natal_basic".into()),
            astro_result: AstroCalculationPayload {
                contract_version: "natal_structured_v14".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({
                    "house_axis_emphasis": [
                        {
                            "axis_code": "resources_sharing",
                            "houses": [2, 8],
                            "theme_codes": ["resources", "shared_resources"],
                            "house_scores": [
                                {
                                    "house_number": 2,
                                    "theme_code": "resources",
                                    "score": 1.0,
                                    "reason_details": [
                                        { "reason_code": "dominant_house" },
                                        { "reason_code": "object_in_house", "object_code": "sun", "house_number": 2, "theme_code": "resources" }
                                    ]
                                },
                                {
                                    "house_number": 8,
                                    "theme_code": "shared_resources",
                                    "score": 0.4,
                                    "reason_details": [
                                        { "reason_code": "cross_axis_aspect", "signal_key": "aspect:jupiter:uranus:opposition" }
                                    ]
                                }
                            ],
                            "primary_house": 2,
                            "secondary_house": 8,
                            "axis_score": 1.0,
                            "polarity_balance": "primary_house_dominant",
                            "interpretive_hint": "Resources and Sharing is activated mainly through house 2.",
                            "reason_details": [
                                { "reason_code": "dominant_house" },
                                { "reason_code": "object_in_house", "object_code": "sun", "house_number": 2, "theme_code": "resources" },
                                { "reason_code": "cross_axis_aspect", "signal_key": "aspect:jupiter:uranus:opposition" }
                            ]
                        }
                    ]
                }),
            },
        })
        .await;

    assert_eq!(
        response.explanations.status, "complete",
        "errors: {:?}",
        response.explanations.errors
    );
    assert_eq!(response.explanations.items[0].kind_code, "house_axis");
    assert!(
        response.neutral_explanations["items"][0]
            .get("astrological_context")
            .is_none(),
        "neutral_explanations public block must not expose internal prompt context"
    );

    let records = persistence.records.lock().await;
    let axis_record = records
        .iter()
        .find(|record| record.kind_code == "house_axis")
        .expect("persisted house axis explanation");
    assert_eq!(
        axis_record.key_json["justification_signature"]["houses"],
        serde_json::json!([2, 8])
    );
    assert!(
        axis_record.key_json["justification_signature"]["supporting_factors"]
            .as_array()
            .is_some_and(|factors| factors.iter().any(|factor| factor["object_code"] == "sun"))
    );
}

#[tokio::test]
async fn natal_explanations_cache_miss_is_persisted_then_reused() {
    let persistence = Arc::new(MemoryExplanationPersistence::default());
    let use_case = use_case_with_fake_fallback(Some(persistence.clone()));
    let request = ExplanationPreparationRequest {
        run_id: Some("run-test".into()),
        user_language: "fr".into(),
        interpretation_profile_code: Some("natal_basic".into()),
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v14".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "planets": {
                    "sun": { "house": 10, "sign": "taurus" }
                }
            }),
        },
    };

    let first = use_case.prepare_natal_explanations(request.clone()).await;
    assert_eq!(first.explanations.status, "complete");
    assert_eq!(first.explanations.items[0].source, "generated");
    assert_eq!(persistence.records.lock().await.len(), 1);

    let second = use_case.prepare_natal_explanations(request).await;
    assert_eq!(second.explanations.status, "complete");
    assert_eq!(second.explanations.items[0].source, "cache");
}

#[tokio::test]
async fn natal_explanations_cache_hit_requires_requested_language() {
    let persistence = Arc::new(MemoryExplanationPersistence::default());
    let use_case = use_case_with_fake_fallback(Some(persistence.clone()));
    let astro_result = AstroCalculationPayload {
        contract_version: "natal_structured_v14".into(),
        chart_type: "natal".into(),
        data: serde_json::json!({
            "planets": {
                "sun": { "house": 10, "sign": "taurus" }
            }
        }),
    };

    let fr = use_case
        .prepare_natal_explanations(ExplanationPreparationRequest {
            run_id: Some("run-fr".into()),
            user_language: "fr".into(),
            interpretation_profile_code: Some("natal_basic".into()),
            astro_result: astro_result.clone(),
        })
        .await;
    assert_eq!(fr.explanations.status, "complete");
    assert_eq!(fr.explanations.language_code, "fr");
    assert_eq!(fr.explanations.items[0].source, "generated");

    let en = use_case
        .prepare_natal_explanations(ExplanationPreparationRequest {
            run_id: Some("run-en".into()),
            user_language: "en".into(),
            interpretation_profile_code: Some("natal_basic".into()),
            astro_result,
        })
        .await;
    assert_eq!(en.explanations.status, "complete");
    assert_eq!(en.explanations.language_code, "en");
    assert_eq!(
        en.explanations.items[0].source, "generated",
        "fr cache entries must not satisfy en requests"
    );
    assert_eq!(persistence.records.lock().await.len(), 2);
}

#[tokio::test]
async fn natal_explanations_stale_cache_entry_is_regenerated() {
    let persistence = Arc::new(MemoryExplanationPersistence::default());
    let use_case = use_case_with_fake_fallback(Some(persistence.clone()));
    let request = ExplanationPreparationRequest {
        run_id: Some("run-stale".into()),
        user_language: "fr".into(),
        interpretation_profile_code: Some("natal_basic".into()),
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v14".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "planets": {
                    "sun": { "house": 10, "sign": "taurus" }
                }
            }),
        },
    };

    let first = use_case.prepare_natal_explanations(request.clone()).await;
    assert_eq!(first.explanations.status, "complete");
    assert_eq!(first.explanations.items[0].source, "generated");

    {
        let mut records = persistence.records.lock().await;
        let record = records.first_mut().expect("stored explanation");
        record.title = "Sun in Taurus in house 10".into();
        record.expression_primary = Some("House 10".into());
    }

    let second = use_case.prepare_natal_explanations(request).await;
    assert_eq!(second.explanations.status, "complete");
    assert_eq!(
        second.explanations.items[0].source, "generated",
        "stale cached language should be regenerated"
    );
    assert_eq!(
        second.explanations.items[0].title,
        "Soleil en Taureau en maison 10"
    );
}

#[tokio::test]
async fn natal_explanations_german_miss_is_persisted_then_reused() {
    let persistence = Arc::new(MemoryExplanationPersistence::default());
    let use_case = use_case_with_fake_fallback(Some(persistence.clone()));
    let request = ExplanationPreparationRequest {
        run_id: Some("run-de".into()),
        user_language: "de".into(),
        interpretation_profile_code: Some("natal_basic".into()),
        astro_result: AstroCalculationPayload {
            contract_version: "natal_structured_v14".into(),
            chart_type: "natal".into(),
            data: serde_json::json!({
                "planets": {
                    "sun": { "house": 10, "sign": "taurus" }
                }
            }),
        },
    };

    let first = use_case.prepare_natal_explanations(request.clone()).await;
    assert_eq!(first.explanations.status, "complete");
    assert_eq!(first.explanations.language_code, "de");
    assert_eq!(first.explanations.items[0].source, "generated");

    let second = use_case.prepare_natal_explanations(request).await;
    assert_eq!(second.explanations.status, "complete");
    assert_eq!(second.explanations.language_code, "de");
    assert_eq!(second.explanations.items[0].source, "cache");
    assert!(persistence
        .records
        .lock()
        .await
        .iter()
        .all(|record| record.language_code == "de"));
}

#[tokio::test]
async fn natal_explanations_unsupported_language_is_unavailable() {
    let use_case = use_case_with_fake_fallback(None);

    let response = use_case
        .prepare_natal_explanations(ExplanationPreparationRequest {
            run_id: Some("run-unsupported".into()),
            user_language: "spanish".into(),
            interpretation_profile_code: Some("natal_basic".into()),
            astro_result: AstroCalculationPayload {
                contract_version: "natal_structured_v14".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({
                    "planets": {
                        "sun": { "house": 10, "sign": "taurus" }
                    }
                }),
            },
        })
        .await;

    assert_eq!(response.explanations.status, "unavailable");
    assert_eq!(response.explanations.language_code, "spanish");
    assert!(response.explanations.items.is_empty());
    assert!(response.explanations.errors[0].contains("unsupported"));
}

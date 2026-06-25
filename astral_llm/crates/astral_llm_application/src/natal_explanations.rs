use std::collections::{HashMap, HashSet};
use std::time::Duration;

use astral_llm_domain::{
    astro_fact::{AstroFactKind, NormalizedAstroFact},
    model_usage_tier::ModelRouteContext,
    provider::SafetyMode,
    AstroCalculationPayload, GenerationError, GenerationErrorCode, PrivacyPolicy, ProviderKind,
};
use astral_llm_providers::{
    GenerationMetadata, PromptMessage, PromptRole, ProviderGenerationRequest,
};
use serde::{Deserialize, Serialize};

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::astro_payload_normalizer::AstroPayloadNormalizer;
use crate::provider_router::ProviderRouter;
use crate::provider_schema_compiler::ProviderSchemaCompiler;
use crate::reading_catalog::ReadingCatalog;
use crate::reading_persistence::{
    hash_json_value, ExplanationCacheKeyRecord, ExplanationCacheRecord, SharedReadingPersistence,
};
use crate::reasoning_generation::resolve_reasoning_effort;

const PROMPT_VERSION: &str = "natal_neutral_explanations_v3";
const DEFAULT_MODEL: &str = "gpt-5-mini";
const MAX_ITEMS_DEFAULT: usize = 12;

#[derive(Debug, Clone, Deserialize)]
pub struct ExplanationPreparationRequest {
    #[serde(default)]
    pub run_id: Option<String>,
    pub user_language: String,
    pub astro_result: AstroCalculationPayload,
    #[serde(default)]
    pub interpretation_profile_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationPreparationResponse {
    pub explanations: NatalExplanationsBlock,
    pub neutral_explanations: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatalExplanationsBlock {
    pub status: String,
    pub language_code: String,
    pub items: Vec<NatalExplanationItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_fact_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NatalExplanationItem {
    pub fact_id: String,
    pub kind_code: String,
    pub title: String,
    pub explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression_primary: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExplanationCacheKey {
    pub language_code: String,
    pub kind_code: String,
    pub key_hash: String,
    pub key_json: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ExplanationCandidate {
    pub fact_id: String,
    pub kind_code: String,
    pub title: String,
    pub title_is_localized: bool,
    pub expression_primary: Option<String>,
    pub cache_key: ExplanationCacheKey,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratedExplanationBatch {
    items: Vec<GeneratedExplanationItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratedExplanationItem {
    key_hash: String,
    title: String,
    explanation: String,
    #[serde(default)]
    expression_primary: Option<String>,
}

struct GeneratedExplanationResult {
    items: Vec<NatalExplanationItem>,
    provider: String,
    model: String,
}

pub struct NatalExplanationService<'a> {
    router: &'a ProviderRouter,
    catalog: &'a ReadingCatalog,
    privacy_policy: &'a PrivacyPolicy,
    persistence: Option<&'a SharedReadingPersistence>,
    default_timeout_ms: u64,
}

impl<'a> NatalExplanationService<'a> {
    pub fn new(
        router: &'a ProviderRouter,
        catalog: &'a ReadingCatalog,
        privacy_policy: &'a PrivacyPolicy,
        persistence: Option<&'a SharedReadingPersistence>,
        default_timeout_ms: u64,
    ) -> Self {
        Self {
            router,
            catalog,
            privacy_policy,
            persistence,
            default_timeout_ms,
        }
    }

    pub async fn prepare(
        &self,
        request: ExplanationPreparationRequest,
    ) -> ExplanationPreparationResponse {
        let mut errors = Vec::new();
        let language_code = match supported_language_code(&request.user_language) {
            Some(language_code) => language_code,
            None => {
                return unavailable(
                    request.user_language.trim().to_ascii_lowercase(),
                    format!(
                        "unsupported natal explanation language_code: {}",
                        request.user_language
                    ),
                );
            }
        };
        let normalized = match AstroPayloadNormalizer::normalize(
            &request.astro_result,
            self.privacy_policy,
            self.catalog.shared_catalog(),
            &language_code,
        ) {
            Ok(facts) => facts,
            Err(err) => {
                return unavailable(
                    language_code,
                    format!("astro facts normalization failed: {}", err.detail().message),
                );
            }
        };

        let limit = explanation_limit(request.interpretation_profile_code.as_deref());
        let candidates = select_major_explanation_candidates(
            &normalized.facts,
            self.catalog,
            &language_code,
            limit,
        );
        if candidates.is_empty() {
            return unavailable(
                language_code,
                "no eligible natal explanation candidates".to_string(),
            );
        }

        let mut items = Vec::new();
        let mut missing = candidates.clone();
        if let Some(persistence) = self.persistence {
            match persistence
                .lookup_natal_explanations(&cache_keys(&candidates))
                .await
            {
                Ok(records) => {
                    let by_hash = records
                        .into_iter()
                        .map(|record| (record.key_hash.clone(), record))
                        .collect::<HashMap<_, _>>();
                    missing.retain(|candidate| {
                        if let Some(record) = by_hash.get(&candidate.cache_key.key_hash) {
                            if record.prompt_version != PROMPT_VERSION {
                                return true;
                            }
                            if candidate.title_is_localized {
                                let title_matches = record.title.trim() == candidate.title.trim();
                                let expression_matches =
                                    record.expression_primary == candidate.expression_primary;
                                if !title_matches || !expression_matches {
                                    return true;
                                }
                            }
                            items.push(NatalExplanationItem {
                                fact_id: candidate.fact_id.clone(),
                                kind_code: candidate.kind_code.clone(),
                                title: record.title.clone(),
                                explanation: record.explanation.clone(),
                                expression_primary: record.expression_primary.clone(),
                                source: "cache".into(),
                            });
                            false
                        } else {
                            true
                        }
                    });
                }
                Err(err) => errors.push(err.to_string()),
            }
        }

        if !missing.is_empty() {
            match self.generate_missing(&request, &missing).await {
                Ok(generated) => {
                    if let Some(persistence) = self.persistence {
                        let records = generated
                            .items
                            .iter()
                            .filter_map(|item| {
                                let candidate = missing
                                    .iter()
                                    .find(|candidate| candidate.fact_id == item.fact_id)?;
                                Some(ExplanationCacheRecord {
                                    language_code: candidate.cache_key.language_code.clone(),
                                    kind_code: candidate.kind_code.clone(),
                                    key_hash: candidate.cache_key.key_hash.clone(),
                                    key_json: candidate.cache_key.key_json.clone(),
                                    title: item.title.clone(),
                                    explanation: item.explanation.clone(),
                                    expression_primary: item.expression_primary.clone(),
                                    provider: generated.provider.clone(),
                                    model: generated.model.clone(),
                                    prompt_version: PROMPT_VERSION.into(),
                                })
                            })
                            .collect::<Vec<_>>();
                        if let Err(err) = persistence.upsert_natal_explanations(&records).await {
                            errors.push(err.to_string());
                        }
                    }
                    items.extend(generated.items);
                }
                Err(err) => errors.push(err.detail().message.clone()),
            }
        }

        let produced_hashes = items
            .iter()
            .filter_map(|item| {
                candidates
                    .iter()
                    .find(|candidate| candidate.fact_id == item.fact_id)
                    .map(|candidate| candidate.cache_key.key_hash.clone())
            })
            .collect::<HashSet<_>>();
        let missing_fact_ids = candidates
            .iter()
            .filter(|candidate| !produced_hashes.contains(&candidate.cache_key.key_hash))
            .map(|candidate| candidate.fact_id.clone())
            .collect::<Vec<_>>();

        let status = if items.is_empty() {
            "unavailable"
        } else if missing_fact_ids.is_empty() && errors.is_empty() {
            "complete"
        } else {
            "partial"
        };

        response_from_parts(&language_code, status, items, missing_fact_ids, errors)
    }

    async fn generate_missing(
        &self,
        request: &ExplanationPreparationRequest,
        candidates: &[ExplanationCandidate],
    ) -> Result<GeneratedExplanationResult, GenerationError> {
        let (provider, model) = self.resolve_generation_engine();
        let model_cap = self
            .router
            .capability_registry()
            .require(&provider, &model)?;
        let payload = serde_json::json!({
            "language_code": candidates
                .first()
                .map(|candidate| candidate.cache_key.language_code.as_str())
                .unwrap_or("fr"),
            "items": candidates.iter().map(candidate_prompt_item).collect::<Vec<_>>(),
            "rules": {
                "style": "phrase courte, neutre, explicative",
                "forbid": ["prediction", "conseil", "diagnostic", "interpretation personnelle", "fatalisme"],
                "max_sentences": 1,
                "title_rule": "Si item.title_source=localized_input, conserver le meme titre. Si item.title_source=source_fallback, reecrire title dans language_code a partir de key, fact_id, kind_code et du titre fourni; ne pas conserver un titre dans une autre langue.",
                "house_axis_rule": "Pour kind_code=house_axis, utiliser 1 a 2 phrases et justifier l'axe par les maisons concernees plus au moins un facteur astrologique concret fourni dans astrological_context. Ne pas ecrire seulement que l'axe parle d'un theme sans preuve astrologique issue du contexte."
            }
        });
        let schema = ProviderSchemaCompiler::compile(&explanation_provider_schema(), model_cap)?;
        let provider_request = ProviderGenerationRequest {
            model: model.clone(),
            messages: vec![
                PromptMessage {
                    role: PromptRole::System,
                    content: "You produce factual, neutral astrological explanations as strict JSON. You do not interpret a person. Write title and explanation in the requested language_code.".into(),
                },
                PromptMessage {
                    role: PromptRole::User,
                    content: format!(
                        "Explain each combination in one neutral sentence. For house_axis items, use one or two neutral sentences and justify the axis with the houses plus at least one concrete astrological factor from astrological_context; avoid generic theme-only wording. Write title and explanation in language_code={}. Return only the requested JSON.\n{}",
                        payload["language_code"].as_str().unwrap_or("fr"),
                        serde_json::to_string_pretty(&payload).unwrap_or_default()
                    ),
                },
            ],
            structured_schema: Some(schema),
            reasoning_effort: resolve_reasoning_effort(
                model_cap,
                &astral_llm_domain::ProductGenerationPolicy::bootstrap_natal_prompter(),
                None,
                ModelRouteContext::Subtask,
            ),
            temperature: None,
            max_output_tokens: Some(1800),
            safety_mode: SafetyMode::PlatformRulesOnly,
            timeout: Duration::from_millis(self.default_timeout_ms.max(1_000)),
            metadata: GenerationMetadata {
                run_id: request.run_id.clone().unwrap_or_else(|| "natal-explanations".into()),
                request_id: request.run_id.clone(),
                product_code: "natal_explanations".into(),
                chapter_code: None,
                prompt_trace_step: Some("natal_explanations_prepare".into()),
                prompt_trace_attempt: Some("primary".into()),
                prompt_family: Some("natal_explanations".into()),
                prompt_version: Some(PROMPT_VERSION.into()),
            },
        };

        let route = self
            .router
            .generate(
                provider_request,
                provider,
                &model,
                true,
                true,
                ModelRouteContext::Subtask,
            )
            .await?;
        let json = route.response.parsed_json.ok_or_else(|| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidJsonOutput,
                "natal explanations provider returned no parsed JSON",
                serde_json::json!({ "raw_text": route.response.raw_text }),
            )
        })?;
        let batch: GeneratedExplanationBatch = serde_json::from_value(json).map_err(|err| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                format!("invalid natal explanations JSON: {err}"),
            )
        })?;
        let generated_by_hash = batch
            .items
            .into_iter()
            .map(|item| (item.key_hash.clone(), item))
            .collect::<HashMap<_, _>>();
        let provider = route.used_provider.as_str().to_string();
        let model = route.response.model_used.clone();
        let items = candidates
            .iter()
            .filter_map(|candidate| {
                let generated = generated_by_hash.get(&candidate.cache_key.key_hash)?;
                Some(NatalExplanationItem {
                    fact_id: candidate.fact_id.clone(),
                    kind_code: candidate.kind_code.clone(),
                    title: if candidate.title_is_localized {
                        candidate.title.clone()
                    } else {
                        non_empty(&generated.title).unwrap_or_else(|| candidate.title.clone())
                    },
                    explanation: non_empty(&generated.explanation)?,
                    expression_primary: candidate
                        .expression_primary
                        .clone()
                        .or_else(|| generated.expression_primary.clone()),
                    source: "generated".into(),
                })
            })
            .collect();
        Ok(GeneratedExplanationResult {
            items,
            provider,
            model,
        })
    }

    fn resolve_generation_engine(&self) -> (ProviderKind, String) {
        let provider = ProviderKind::OpenAi;
        let model = DEFAULT_MODEL.to_string();
        if self
            .router
            .capability_registry()
            .require(&provider, &model)
            .is_ok()
        {
            return (provider, model);
        }

        let fallback = ProviderKind::Fake;
        let fallback_model = self
            .router
            .capability_registry()
            .default_model_for_provider(&fallback)
            .unwrap_or_else(|| "fake-model".into());
        (fallback, fallback_model)
    }
}

pub async fn prepare_natal_explanations_response(
    service: NatalExplanationService<'_>,
    request: ExplanationPreparationRequest,
) -> ExplanationPreparationResponse {
    service.prepare(request).await
}

pub fn select_major_explanation_candidates(
    facts: &[NormalizedAstroFact],
    catalog: &ReadingCatalog,
    language: &str,
    limit: usize,
) -> Vec<ExplanationCandidate> {
    let mut ordered = facts
        .iter()
        .filter_map(|fact| candidate_from_fact(fact, catalog, language))
        .collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        candidate_rank(&a.fact_id, &a.kind_code).cmp(&candidate_rank(&b.fact_id, &b.kind_code))
    });
    let mut seen = HashSet::new();
    ordered
        .into_iter()
        .filter(|candidate| seen.insert(candidate.cache_key.key_hash.clone()))
        .take(limit)
        .collect()
}

fn candidate_from_fact(
    fact: &NormalizedAstroFact,
    catalog: &ReadingCatalog,
    language: &str,
) -> Option<ExplanationCandidate> {
    let kind_code = fact.effective_kind_code().to_string();
    let key_json = canonical_key_json(fact)?;
    let key_hash = hash_json_value(&key_json);
    let (title, title_is_localized) =
        localize_explanation_title_with_origin(fact, catalog, language);
    Some(ExplanationCandidate {
        fact_id: fact.id.clone(),
        kind_code,
        title,
        title_is_localized,
        expression_primary: expression_primary(fact, language),
        cache_key: ExplanationCacheKey {
            language_code: language.to_string(),
            kind_code: fact.effective_kind_code().to_string(),
            key_hash,
            key_json,
        },
    })
}

fn canonical_key_json(fact: &NormalizedAstroFact) -> Option<serde_json::Value> {
    match fact.kind {
        AstroFactKind::PlanetPosition => {
            let object = string_at(&fact.value, &["object", "object_code"])?;
            let sign = string_at(&fact.value, &["sign", "sign_code"])?;
            Some(serde_json::json!({
                "type": "placement",
                "object": normalize_token(&object),
                "sign": normalize_token(&sign),
                "house": fact.value.get("house").and_then(|v| v.as_u64())
            }))
        }
        AstroFactKind::Angle => {
            let sign = string_at(&fact.value, &["sign", "sign_code"])?;
            let value_angle = string_at(&fact.value, &["angle", "object"]);
            let angle = fact
                .id
                .strip_prefix("angle:")
                .and_then(|rest| rest.split(':').next())
                .or(value_angle.as_deref())
                .unwrap_or("angle")
                .to_string();
            Some(serde_json::json!({
                "type": "angle",
                "angle": normalize_token(&angle),
                "sign": normalize_token(&sign)
            }))
        }
        AstroFactKind::Aspect => Some(serde_json::json!({
            "type": "aspect",
            "label": normalize_token(&fact.label)
        })),
        AstroFactKind::HousePlacement => {
            if fact.effective_kind_code() == "house_emphasis" {
                Some(serde_json::json!({
                    "type": "house_emphasis",
                    "house": fact.value.get("house_number").or_else(|| fact.value.get("house")).cloned(),
                    "theme": fact.value.get("theme_code").or_else(|| fact.value.get("theme")).cloned()
                }))
            } else if fact.effective_kind_code() == "house_axis" {
                Some(serde_json::json!({
                    "type": "house_axis",
                    "axis": house_axis_code(fact),
                    "justification_signature": house_axis_justification_signature(fact)
                }))
            } else {
                None
            }
        }
        AstroFactKind::Ruler | AstroFactKind::Dignity => Some(serde_json::json!({
            "type": fact.effective_kind_code(),
            "label": normalize_token(&fact.label)
        })),
        AstroFactKind::DomainScore | AstroFactKind::Other => None,
    }
}

fn house_axis_astrological_context(fact: &NormalizedAstroFact) -> Option<serde_json::Value> {
    if fact.effective_kind_code() != "house_axis" {
        return None;
    }

    let houses = house_axis_houses(&fact.value);
    let theme_codes = house_axis_theme_codes(&fact.value);
    let factors = house_axis_supporting_factors(&fact.value);
    let mut context = serde_json::Map::new();

    insert_string(&mut context, "axis_code", house_axis_code(fact));
    insert_array(&mut context, "houses", houses);
    insert_array(&mut context, "theme_codes", theme_codes);
    insert_u64_value(
        &mut context,
        "primary_house",
        fact.value.get("primary_house"),
    );
    insert_u64_value(
        &mut context,
        "secondary_house",
        fact.value.get("secondary_house"),
    );
    insert_string_value(
        &mut context,
        "polarity_balance",
        fact.value
            .get("polarity_balance")
            .or_else(|| fact.value.get("balance")),
    );
    insert_number_value(&mut context, "axis_score", fact.value.get("axis_score"));
    insert_string_value(
        &mut context,
        "interpretive_hint",
        fact.value
            .get("interpretive_hint")
            .or_else(|| fact.value.get("summary")),
    );
    insert_array(&mut context, "supporting_factors", factors);

    if context.len() <= 1 {
        return None;
    }
    Some(serde_json::Value::Object(context))
}

fn house_axis_justification_signature(fact: &NormalizedAstroFact) -> serde_json::Value {
    house_axis_astrological_context(fact).unwrap_or_else(|| {
        serde_json::json!({
            "axis_code": house_axis_code(fact)
        })
    })
}

fn house_axis_code(fact: &NormalizedAstroFact) -> serde_json::Value {
    let code = fact
        .value
        .get("axis_code")
        .or_else(|| fact.value.get("axis"))
        .or_else(|| fact.value.get("theme"))
        .or_else(|| fact.value.get("label"))
        .and_then(|value| value.as_str())
        .or_else(|| fact.id.strip_prefix("house_axis:"))
        .unwrap_or("axis");
    serde_json::Value::String(normalize_token(code))
}

fn house_axis_houses(value: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(houses) = value.get("houses").and_then(|v| v.as_array()) {
        return houses
            .iter()
            .filter_map(|house| {
                house
                    .as_u64()
                    .or_else(|| house.get("number").and_then(|v| v.as_u64()))
            })
            .map(serde_json::Value::from)
            .collect();
    }
    value
        .get("house_scores")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|score| score.get("house_number").and_then(|v| v.as_u64()))
        .map(serde_json::Value::from)
        .collect()
}

fn house_axis_theme_codes(value: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(themes) = value.get("theme_codes").and_then(|v| v.as_array()) {
        return themes
            .iter()
            .filter_map(|theme| theme.as_str())
            .map(normalize_token)
            .map(serde_json::Value::from)
            .collect();
    }
    if let Some(houses) = value.get("houses").and_then(|v| v.as_array()) {
        let themes = houses
            .iter()
            .filter_map(|house| house.get("theme").and_then(|v| v.as_str()))
            .map(normalize_token)
            .map(serde_json::Value::from)
            .collect::<Vec<_>>();
        if !themes.is_empty() {
            return themes;
        }
    }
    value
        .get("house_scores")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|score| score.get("theme_code").and_then(|v| v.as_str()))
        .map(normalize_token)
        .map(serde_json::Value::from)
        .collect()
}

fn house_axis_supporting_factors(value: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut factors = Vec::new();
    push_string_array_values(&mut factors, value.get("supporting_factors"));
    if !factors.is_empty() {
        return factors;
    }

    push_reason_factor_values(&mut factors, value.get("reason_details"));
    if let Some(scores) = value.get("house_scores").and_then(|v| v.as_array()) {
        for score in scores {
            push_reason_factor_values(&mut factors, score.get("reason_details"));
        }
    }
    factors.truncate(8);
    factors
}

fn push_string_array_values(
    target: &mut Vec<serde_json::Value>,
    value: Option<&serde_json::Value>,
) {
    if let Some(items) = value.and_then(|v| v.as_array()) {
        for item in items.iter().filter_map(|item| item.as_str()) {
            push_unique_value(target, serde_json::Value::String(item.trim().to_string()));
        }
    }
}

fn push_reason_factor_values(
    target: &mut Vec<serde_json::Value>,
    value: Option<&serde_json::Value>,
) {
    let Some(reasons) = value.and_then(|v| v.as_array()) else {
        return;
    };
    for reason in reasons {
        let Some(reason_code) = reason.get("reason_code").and_then(|v| v.as_str()) else {
            continue;
        };
        let mut factor = serde_json::Map::new();
        factor.insert(
            "reason_code".into(),
            serde_json::Value::String(reason_code.into()),
        );
        for field in [
            "object_code",
            "house_number",
            "theme_code",
            "dignity_type",
            "signal_key",
            "context_key",
        ] {
            if let Some(value) = reason.get(field) {
                factor.insert(field.into(), value.clone());
            }
        }
        push_unique_value(target, serde_json::Value::Object(factor));
    }
}

fn insert_string(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    if value.as_str().is_some_and(|value| !value.trim().is_empty()) {
        target.insert(key.into(), value);
    }
}

fn insert_string_value(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&serde_json::Value>,
) {
    if let Some(text) = value
        .and_then(|v| v.as_str())
        .filter(|text| !text.trim().is_empty())
    {
        target.insert(key.into(), serde_json::Value::String(text.to_string()));
    }
}

fn insert_u64_value(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&serde_json::Value>,
) {
    if let Some(number) = value.and_then(|v| v.as_u64()) {
        target.insert(key.into(), serde_json::Value::from(number));
    }
}

fn insert_number_value(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&serde_json::Value>,
) {
    if let Some(number) = value
        .and_then(|v| v.as_f64())
        .and_then(serde_json::Number::from_f64)
    {
        target.insert(key.into(), serde_json::Value::Number(number));
    }
}

fn insert_array(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    values: Vec<serde_json::Value>,
) {
    if !values.is_empty() {
        target.insert(key.into(), serde_json::Value::Array(values));
    }
}

fn push_unique_value(target: &mut Vec<serde_json::Value>, value: serde_json::Value) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn expression_primary(fact: &NormalizedAstroFact, language: &str) -> Option<String> {
    let locale = AstroLabelHumanizer::locale_key(language);
    fact.value
        .get("house")
        .and_then(|v| v.as_u64())
        .map(|house| localized_house_label(locale, house))
        .or_else(|| {
            fact.value
                .get("theme")
                .and_then(|v| v.as_str())
                .map(localize_primary_token)
        })
        .or_else(|| {
            fact.value
                .get("theme_code")
                .and_then(|v| v.as_str())
                .map(localize_primary_token)
        })
}

fn candidate_rank(fact_id: &str, kind_code: &str) -> u8 {
    let id = fact_id.to_ascii_lowercase();
    if id.starts_with("placement:sun:") || id.starts_with("placement:soleil:") {
        0
    } else if id.starts_with("placement:moon:") || id.starts_with("placement:lune:") {
        1
    } else if id.starts_with("angle:ascendant:") || id.starts_with("placement:ascendant:") {
        2
    } else if kind_code == "angle" {
        3
    } else if kind_code == "house_axis" {
        4
    } else if kind_code == "house_emphasis" {
        5
    } else if kind_code == "placement" {
        6
    } else if kind_code == "aspect" {
        7
    } else {
        8
    }
}

fn cache_keys(candidates: &[ExplanationCandidate]) -> Vec<ExplanationCacheKeyRecord> {
    candidates
        .iter()
        .map(|candidate| ExplanationCacheKeyRecord {
            language_code: candidate.cache_key.language_code.clone(),
            key_hash: candidate.cache_key.key_hash.clone(),
        })
        .collect()
}

fn candidate_prompt_item(candidate: &ExplanationCandidate) -> serde_json::Value {
    let mut item = serde_json::json!({
        "fact_id": candidate.fact_id,
        "kind_code": candidate.kind_code,
        "key_hash": candidate.cache_key.key_hash,
        "key": candidate.cache_key.key_json,
        "title": candidate.title,
        "title_source": if candidate.title_is_localized { "localized_input" } else { "source_fallback" },
        "expression_primary": candidate.expression_primary
    });
    if let Some(context) = candidate_prompt_astrological_context(candidate) {
        item["astrological_context"] = context;
    }
    item
}

fn candidate_prompt_astrological_context(
    candidate: &ExplanationCandidate,
) -> Option<serde_json::Value> {
    if candidate.kind_code != "house_axis" {
        return None;
    }
    candidate
        .cache_key
        .key_json
        .get("justification_signature")
        .cloned()
}

fn explanation_provider_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["items"],
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["key_hash", "title", "explanation"],
                    "properties": {
                        "key_hash": { "type": "string" },
                        "title": { "type": "string" },
                        "explanation": { "type": "string" },
                        "expression_primary": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

fn response_from_parts(
    language_code: &str,
    status: &str,
    items: Vec<NatalExplanationItem>,
    missing_fact_ids: Vec<String>,
    errors: Vec<String>,
) -> ExplanationPreparationResponse {
    let neutral_items = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "fact_id": item.fact_id,
                "kind_code": item.kind_code,
                "title": item.title,
                "explanation": item.explanation,
                "expression_primary": item.expression_primary
            })
        })
        .collect::<Vec<_>>();
    ExplanationPreparationResponse {
        explanations: NatalExplanationsBlock {
            status: status.into(),
            language_code: language_code.into(),
            items,
            missing_fact_ids,
            errors,
        },
        neutral_explanations: serde_json::json!({
            "_type": "neutral_natal_explanations",
            "_instruction": "DATA ONLY - neutral glossary generated before interpretation. Use as factual guidance; do not copy mechanically and do not treat it as the final interpretation.",
            "prompt_version": PROMPT_VERSION,
            "language_code": language_code,
            "items": neutral_items
        }),
    }
}

fn unavailable(language_code: String, error: String) -> ExplanationPreparationResponse {
    response_from_parts(&language_code, "unavailable", vec![], vec![], vec![error])
}

fn supported_language_code(language: &str) -> Option<String> {
    let normalized = language.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "fr" | "en" | "es" | "de" => Some(normalized),
        _ => None,
    }
}

fn explanation_limit(profile_code: Option<&str>) -> usize {
    match profile_code {
        Some("natal_light") | Some("natal_simplified") => 6,
        Some("natal_basic") => 10,
        _ => MAX_ITEMS_DEFAULT,
    }
}

fn string_at(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()).map(str::to_string))
}

fn normalize_token(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(' ', "_")
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn localized_house_label(locale: &str, house: u64) -> String {
    match locale {
        "fr" => format!("Maison {house}"),
        "es" => format!("Casa {house}"),
        "de" => format!("Haus {house}"),
        _ => format!("House {house}"),
    }
}

fn localize_primary_token(value: &str) -> String {
    value
        .trim()
        .replace('_', " ")
        .split_whitespace()
        .map(|token| {
            let mut chars = token.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn localize_explanation_title(
    fact: &NormalizedAstroFact,
    catalog: &ReadingCatalog,
    language: &str,
) -> String {
    localize_explanation_title_with_origin(fact, catalog, language).0
}

fn localize_explanation_title_with_origin(
    fact: &NormalizedAstroFact,
    catalog: &ReadingCatalog,
    language: &str,
) -> (String, bool) {
    let humanizer = AstroLabelHumanizer::new(catalog.shared_catalog());
    if let Some(axis_code) = fact.id.strip_prefix("house_axis:") {
        let locale = AstroLabelHumanizer::locale_key(language);
        if let Some(label) = catalog
            .shared_catalog()
            .house_axis_labels
            .get(&(locale.to_string(), axis_code.to_string()))
        {
            return (label.display_label.clone(), true);
        }
        return (
            humanizer
                .label_for_fact_id(&fact.id, language, None)
                .unwrap_or_else(|| fact.label.clone()),
            false,
        );
    }
    if let Some(label) = humanizer.label_for_fact_id(&fact.id, language, None) {
        return (label, true);
    }

    let label = humanizer.humanize_fact_label(fact, language, None);
    let localized_from_structured_data = label.trim() != fact.label.trim();
    (label, localized_from_structured_data)
}

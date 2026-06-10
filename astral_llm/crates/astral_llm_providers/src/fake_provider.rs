use async_trait::async_trait;

use astral_llm_domain::{
    generation_response::{
        ChapterProviderResponse, ConfidenceLevel, LegalBlock, NatalReadingResponse,
        QualityMetadata, ReadingChapter, ReadingSummary, SummaryProviderResponse,
    },
    output_contract::GenerationMode,
    provider::{ProviderCapabilities, ProviderKind, StructuredOutputMode},
    ProviderKind as DomainProviderKind,
};

use crate::provider_trait::LlmProvider;
use crate::types::{ProviderGenerationRequest, ProviderGenerationResponse, TokenUsage};
use crate::LlmProviderError;

use astral_llm_domain::chapter_orchestration::READING_SUMMARY_STEP_CODE;

pub struct FakeProvider;

const SUMMARY_SHORT_TEXT: &str =
    "Votre theme met en avant une dynamique d'affirmation personnelle, \
    une grande richesse emotionnelle et un chemin relationnel structurant. Cette configuration \
    symbolique invite a accueillir les transitions interieures comme des espaces de croissance \
    authentique, sans figer votre parcours dans une trajectoire unique.";

#[async_trait]
impl LlmProvider for FakeProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Fake
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            structured_output: StructuredOutputMode::JsonSchemaStrict,
            supports_reasoning_effort: true,
            supports_streaming: false,
            supports_native_safety_prompt: false,
            supports_prompt_cache: false,
            max_input_tokens: Some(128_000),
            max_output_tokens: Some(8_000),
        }
    }

    async fn generate(
        &self,
        request: ProviderGenerationRequest,
    ) -> Result<ProviderGenerationResponse, LlmProviderError> {
        crate::http::with_timeout(request.timeout, async {
            let json = if is_full_reading_request(&request) {
                serde_json::to_value(build_full_reading(&request))
            } else if request.metadata.chapter_code.as_deref() == Some(READING_SUMMARY_STEP_CODE) {
                serde_json::to_value(build_summary_response())
            } else if request.metadata.chapter_code.is_some() {
                serde_json::to_value(build_chapter_response(&request))
            } else {
                serde_json::to_value(build_full_reading(&request))
            }
            .map_err(|e| LlmProviderError::InvalidResponse(e.to_string()))?;

            Ok(ProviderGenerationResponse {
                raw_text: json.to_string(),
                parsed_json: Some(json),
                usage: Some(TokenUsage {
                    input_tokens: 120,
                    output_tokens: 450,
                }),
                provider_metadata: serde_json::json!({ "fake": true }),
                model_used: request.model,
                provider_kind: DomainProviderKind::Fake,
            })
        })
        .await
    }
}

fn is_full_reading_request(request: &ProviderGenerationRequest) -> bool {
    let Some(schema) = request.structured_schema.as_ref() else {
        return false;
    };
    let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) else {
        return false;
    };
    properties.contains_key("chapters") && properties.contains_key("summary")
}

fn build_summary_response() -> SummaryProviderResponse {
    SummaryProviderResponse {
        title: "Une dynamique d'affirmation et de profondeur".into(),
        short_text: SUMMARY_SHORT_TEXT.into(),
    }
}

fn build_chapter_response(request: &ProviderGenerationRequest) -> ChapterProviderResponse {
    let code = request
        .metadata
        .chapter_code
        .clone()
        .unwrap_or_else(|| "identity".into());
    let available = extract_fact_ids_from_messages(&request.messages);
    let interpretive = available
        .iter()
        .find(|id| !id.starts_with("domain_score:"))
        .cloned()
        .unwrap_or_else(|| {
            available
                .first()
                .cloned()
                .unwrap_or_else(|| "placement:sun:capricorn:house:2".into())
        });
    let supporting = available
        .iter()
        .find(|id| *id != interpretive.as_str() && !id.starts_with("domain_score:"))
        .cloned();

    let mut basis = vec![astral_llm_domain::AstroBasisItem {
        fact_id: Some(interpretive),
        label: None,
        factor: "placement".into(),
        interpretive_role: "core".into(),
    }];
    if let Some(sid) = supporting {
        basis.push(astral_llm_domain::AstroBasisItem {
            fact_id: Some(sid),
            label: None,
            factor: "supporting".into(),
            interpretive_role: "supporting".into(),
        });
    }

    ChapterProviderResponse {
        code: code.clone(),
        title: code.replace('_', " "),
        body: chapter_body_for_code(&code),
        astro_basis: basis,
        confidence: ConfidenceLevel::Medium,
    }
}

const FAKE_CHAPTER_SUFFIX: &str = "Cette lecture symbolique reste une piste de reflexion, \
    jamais une prescription rigide ni une promesse certaine.";

/// Seuil conservateur au-dessus des min_words des profils de test (ex. natal_basic = 70).
const FAKE_MIN_CHAPTER_WORDS: u32 = 80;

const FAKE_CHAPTER_PAD: &[&str] = &[
    "Observer ces dynamiques avec curiosite permet de transformer l incertitude en matiere creative.",
    "Chaque cycle interieur merite d etre accueilli comme une information utile, jamais comme un verdict.",
    "La symbolique astrologique eclaire des possibles, non des obligations figees dans le temps.",
    "Accueillir vos contradictions interieures ouvre un espace de lucidite bienveillante et concrete.",
];

fn chapter_word_count(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

fn pad_to_min_words(mut body: String, min: u32) -> String {
    let mut i = 0usize;
    while chapter_word_count(&body) < min {
        body.push(' ');
        body.push_str(FAKE_CHAPTER_PAD[i % FAKE_CHAPTER_PAD.len()]);
        i += 1;
    }
    body
}

fn chapter_body_for_code(code: &str) -> String {
    let core = match code {
        "identity" => "Votre identite se construit par strates successives, entre affirmations \
            prudentes et questionnements feconds. Les signaux du theme invitent a reconnaitre une \
            sensibilite aux transitions, plutot qu'une identite figee. Vous accueillez l'inconnu \
            comme matiere de croissance, sans vous reduire a un seul role social. La symbolique \
            astrologique eclaire des ressorts interieurs, des habitudes relationnelles et des \
            choix de vie qui resonnent avec votre tempo personnel. Cette lecture reste une \
            invitation a explorer, jamais une etiquette definitive ni une sentence absolue imposee."
            .into(),
        "relationships" => "Vos liens humains expriment une recherche d'authenticite et de \
            reciprocite, parfois teintee de reserve. Le theme met en lumiere des besoins \
            affectifs nuancees, une ecoute attentive et une capacite d'ajustement lorsque la \
            confiance s'installe. Les dynamiques de couple ou d'amitie gagnent en clarte lorsque \
            vous acceptez des rythmes differents, sans imposer une forme unique de proximite. \
            Chaque relation devient alors un miroir evolutif, jamais un contrat fige."
            .into(),
        "emotional_life" => "Votre vie emotionnelle apparait comme un espace de nuances, entre \
            intensite contenue et moments d ouverture. Le theme suggere une intelligence \
            affective en developpement, capable d accueillir l ambivalence sans la subir. Les \
            cycles interieurs trouvent un sens lorsque vous les reliez a des experiences \
            symboliques, plutot qu a des jugements rigides sur vous-meme. Accueillir vos \
            variations d humeur devient alors un exercice de lucidite, jamais une condamnation \
            de votre sensibilite naturelle."
            .into(),
        "career" => "Votre trajectoire professionnelle se dessine avec pragmatisme et intuition, \
            en equilibre entre structure et creativite. Le theme souligne une ambition mesuree, \
            attentive aux contextes et aux alliances utiles. Vous progressez lorsque vos \
            motivations profondes sont reconnues, sans sacrifier votre integrite ni votre rythme \
            personnel de maturation. Les transitions de carriere gagnent en coherence lorsque \
            vous accueillez les phases d apprentissage comme des investissements durables."
            .into(),
        "growth_path" => "Votre chemin de croissance personnelle se revele par etapes, entre \
            remises en question salutaires et consolidations progressives. Le theme suggere une \
            capacite a transformer les epreuves en recit de sens, plutot qu en accumulation de \
            frustrations muettes. Vous avancez lorsque vous accueillez l incertitude comme \
            compagne de route, jamais comme ennemie a eliminer. Chaque saison interieure devient \
            alors une occasion d affiner votre boussole existentielle avec plus de finesse."
            .into(),
        "talents" => "Vos talents s expriment souvent de maniere discrete, a travers des gestes \
            concrets, une ecoute fine ou une capacite d adaptation remarquable. Le theme met en \
            lumiere des ressources creatrices parfois sous-estimees, qui emergent lorsque la \
            confiance remplace l autocritique excessive. Cultiver ces dons demande de la patience, \
            un cadre protecteur et des occasions d essai sans jugement immediat. La reconnaissance \
            progressive de vos forces ouvre des voies inattendues, toujours modulables."
            .into(),
        _ => format!(
            "Le domaine {code} du theme offre une lecture symbolique accessible, orientee vers \
             la comprehension des dynamiques observees sans posture deterministe. Cette perspective \
             invite a relier les signaux du ciel aux choix concrets du quotidien, avec nuance \
             et recul."
        ),
    };
    pad_to_min_words(
        format!("{core} {FAKE_CHAPTER_SUFFIX}"),
        FAKE_MIN_CHAPTER_WORDS,
    )
}

fn extract_fact_ids_from_messages(messages: &[crate::types::PromptMessage]) -> Vec<String> {
    let mut ids = Vec::new();
    for message in messages {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&message.content) {
            collect_fact_ids_from_json(&value, &mut ids);
            continue;
        }
        collect_fact_ids_from_text(&message.content, &mut ids);
    }
    ids.sort();
    ids.dedup();
    ids
}

fn collect_fact_ids_from_json(value: &serde_json::Value, ids: &mut Vec<String>) {
    if value.get("_type").and_then(|v| v.as_str()) == Some("chapter_evidence_pack") {
        for key in ["core", "supporting", "nuance"] {
            if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(id) = item.get("fact_id").and_then(|v| v.as_str()) {
                        ids.push(id.to_string());
                    }
                }
            }
        }
        return;
    }
    if let Some(arr) = value.get("facts").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                ids.push(id.to_string());
            }
        }
    }
}

fn collect_fact_ids_from_text(content: &str, ids: &mut Vec<String>) {
    if let Some(json) = extract_astro_data_json(content) {
        collect_fact_ids_from_json(&json, ids);
        return;
    }
    if content.contains("chapter_evidence_pack") {
        for part in content.split("\"fact_id\":") {
            let trimmed = part.trim_start_matches([' ', ':']).trim_start_matches('"');
            if let Some(end) = trimmed.find('"') {
                let id = trimmed[..end].trim();
                if !id.is_empty() {
                    ids.push(id.to_string());
                }
            }
        }
        return;
    }
    if let Some(start) = content.find("\"facts\"") {
        let slice = &content[start..];
        for part in slice.split("\"id\":") {
            let trimmed = part.trim_start_matches([' ', ':']).trim_start_matches('"');
            if let Some(end) = trimmed.find('"') {
                ids.push(trimmed[..end].to_string());
            }
        }
    }
}

fn extract_astro_data_json(content: &str) -> Option<serde_json::Value> {
    let start = content.find("--- BEGIN ASTRO DATA")?;
    let end = content.find("--- END ASTRO DATA")?;
    let slice = content.get(start..end)?;
    let json_start = slice.find('{')?;
    serde_json::from_str(slice[json_start..].trim()).ok()
}

fn build_full_reading(request: &ProviderGenerationRequest) -> NatalReadingResponse {
    let chapter_code = request
        .metadata
        .chapter_code
        .clone()
        .unwrap_or_else(|| "identity".to_string());
    let core = if chapter_code == "ambiguous_core_identity" {
        "Votre Soleil se situe dans une zone de changement possible entre deux signes. \
         Sans heure ou fuseau plus precis, on ne peut pas poser clairement le coeur solaire \
         du profil. Les autres placements stables peuvent neanmoins donner des indications \
         secondaires, avec prudence."
    } else {
        "Interpretation symbolique : votre theme suggere une personnalite reflechie, \
         orientee vers la comprehension des experiences."
    };
    let body = pad_to_min_words(core.to_string(), FAKE_MIN_CHAPTER_WORDS);
    let title = if chapter_code == "ambiguous_core_identity" {
        "Identite - Soleil ambigu"
    } else {
        "Identite"
    };
    let astro_contract_version = extract_simplified_contract_version(&request.messages)
        .unwrap_or_else(|| "natal_simplified_structured_v1".to_string());
    NatalReadingResponse {
        schema_version: "natal_reading_v1".to_string(),
        language: "fr".to_string(),
        reading_type: request.metadata.product_code.clone(),
        summary: ReadingSummary {
            title: "Lecture symbolique de demonstration".to_string(),
            short_text: "Interpretation symbolique de demonstration via FakeProvider.".to_string(),
        },
        chapters: vec![ReadingChapter {
            code: chapter_code,
            title: title.to_string(),
            body,
            astro_basis: simplified_fake_astro_basis(&request.messages),
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        }],
        legal: LegalBlock {
            disclaimer: astral_llm_domain::default_legal_disclaimer("fr", true),
        },
        quality: QualityMetadata {
            used_provider: "fake".to_string(),
            used_model: request.model.clone(),
            generation_mode: GenerationMode::SinglePass,
            prompt_family: request.metadata.product_code.clone(),
            prompt_version: "v1".to_string(),
            astro_contract_version,
            fallback_used: false,
        },
    }
}

fn simplified_fake_astro_basis(
    messages: &[crate::types::PromptMessage],
) -> Vec<astral_llm_domain::AstroBasisItem> {
    let blocked = extract_blocked_object_codes(messages);
    extract_allowed_astro_basis_ids(messages)
        .into_iter()
        .filter(|id| {
            id.strip_prefix("placement:")
                .is_none_or(|object| !blocked.contains(&object.to_string()))
        })
        .take(2)
        .map(|id| astral_llm_domain::AstroBasisItem {
            fact_id: Some(id),
            label: None,
            factor: "placement".into(),
            interpretive_role: "core".into(),
        })
        .collect()
}

fn extract_simplified_contract_version(messages: &[crate::types::PromptMessage]) -> Option<String> {
    for msg in messages {
        let Some(data) = extract_astro_data_json(&msg.content) else {
            continue;
        };
        if let Some(version) = data
            .get("payload_contract")
            .and_then(|v| v.as_str())
            .map(str::to_string)
        {
            return Some(version);
        }
    }
    None
}

fn extract_blocked_object_codes(messages: &[crate::types::PromptMessage]) -> Vec<String> {
    for msg in messages {
        let Some(data) = extract_astro_data_json(&msg.content) else {
            continue;
        };
        if let Some(items) = data
            .pointer("/llm_controls/blocked_interpretation_fact_codes")
            .and_then(|v| v.as_array())
        {
            return items
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|code| code.strip_suffix(".sign"))
                .map(str::to_string)
                .collect();
        }
    }
    Vec::new()
}

fn extract_allowed_astro_basis_ids(messages: &[crate::types::PromptMessage]) -> Vec<String> {
    for msg in messages {
        let Some(data) = extract_astro_data_json(&msg.content) else {
            continue;
        };
        if let Some(items) = data
            .pointer("/llm_controls/allowed_astro_basis_fact_ids")
            .and_then(|v| v.as_array())
        {
            return items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GenerationMetadata, PromptMessage, PromptRole};
    use astral_llm_domain::provider::ReasoningEffort;
    use std::time::Duration;

    #[test]
    fn extracts_fact_ids_from_evidence_pack_message() {
        let pack = serde_json::json!({
            "_type": "chapter_evidence_pack",
            "core": [{ "fact_id": "signal:object_position:venus" }],
            "supporting": [],
            "nuance": []
        });
        let content = format!(
            "task\n\n--- BEGIN ASTRO DATA (read-only) ---\n{}\n--- END ASTRO DATA ---\n",
            serde_json::to_string_pretty(&pack).unwrap()
        );
        let ids = extract_fact_ids_from_messages(&[PromptMessage {
            role: PromptRole::User,
            content,
        }]);
        assert!(ids.contains(&"signal:object_position:venus".to_string()));
    }

    #[test]
    fn emotional_life_meets_min_words_for_basic_profile() {
        let body = chapter_body_for_code("emotional_life");
        assert!(
            chapter_word_count(&body) >= FAKE_MIN_CHAPTER_WORDS,
            "expected at least {FAKE_MIN_CHAPTER_WORDS} words, got {}",
            chapter_word_count(&body)
        );
    }

    #[tokio::test]
    async fn fake_provider_returns_chapter_json() {
        let provider = FakeProvider;
        let request = ProviderGenerationRequest {
            model: "fake-model".to_string(),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                content: "test".to_string(),
            }],
            structured_schema: None,
            reasoning_effort: Some(ReasoningEffort::Low),
            temperature: Some(0.4),
            max_output_tokens: Some(1000),
            safety_mode: astral_llm_domain::SafetyMode::PlatformRulesOnly,
            timeout: Duration::from_secs(5),
            metadata: GenerationMetadata {
                run_id: "run-1".to_string(),
                request_id: None,
                product_code: "natal_prompter".to_string(),
                chapter_code: Some("career".into()),
            },
        };

        let response = provider.generate(request).await.expect("fake ok");
        let code = response
            .parsed_json
            .as_ref()
            .and_then(|v| v.get("code"))
            .and_then(|v| v.as_str());
        assert_eq!(code, Some("career"));
    }

    #[tokio::test]
    async fn fake_provider_returns_full_reading_when_full_schema_has_chapter_code() {
        let provider = FakeProvider;
        let request = ProviderGenerationRequest {
            model: "fake-model".to_string(),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                content: "test".to_string(),
            }],
            structured_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "schema_version": { "type": "string" },
                    "language": { "type": "string" },
                    "reading_type": { "type": "string" },
                    "summary": { "type": "object" },
                    "chapters": { "type": "array" },
                    "legal": { "type": "object" },
                    "quality": { "type": "object" }
                }
            })),
            reasoning_effort: Some(ReasoningEffort::Low),
            temperature: Some(0.4),
            max_output_tokens: Some(1000),
            safety_mode: astral_llm_domain::SafetyMode::PlatformRulesOnly,
            timeout: Duration::from_secs(5),
            metadata: GenerationMetadata {
                run_id: "run-1".to_string(),
                request_id: None,
                product_code: "natal_prompter".to_string(),
                chapter_code: Some("career".into()),
            },
        };

        let response = provider.generate(request).await.expect("fake ok");
        let json = response.parsed_json.as_ref().expect("parsed json");
        assert!(json.get("chapters").is_some());
        assert!(json.get("summary").is_some());
        assert_eq!(
            json.pointer("/chapters/0/code").and_then(|v| v.as_str()),
            Some("career")
        );
    }

    #[tokio::test]
    async fn fake_provider_returns_summary_json() {
        let provider = FakeProvider;
        let request = ProviderGenerationRequest {
            model: "fake-model".to_string(),
            messages: vec![],
            structured_schema: None,
            reasoning_effort: None,
            temperature: None,
            max_output_tokens: Some(400),
            safety_mode: astral_llm_domain::SafetyMode::PlatformRulesOnly,
            timeout: Duration::from_secs(5),
            metadata: GenerationMetadata {
                run_id: "run-1".to_string(),
                request_id: None,
                product_code: "natal_prompter".to_string(),
                chapter_code: Some(READING_SUMMARY_STEP_CODE.into()),
            },
        };

        let response = provider.generate(request).await.expect("fake ok");
        let title = response
            .parsed_json
            .as_ref()
            .and_then(|v| v.get("title"))
            .and_then(|v| v.as_str());
        assert!(title.is_some());
    }
}

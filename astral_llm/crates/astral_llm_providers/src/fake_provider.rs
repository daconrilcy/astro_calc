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


const SUMMARY_SHORT_TEXT: &str = "Votre theme met en avant une dynamique d'affirmation personnelle, \
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
            let json = if request.metadata.chapter_code.as_deref() == Some(READING_SUMMARY_STEP_CODE) {
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

    let mut basis = vec![
        astral_llm_domain::AstroBasisItem {
            fact_id: Some(interpretive),
            label: None,
            factor: "placement".into(),
            interpretive_role: "core".into(),
        },
    ];
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
            intensite contenue et moments d'ouverture. Le theme suggere une intelligence \
            affective en developpement, capable d'accueillir l'ambivalence sans la subir. Les \
            cycles interieurs trouvent un sens lorsque vous les reliez a des experiences \
            symboliques, plutot qu'a des jugements rigides sur vous-meme."
            .into(),
        "career" => "Votre trajectoire professionnelle se dessine avec pragmatisme et intuition, \
            en equilibre entre structure et creativite. Le theme souligne une ambition mesuree, \
            attentive aux contextes et aux alliances utiles. Vous progressez lorsque vos \
            motivations profondes sont reconnues, sans sacrifier votre integrite ni votre rythme \
            personnel de maturation."
            .into(),
        _ => format!(
            "Le domaine {code} du theme offre une lecture symbolique accessible, orientee vers \
             la comprehension des dynamiques observees sans posture deterministe."
        ),
    };
    format!("{core} {FAKE_CHAPTER_SUFFIX}")
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
    NatalReadingResponse {
        schema_version: "natal_reading_v1".to_string(),
        language: "fr".to_string(),
        reading_type: request.metadata.product_code.clone(),
        summary: ReadingSummary {
            title: "Lecture symbolique de demonstration".to_string(),
            short_text: "Interpretation symbolique de demonstration via FakeProvider.".to_string(),
        },
        chapters: vec![ReadingChapter {
            code: "identity".to_string(),
            title: "Identite".to_string(),
            body: "Interpretation symbolique : votre theme suggere une personnalite reflechie, \
                   orientee vers la comprehension des experiences."
                .to_string(),
            astro_basis: vec![],
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
            astro_contract_version: "unknown".to_string(),
            fallback_used: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::provider::ReasoningEffort;
    use crate::types::{GenerationMetadata, PromptMessage, PromptRole};
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

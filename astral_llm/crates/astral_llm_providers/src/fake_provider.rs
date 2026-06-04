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

const CHAPTER_BODY: &str = "Votre theme suggere une personnalite reflechie, orientee vers la \
    comprehension symbolique des experiences et des transitions interieures. Vous avancez avec \
    prudence lorsque le sens n'est pas clair, tout en montrant une grande capacite d'adaptation \
    lorsque vous sentez une direction authentique. Cette configuration invite a accueillir les \
    phases de questionnement comme des espaces creatifs, plutot que comme des blocages. Elle \
    favorise aussi une lucidite emotionnelle progressive, utile pour comprendre vos motivations \
    profondes sans vous figer dans un role unique ni rigide.";

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
        .unwrap_or_else(|| "placement:sun:capricorn:house:2".into());

    ChapterProviderResponse {
        code: code.clone(),
        title: code.replace('_', " "),
        body: CHAPTER_BODY.to_string(),
        astro_basis: vec![
            astral_llm_domain::AstroBasisItem {
                fact_id: Some(format!("domain_score:{code}")),
                label: Some(format!("Score domaine {code}")),
                factor: code.clone(),
                interpretive_role: "signal de selection du domaine".into(),
            },
            astral_llm_domain::AstroBasisItem {
                fact_id: Some(interpretive),
                label: None,
                factor: "placement".into(),
                interpretive_role: "base interpretative du chapitre".into(),
            },
        ],
        confidence: ConfidenceLevel::Medium,
    }
}

fn extract_fact_ids_from_messages(messages: &[crate::types::PromptMessage]) -> Vec<String> {
    let mut ids = Vec::new();
    for message in messages {
        if let Some(start) = message.content.find("\"facts\"") {
            let slice = &message.content[start..];
            for part in slice.split("\"id\":") {
                if let Some(rest) = part.strip_prefix(' ') {
                    let trimmed = rest.trim_start_matches('"');
                    if let Some(end) = trimmed.find('"') {
                        ids.push(trimmed[..end].to_string());
                    }
                }
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
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
            disclaimer: "Cette lecture est une interpretation symbolique et ne remplace aucun \
                          avis medical, psychologique, juridique ou financier."
                .to_string(),
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
                product_code: "natal_basic".to_string(),
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
                product_code: "natal_premium".to_string(),
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

use astral_llm_domain::{
    model_capability::ModelCapability, output_contract::ChapterContract, GenerationError,
    GenerationErrorCode,
};

use crate::reasoning_generation::apply_reasoning_output_reserve;

pub struct TokenBudget;

impl TokenBudget {
    pub fn chapter_max_tokens(
        chapter: &ChapterContract,
        engine_max: Option<u32>,
        cap: &ModelCapability,
    ) -> u32 {
        let body_budget = chapter
            .target_tokens
            .or_else(|| chapter.max_words.map(|w| w.saturating_mul(4)))
            .unwrap_or(800);

        // Structured chapter JSON (body + astro_basis[]) needs headroom beyond body text alone.
        let mut tokens = body_budget.saturating_add(500);

        if let Some(engine_max) = engine_max {
            tokens = tokens.max(engine_max);
        }

        apply_reasoning_output_reserve(cap, tokens).min(16_000)
    }

    pub fn word_count(text: &str) -> u32 {
        text.split_whitespace().count() as u32
    }

    pub fn validate_chapter_lengths(
        chapters: &[(String, String)],
        contracts: &[ChapterContract],
    ) -> Result<(), GenerationError> {
        for contract in contracts {
            let body = chapters
                .iter()
                .find(|(code, _)| code == &contract.code)
                .map(|(_, body)| body.as_str())
                .unwrap_or("");

            let words = Self::word_count(body);
            if let Some(min) = contract.min_words {
                if words < min {
                    return Err(GenerationError::with_details(
                        GenerationErrorCode::SchemaValidationFailed,
                        format!("chapter {} below min_words", contract.code),
                        serde_json::json!({ "code": contract.code, "words": words, "min": min }),
                    ));
                }
            }
            // max_words : consigne prompt uniquement, jamais bloquant en validation.
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::output_contract::ChapterContract;

    #[test]
    fn rejects_only_below_min_words() {
        let contracts = vec![ChapterContract {
            code: "career".into(),
            title: "Career".into(),
            min_words: Some(80),
            max_words: Some(300),
            target_tokens: None,
            required_fields: vec![],
        }];
        assert!(TokenBudget::validate_chapter_lengths(
            &[("career".into(), "short".into())],
            &contracts,
        )
        .is_err());
        let long = (0..400).map(|_| "word").collect::<Vec<_>>().join(" ");
        assert!(TokenBudget::validate_chapter_lengths(
            &[("career".into(), long)],
            &contracts,
        )
        .is_ok());
    }
}

use astral_llm_domain::{
    output_contract::ChapterContract, GenerationError, GenerationErrorCode,
};

/// Marge acceptee au-dela de max_words (variance LLM en sortie structuree).
const CHAPTER_MAX_WORDS_SLACK: u32 = 25;

pub struct TokenBudget;

impl TokenBudget {
    pub fn chapter_max_tokens(chapter: &ChapterContract, engine_max: Option<u32>) -> u32 {
        let body_budget = chapter
            .target_tokens
            .or_else(|| chapter.max_words.map(|w| w.saturating_mul(4)))
            .unwrap_or(800);

        // Structured chapter JSON (body + astro_basis[]) needs headroom beyond body text alone.
        let mut tokens = body_budget.saturating_add(500);

        if let Some(engine_max) = engine_max {
            tokens = tokens.max(engine_max);
        }

        tokens.min(16_000)
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
            if let Some(max) = contract.max_words {
                if words > max.saturating_add(CHAPTER_MAX_WORDS_SLACK) {
                    return Err(GenerationError::with_details(
                        GenerationErrorCode::SchemaValidationFailed,
                        format!("chapter {} above max_words", contract.code),
                        serde_json::json!({ "code": contract.code, "words": words, "max": max }),
                    ));
                }
            }
        }
        Ok(())
    }
}

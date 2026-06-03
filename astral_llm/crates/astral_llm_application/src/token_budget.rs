use astral_llm_domain::{
    output_contract::ChapterContract, GenerationError, GenerationErrorCode,
};

pub struct TokenBudget;

impl TokenBudget {
    pub fn chapter_max_tokens(chapter: &ChapterContract, global_max: Option<u32>) -> u32 {
        if let Some(tokens) = chapter.target_tokens {
            return tokens;
        }
        if let Some(words) = chapter.max_words {
            return words.saturating_mul(4);
        }
        global_max.unwrap_or(800)
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
                if words > max {
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

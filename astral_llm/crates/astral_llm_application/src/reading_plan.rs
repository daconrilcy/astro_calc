use astral_llm_domain::{
    chapter_orchestration::{ReadingPlan, ReadingPlanChapter},
    output_contract::ChapterContract,
    GenerateReadingRequest, GenerationError, GenerationErrorCode,
};

pub struct ReadingPlanBuilder;

impl ReadingPlanBuilder {
    pub fn build(request: &GenerateReadingRequest, domains: &[String]) -> ReadingPlan {
        let chapters: Vec<ReadingPlanChapter> = if !request.response_contract.chapters.is_empty() {
            request
                .response_contract
                .chapters
                .iter()
                .map(contract_to_plan_chapter)
                .collect()
        } else {
            domains
                .iter()
                .map(|code| ReadingPlanChapter {
                    code: code.clone(),
                    title: humanize_domain(code),
                    min_words: 80,
                    target_words: 150,
                    max_words: 250,
                })
                .collect()
        };

        ReadingPlan {
            product_code: request.product_context.product_code.clone(),
            domain_count: domains.len() as u8,
            selected_domains: domains.to_vec(),
            chapters,
        }
    }

    pub fn validate(plan: &ReadingPlan) -> Result<(), GenerationError> {
        if plan.chapters.is_empty() {
            return Err(GenerationError::new(
                GenerationErrorCode::InvalidInput,
                "reading plan has no chapters",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for ch in &plan.chapters {
            if !seen.insert(&ch.code) {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::InvalidInput,
                    format!("duplicate chapter code in plan: {}", ch.code),
                    serde_json::json!({ "code": ch.code }),
                ));
            }
            if ch.min_words > ch.max_words {
                return Err(GenerationError::new(
                    GenerationErrorCode::InvalidInput,
                    format!("chapter {} has min_words > max_words", ch.code),
                ));
            }
        }
        Ok(())
    }

    pub fn to_chapter_contracts(plan: &ReadingPlan) -> Vec<ChapterContract> {
        plan.chapters
            .iter()
            .map(|ch| ChapterContract {
                code: ch.code.clone(),
                title: ch.title.clone(),
                min_words: Some(ch.min_words),
                max_words: Some(ch.max_words),
                target_tokens: Some(ch.target_words.saturating_mul(4)),
                required_fields: vec!["body".into()],
            })
            .collect()
    }
}

fn contract_to_plan_chapter(contract: &ChapterContract) -> ReadingPlanChapter {
    ReadingPlanChapter {
        code: contract.code.clone(),
        title: contract.title.clone(),
        min_words: contract.min_words.unwrap_or(80),
        target_words: contract
            .max_words
            .map(|w| (contract.min_words.unwrap_or(80) + w) / 2)
            .unwrap_or(150),
        max_words: contract.max_words.unwrap_or(250),
    }
}

fn humanize_domain(code: &str) -> String {
    code.replace('_', " ")
}

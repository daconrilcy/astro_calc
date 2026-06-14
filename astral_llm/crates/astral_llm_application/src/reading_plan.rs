use astral_llm_domain::{
    chapter_orchestration::{ReadingPlan, ReadingPlanChapter},
    interpretation_profile::SYNTHESIS_CHAPTER_CODE,
    output_contract::ChapterContract,
    GenerateReadingRequest, GenerationError, GenerationErrorCode,
};

use crate::interpretation_profile_resolver::ResolvedInterpretationContext;

pub struct ReadingPlanBuilder;

impl ReadingPlanBuilder {
    pub fn build(
        request: &GenerateReadingRequest,
        domains: &[String],
        interpretation: Option<&ResolvedInterpretationContext>,
    ) -> ReadingPlan {
        let (min_w, target_w, max_w) = interpretation
            .map(|c| {
                let t = &c.profile.document.chapter_word_targets;
                (t.min, t.target, t.max)
            })
            .unwrap_or((80, 150, 300));

        let fixed_sequence = interpretation
            .map(|ctx| ctx.profile.uses_fixed_chapter_sequence())
            .unwrap_or(false);
        let use_client_chapters = !request.response_contract.chapters.is_empty() && !fixed_sequence;

        let mut chapters: Vec<ReadingPlanChapter> = if use_client_chapters {
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
                    min_words: min_w as u32,
                    target_words: target_w as u32,
                    max_words: max_w as u32,
                })
                .collect()
        };

        if let Some(ctx) = interpretation {
            if ctx.profile.has_final_synthesis_chapter()
                && !chapters.iter().any(|c| c.code == SYNTHESIS_CHAPTER_CODE)
            {
                let (syn_min, syn_target, syn_max) = ctx.profile.synthesis_word_targets();
                chapters.push(ReadingPlanChapter {
                    code: SYNTHESIS_CHAPTER_CODE.into(),
                    title: humanize_domain(SYNTHESIS_CHAPTER_CODE),
                    min_words: syn_min as u32,
                    target_words: syn_target as u32,
                    max_words: syn_max as u32,
                });
            }
        }

        ReadingPlan {
            product_code: request.product_context.product_code.clone(),
            domain_count: domains.len() as u8,
            selected_domains: domains.to_vec(),
            chapters,
        }
    }

    pub fn validate(
        plan: &ReadingPlan,
        interpretation: Option<&ResolvedInterpretationContext>,
    ) -> Result<(), GenerationError> {
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
        if let Some(ctx) = interpretation {
            let max = ctx.profile.document.max_chapters as usize;
            if plan.chapters.len() > max {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::InvalidInput,
                    format!("reading plan exceeds profile max_chapters ({max})"),
                    serde_json::json!({
                        "profile_code": ctx.profile.profile_code,
                        "chapter_count": plan.chapters.len(),
                        "max_chapters": max,
                    }),
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
    match code {
        SYNTHESIS_CHAPTER_CODE => "Synthèse intégrative".into(),
        "family_roots" => "Racines familiales".into(),
        "communication_mind" => "Communication et esprit".into(),
        "resources" => "Ressources et valeurs".into(),
        _ => code.replace('_', " "),
    }
}

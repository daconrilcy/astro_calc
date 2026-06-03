use astral_llm_domain::{
    astro_fact::NormalizedAstroFacts, generation_response::ReadingChapter, GenerationError,
    GenerationErrorCode,
};

pub struct AstroBasisValidator;

impl AstroBasisValidator {
    pub fn validate_chapters(
        chapters: &[ReadingChapter],
        facts: &NormalizedAstroFacts,
    ) -> Result<(), GenerationError> {
        for chapter in chapters {
            Self::validate_chapter_with_min_refs(chapter, facts, 0)?;
        }
        Ok(())
    }

    pub fn validate_chapter_with_min_refs(
        chapter: &ReadingChapter,
        facts: &NormalizedAstroFacts,
        min_refs: u8,
    ) -> Result<(), GenerationError> {
        let valid_refs = chapter
            .astro_basis
            .iter()
            .filter(|b| {
                b.fact_id
                    .as_ref()
                    .is_some_and(|id| facts.contains_fact(id))
            })
            .count();

        if valid_refs < min_refs as usize {
            return Err(GenerationError::with_details(
                GenerationErrorCode::SchemaValidationFailed,
                format!(
                    "chapter '{}' requires at least {min_refs} valid astro_basis references",
                    chapter.code
                ),
                serde_json::json!({
                    "chapter": chapter.code,
                    "valid_refs": valid_refs,
                    "min_refs": min_refs,
                    "available_facts": facts.fact_ids()
                }),
            ));
        }

        for basis in &chapter.astro_basis {
            if let Some(fact_id) = &basis.fact_id {
                if !facts.contains_fact(fact_id) {
                    return Err(GenerationError::with_details(
                        GenerationErrorCode::SchemaValidationFailed,
                        format!(
                            "chapter '{}' cites unknown astro fact_id: {fact_id}",
                            chapter.code
                        ),
                        serde_json::json!({
                            "chapter": chapter.code,
                            "fact_id": fact_id,
                            "available_facts": facts.fact_ids()
                        }),
                    ));
                }
            }
        }
        Ok(())
    }
}

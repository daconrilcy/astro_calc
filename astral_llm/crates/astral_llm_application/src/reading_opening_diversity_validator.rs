//! Detection des amorces repetees entre chapitres (post-lecture).

use astral_llm_domain::{
    chapter_orchestration::ReadingPlanChapter,
    generation_response::ReadingChapter,
    GenerationError, GenerationErrorCode,
};

use crate::text_trigrams::{
    chapter_opening_phrase, detect_duplicate_openings, openings_to_avoid_from_prior,
    STOCK_OPENINGS_FR,
};

pub struct ReadingOpeningDiversityValidator;

#[derive(Debug, Clone)]
pub struct OpeningViolation {
    pub chapter_code: String,
    pub phrase: String,
    pub kind: String,
}

impl ReadingOpeningDiversityValidator {
    pub fn detect(chapters: &[ReadingChapter], locale: &str) -> Vec<OpeningViolation> {
        detect_duplicate_openings(chapters, locale)
            .into_iter()
            .map(|(code, phrase, kind)| OpeningViolation {
                chapter_code: code,
                phrase,
                kind,
            })
            .collect()
    }

    pub fn validate(chapters: &[ReadingChapter], locale: &str) -> Result<(), GenerationError> {
        let violations = Self::detect(chapters, locale);
        if violations.is_empty() {
            return Ok(());
        }
        Err(GenerationError::with_details(
            GenerationErrorCode::ReadingQualityFailed,
            "Repeated chapter or paragraph openings across the reading",
            serde_json::json!({
                "violations": violations.iter().map(|v| serde_json::json!({
                    "chapter": v.chapter_code,
                    "phrase": v.phrase,
                    "kind": v.kind,
                })).collect::<Vec<_>>(),
            }),
        ))
    }

    pub fn append_opening_repair_directives(
        bundle: &mut crate::prompt_compiler::PromptBundle,
        chapter: &ReadingPlanChapter,
        prior_chapters: &[ReadingChapter],
        locale: &str,
        violations: &[OpeningViolation],
    ) {
        let avoid = openings_to_avoid_from_prior(
            &prior_chapters.iter().map(|c| c.body.as_str()).collect::<Vec<_>>(),
            locale,
            8,
        );
        bundle.task_instructions.push_str(&format!(
            "\n\nREPAIR (opening diversity): Chapter '{}'. \
             Rewrite with a distinct chapter opening (first 5 words must differ from prior chapters) \
             and distinct paragraph openings (first 4 words per paragraph). \
             Do not reuse stock formulas.",
            chapter.code
        ));
        if !avoid.is_empty() {
            bundle.task_instructions.push_str("\nBanned opening phrases from prior chapters:\n");
            for p in &avoid {
                bundle.task_instructions.push_str(&format!("- \"{p}\"\n"));
            }
        }
        for v in violations.iter().filter(|v| v.chapter_code == chapter.code) {
            bundle.task_instructions.push_str(&format!(
                "- Conflicting opening ({}) : \"{}\"\n",
                v.kind, v.phrase
            ));
        }
        if locale == "fr" {
            bundle.task_instructions.push_str(
                "\nAvoid reusing these stock openings if already used earlier in the reading:\n",
            );
            for s in STOCK_OPENINGS_FR {
                bundle.task_instructions.push_str(&format!("- {s}\n"));
            }
        }
    }

    pub fn opening_phrase_for_chapter(chapter: &ReadingChapter, locale: &str) -> String {
        chapter_opening_phrase(&chapter.body, locale)
    }
}

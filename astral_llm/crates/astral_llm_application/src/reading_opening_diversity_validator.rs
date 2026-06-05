//! Detection des amorces repetees entre chapitres (post-lecture).

use astral_llm_domain::{
    chapter_orchestration::ReadingPlanChapter,
    generation_response::ReadingChapter,
    interpretation_profile::SYNTHESIS_CHAPTER_CODE,
    GenerationError, GenerationErrorCode,
};

use crate::text_trigrams::{
    chapter_opening_phrase, detect_duplicate_openings, openings_to_avoid_from_prior,
    paragraph_opening_phrases, source_chapter_from_duplicate_kind,
    is_planet_in_sign_paragraph_opening, STOCK_OPENINGS_FR,
};

pub struct ReadingOpeningDiversityValidator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningViolation {
    pub chapter_code: String,
    pub phrase: String,
    pub kind: String,
}

impl OpeningViolation {
    pub fn is_blocking(&self) -> bool {
        self.kind.starts_with("chapter_opening_duplicate_of:")
    }
}

impl ReadingOpeningDiversityValidator {
    pub fn detect(chapters: &[ReadingChapter], locale: &str) -> Vec<OpeningViolation> {
        Self::detect_all(chapters, locale)
            .into_iter()
            .filter(|v| v.is_blocking())
            .collect()
    }

    pub fn detect_all(chapters: &[ReadingChapter], locale: &str) -> Vec<OpeningViolation> {
        detect_duplicate_openings(chapters, locale)
            .into_iter()
            .filter(|(code, _, kind)| {
                code != SYNTHESIS_CHAPTER_CODE
                    && (kind.contains("duplicate") || kind.contains("stock_opening"))
            })
            .map(|(code, phrase, kind)| OpeningViolation {
                chapter_code: code,
                phrase,
                kind,
            })
            .collect()
    }

    pub fn detect_warnings(chapters: &[ReadingChapter], locale: &str) -> Vec<OpeningViolation> {
        Self::detect_all(chapters, locale)
            .into_iter()
            .filter(|v| !v.is_blocking())
            .collect()
    }

    pub fn validate(chapters: &[ReadingChapter], locale: &str) -> Result<(), GenerationError> {
        let blocking = Self::detect(chapters, locale);
        let warnings = Self::detect_warnings(chapters, locale);
        if !warnings.is_empty() {
            tracing::warn!(
                count = warnings.len(),
                "non-blocking opening diversity warnings"
            );
        }
        if blocking.is_empty() {
            return Ok(());
        }
        Err(GenerationError::with_details(
            GenerationErrorCode::ReadingQualityFailed,
            "Repeated chapter openings across the reading",
            serde_json::json!({
                "violations": blocking.iter().map(|v| serde_json::json!({
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
        let prior_bodies: Vec<&str> = prior_chapters.iter().map(|c| c.body.as_str()).collect();
        let avoid = openings_to_avoid_from_prior(&prior_bodies, locale, 10);
        let chapter_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.chapter_code == chapter.code)
            .collect();

        bundle.task_instructions.push_str(&format!(
            "\n\nREPAIR (opening diversity) — chapter '{}': adjust openings only. \
             First 5 words of the chapter and first 4 words of EACH paragraph must be unique \
             across the whole reading. Do not paraphrase a banned opening: change structure. \
             Keep fact_ids, title, and overall length.",
            chapter.code
        ));

        let mut banned: Vec<String> = Vec::new();
        let mut push_banned = |p: String| {
            if p.split_whitespace().count() >= 3 && !banned.iter().any(|x| x == &p) {
                banned.push(p);
            }
        };

        for p in avoid {
            push_banned(p);
        }
        for v in &chapter_violations {
            push_banned(v.phrase.clone());
            if let Some(source_code) = source_chapter_from_duplicate_kind(&v.kind) {
                if let Some(src) = prior_chapters.iter().find(|c| c.code == source_code) {
                    push_banned(chapter_opening_phrase(&src.body, locale));
                    for para in paragraph_opening_phrases(&src.body) {
                        push_banned(para);
                    }
                }
            }
        }

        if !banned.is_empty() {
            bundle.task_instructions.push_str(
                "\nForbidden sentence openings (do not start the chapter or any paragraph with these):\n",
            );
            for p in &banned {
                bundle.task_instructions.push_str(&format!("- \"{p}\"\n"));
            }
        }

        let needs_astro_rule = chapter_violations
            .iter()
            .any(|v| is_planet_in_sign_paragraph_opening(&v.phrase))
            || banned.iter().any(|p| is_planet_in_sign_paragraph_opening(p));
        if needs_astro_rule {
            bundle.task_instructions.push_str(
                "\nPlacement citations: do NOT open any paragraph with « [planet] en [sign] en » \
                 (e.g. Jupiter en Cancer en…, Saturne en Capricorne en…). \
                 Start with house, aspect, life domain, or an interpretive verb; name the planet later in the sentence.\n",
            );
        }

        if locale == "fr" {
            bundle.task_instructions.push_str(
                "\nStock formulas already used elsewhere — do not reuse as openings:\n",
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

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::generation_response::{ConfidenceLevel, ReadingChapter};

    fn chapter(code: &str, body: &str) -> ReadingChapter {
        ReadingChapter {
            code: code.into(),
            title: code.into(),
            body: body.into(),
            astro_basis: vec![],
            confidence: ConfidenceLevel::Medium,
            safety_flags: vec![],
        }
    }

    #[test]
    fn paragraph_duplicate_is_warning_not_blocking() {
        let body_a = "Intro A.\n\nCette dynamique invite une lecture \
            singuliere du theme.\n\nSuite A.";
        let body_b = "Intro B.\n\nCette dynamique invite une autre \
            lecture du meme theme.\n\nSuite B.";
        let chapters = vec![chapter("identity", body_a), chapter("career", body_b)];
        assert!(ReadingOpeningDiversityValidator::detect_warnings(&chapters, "fr").len() >= 1);
        assert!(ReadingOpeningDiversityValidator::validate(&chapters, "fr").is_ok());
    }

    #[test]
    fn chapter_opening_duplicate_blocks_validation() {
        let opening = "Des le premier regard votre carte revele une force.";
        let chapters = vec![
            chapter("identity", &format!("{opening}\n\nSuite.")),
            chapter("career", &format!("{opening}\n\nAutre suite.")),
        ];
        assert!(ReadingOpeningDiversityValidator::validate(&chapters, "fr").is_err());
    }
}

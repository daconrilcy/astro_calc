//! Consignes structurelles injectees avant generation chapitre (anti-repetition en amont).

use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::ChapterEvidencePack,
};

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::prompt_compiler::PromptBundle;
use crate::text_trigrams::phrases_to_avoid_from_prior;
use astral_llm_domain::chapter_orchestration::ReadingPlanChapter;

const MAX_PRIOR_PHRASES: usize = 12;
const STOCK_OPENINGS_FR: &[&str] = &[
    "Votre parcours",
    "Cette configuration",
    "Dans l'ensemble",
    "En toile de fond",
    "Il se peut que",
    "Cette dynamique",
    "Ainsi, votre",
];

pub struct ChapterWritingGuidance;

impl ChapterWritingGuidance {
    pub fn append_upstream_directives(
        bundle: &mut PromptBundle,
        chapter: &ReadingPlanChapter,
        prior_chapters: &[ReadingChapter],
        pack: Option<&ChapterEvidencePack>,
        language: &str,
    ) {
        let locale = AstroLabelHumanizer::locale_key(language);
        let prior_bodies: Vec<&str> = prior_chapters.iter().map(|c| c.body.as_str()).collect();
        let avoid_phrases = phrases_to_avoid_from_prior(&prior_bodies, locale, MAX_PRIOR_PHRASES);

        let mut block = format!(
            "\n\n--- CHAPTER WRITING STRUCTURE (chapter '{}') ---\n\
             Mandatory body layout: exactly 4 paragraphs separated by blank lines.\n\
             - Paragraph 1: open with a fresh angle on the main CORE evidence (unique opening sentence).\n\
             - Paragraph 2: develop a second CORE or SUPPORTING evidence with different vocabulary.\n\
             - Paragraph 3: NUANCE or remaining evidence; new sentence openings only.\n\
             - Paragraph 4: brief integrative close (max 3 sentences); do not reuse opening phrases from paragraphs 1-3.\n\
             Rules: never repeat the same 3-word sequence twice in the body; vary interpretive verbs \
             (suggere, evoque, invite, revele, colore, temper, enrichit — not the same twice in a row).",
            chapter.code
        );

        if !avoid_phrases.is_empty() {
            block.push_str(
                "\nDo not reuse these 3-word sequences (already used in prior chapters):\n",
            );
            for p in &avoid_phrases {
                block.push_str(&format!("- \"{p}\"\n"));
            }
        }

        if locale == "fr" {
            block.push_str("\nUse each of these openings at most once per chapter (if at all):\n");
            for opening in STOCK_OPENINGS_FR {
                block.push_str(&format!("- {opening}\n"));
            }
        }

        if let Some(pack) = pack {
            if !pack.avoid_repeating.is_empty() {
                block.push_str(
                    "\nDo not develop these fact_ids again (other chapters): ",
                );
                block.push_str(&pack.avoid_repeating.join(", "));
                block.push('\n');
            }
        }

        block.push_str("--- END CHAPTER WRITING STRUCTURE ---");
        bundle.task_instructions.push_str(&block);
    }
}

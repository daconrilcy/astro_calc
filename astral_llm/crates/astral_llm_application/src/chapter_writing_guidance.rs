//! Consignes structurelles injectees avant generation chapitre (anti-repetition en amont).

use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::ChapterEvidencePack,
};

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::prompt_compiler::PromptBundle;
use crate::text_trigrams::{openings_to_avoid_from_prior, phrases_to_avoid_from_prior};
use astral_llm_domain::chapter_orchestration::ReadingPlanChapter;

const MAX_PRIOR_PHRASES: usize = 12;
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
        let avoid_openings = openings_to_avoid_from_prior(&prior_bodies, locale, 8);

        let mut block = format!(
            "\n\n--- CHAPTER WRITING STRUCTURE (chapter '{}') ---\n\
             Mandatory body layout: exactly 4 paragraphs separated by blank lines.\n\
             - Paragraph 1: open with a fresh angle on the main CORE evidence (unique opening sentence).\n\
             - Paragraph 2: develop a second CORE or SUPPORTING evidence with different vocabulary.\n\
             - Paragraph 3: NUANCE or remaining evidence; new sentence openings only.\n\
             - Paragraph 4: brief integrative close (max 3 sentences); do not reuse opening phrases from paragraphs 1-3.\n\
             Do not start paragraphs 2-4 with generic connectors already used earlier in this reading \
             (e.g. Par ailleurs, En synthèse, Pour finir, Sous l'influence).\n\
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

        if !avoid_openings.is_empty() {
            block.push_str(
                "\nDo not start the chapter or any paragraph with these openings (prior chapters):\n",
            );
            for p in &avoid_openings {
                block.push_str(&format!("- \"{p}\"\n"));
            }
        }

        if locale == "fr" {
            block.push_str(
                "\nStock formulas — use at most once in the whole reading, never as chapter opening if already used: \
                 \"Dès le premier regard\", \"Cette configuration\", \"Votre parcours\", \"Cette dynamique\".\n",
            );
        }

        if let Some(pack) = pack {
            if !pack.avoid_repeating.is_empty() {
                block.push_str(
                    "\nDo not develop these interpretive facts again (semantic keys from prior chapter cores): ",
                );
                block.push_str(&pack.avoid_repeating.join(", "));
                block.push('\n');
            }
            let basis_lines: Vec<String> = pack
                .core
                .iter()
                .map(|e| format!("- {} (core)", e.fact_id))
                .chain(
                    pack.supporting
                        .iter()
                        .map(|e| format!("- {} (supporting)", e.fact_id)),
                )
                .collect();
            if !basis_lines.is_empty() {
                block.push_str(
                    "\nMandatory astro_basis: list every fact_id below with the matching interpretive_role \
                     before writing the body (no omissions):\n",
                );
                for line in &basis_lines {
                    block.push_str(line);
                    block.push('\n');
                }
            }
        }

        block.push_str("--- END CHAPTER WRITING STRUCTURE ---");
        bundle.task_instructions.push_str(&block);
    }
}

//! Consignes structurelles injectees avant generation chapitre (anti-repetition en amont).

use astral_llm_domain::{
    generation_response::ReadingChapter,
    interpretive_evidence::ChapterEvidencePack,
    interpretation_profile::BodyStructureConfig,
};

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::prompt_compiler::PromptBundle;
use crate::text_trigrams::{openings_to_avoid_from_prior, phrases_to_avoid_from_prior};
use astral_llm_domain::chapter_orchestration::ReadingPlanChapter;

const MAX_PRIOR_PHRASES: usize = 8;
const MAX_PRIOR_OPENINGS: usize = 8;

pub struct ChapterWritingGuidance;

impl ChapterWritingGuidance {
    pub fn append_upstream_directives(
        bundle: &mut PromptBundle,
        chapter: &ReadingPlanChapter,
        prior_chapters: &[ReadingChapter],
        pack: Option<&ChapterEvidencePack>,
        language: &str,
        interpretation: Option<&ResolvedInterpretationContext>,
    ) {
        let locale = AstroLabelHumanizer::locale_key(language);
        let prior_bodies: Vec<&str> = prior_chapters.iter().map(|c| c.body.as_str()).collect();
        let avoid_phrases = phrases_to_avoid_from_prior(&prior_bodies, locale, MAX_PRIOR_PHRASES);
        let avoid_openings = openings_to_avoid_from_prior(&prior_bodies, locale, MAX_PRIOR_OPENINGS);
        let rich_editorial = interpretation
            .map(|ctx| ctx.profile.uses_rich_editorial_structure())
            .unwrap_or(false);

        let mut block = if rich_editorial {
            let bs = interpretation
                .and_then(|ctx| ctx.profile.body_structure().cloned())
                .unwrap_or_else(default_editorial_body_structure);
            let (min_w, target_w, max_w) = interpretation
                .map(|ctx| {
                    let t = &ctx.profile.document.chapter_word_targets;
                    (t.min, t.target, t.max)
                })
                .unwrap_or((520, 650, 850));
            format!(
                "\n\n--- CHAPTER WRITING STRUCTURE (chapter '{}') ---\n\
                 Mandatory body layout: exactly {} paragraphs separated by blank lines.\n\
                 Each paragraph: {}–{} words.\n\
                 Total body length: {}–{} words; target ~{}.\n\
                 Do not stop after merely satisfying the minimum.\n\
                 Suggested progression:\n\
                 1. embodied opening on the chapter theme (fresh angle, unique opening sentence).\n\
                 2. main astrological development using CORE evidence.\n\
                 3. supporting evidence and symbolic interpretation.\n\
                 4. nuance, paradox, or tension when relevant.\n\
                 5. concrete life manifestation (symbolic, non-prescriptive).\n\
                 6. integrative close or symbolic vigilance point.\n\
                 Avoid visible repetitions: same paragraph openings, stock formulas, or distinctive phrases already used.\n\
                 Natural grammatical repetitions are acceptable.\n\
                 Do not start paragraphs with raw placement citations such as « Saturne en Capricorne… »; \
                 start with the life domain or interpretive idea; cite the astrological factor later in the sentence.\n\
                 Do not start consecutive paragraphs with generic connectors already used earlier in this reading \
                 (e.g. Par ailleurs, En synthèse, Pour finir, Sous l'influence).",
                chapter.code,
                bs.paragraph_count,
                bs.paragraph_min_words,
                bs.paragraph_max_words,
                min_w,
                max_w,
                target_w
            )
        } else {
            let bs = interpretation
                .and_then(|ctx| ctx.profile.body_structure().cloned())
                .unwrap_or_else(default_compact_body_structure);
            let (min_w, target_w, max_w) = interpretation
                .map(|ctx| {
                    let t = &ctx.profile.document.chapter_word_targets;
                    (t.min, t.target, t.max)
                })
                .unwrap_or((280, 350, 450));
            format!(
                "\n\n--- CHAPTER WRITING STRUCTURE (chapter '{}') ---\n\
                 Mandatory body layout: exactly {} paragraphs separated by blank lines.\n\
                 Each paragraph: {}–{} words.\n\
                 Total body length: {}–{} words; target ~{}.\n\
                 - Paragraph 1: open with a fresh angle on the main CORE evidence (unique opening sentence).\n\
                 - Paragraph 2: develop a second CORE or SUPPORTING evidence with different vocabulary.\n\
                 - Paragraph 3: NUANCE or remaining evidence; new sentence openings only.\n\
                 - Paragraph 4: brief integrative close (max 3 sentences); do not reuse opening phrases from paragraphs 1-3.\n\
                 Avoid visible repetitions: same paragraph openings, stock formulas, or distinctive phrases already used.\n\
                 Natural grammatical repetitions are acceptable.\n\
                 Do not start paragraphs 2-4 with generic connectors already used earlier in this reading \
                 (e.g. Par ailleurs, En synthèse, Pour finir, Sous l'influence).",
                chapter.code,
                bs.paragraph_count,
                bs.paragraph_min_words,
                bs.paragraph_max_words,
                min_w,
                max_w,
                target_w
            )
        };

        if !avoid_phrases.is_empty() {
            block.push_str(
                "\nDo not reuse these distinctive 3-word sequences (already used in prior chapters):\n",
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
                .chain(pack.nuance.iter().map(|e| format!("- {} (nuance)", e.fact_id)))
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

fn default_editorial_body_structure() -> BodyStructureConfig {
    BodyStructureConfig {
        paragraph_count: 6,
        paragraph_min_words: 80,
        paragraph_max_words: 120,
        style: "editorial_flow".into(),
    }
}

fn default_compact_body_structure() -> BodyStructureConfig {
    BodyStructureConfig {
        paragraph_count: 4,
        paragraph_min_words: 60,
        paragraph_max_words: 90,
        style: "compact_flow".into(),
    }
}

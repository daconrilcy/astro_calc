use std::time::Instant;

use astral_llm_domain::{
    chapter_orchestration::{ChapterGenerationStatus, ReadingPlanChapter},
    generation_response::ReadingChapter,
    GenerationError, GenerationErrorCode,
};

use crate::engine_defaults::ResolvedEngineParams;
use crate::execution_audit::ExecutionAudit;
use crate::prompt_compiler::PromptBundle;
use crate::reading_opening_diversity_validator::OpeningViolation;
use crate::reading_quality_validator::{PremiumQualityThresholds, ReadingQualityValidator};

#[derive(Debug, Clone, PartialEq)]
pub enum ChapterRepairKind {
    TooShort {
        words: u32,
        min_words: u32,
        max_words: u32,
    },
    Repetition { score: usize, max_allowed: usize },
    EvidenceCoherence {
        missing_pack_fact_ids: Vec<String>,
        orphan_object_codes: Vec<String>,
    },
    OpeningDiversity {
        violations: Vec<OpeningViolation>,
    },
}

pub struct ChapterOutcome {
    pub reading_chapter: ReadingChapter,
    pub bundle: PromptBundle,
    pub status: ChapterGenerationStatus,
    pub route_meta: (String, String, bool, Option<u32>, Option<u32>),
}

const MAX_REPETITION_REPAIR_ATTEMPTS: usize = 3;
const MAX_MIN_WORDS_REPAIR_ATTEMPTS: usize = 2;

pub fn length_repair_from_error(
    err: &GenerationError,
    chapter: &ReadingPlanChapter,
) -> ChapterRepairKind {
    let details = err.detail().details.as_ref();
    let words = details
        .and_then(|d| d.get("words"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(0);
    ChapterRepairKind::TooShort {
        words,
        min_words: chapter.min_words,
        max_words: chapter.max_words,
    }
}

pub async fn retry_chapter_on_min_words<F, Fut>(
    chapter: &ReadingPlanChapter,
    initial_err: GenerationError,
    run_id: &str,
    engine: &ResolvedEngineParams,
    started: Instant,
    audit: &mut ExecutionAudit,
    regenerate: F,
) -> Result<
    (
        ReadingChapter,
        PromptBundle,
        (String, String, bool, Option<u32>, Option<u32>),
    ),
    GenerationError,
>
where
    F: Fn(Option<ChapterRepairKind>) -> Fut,
    Fut: std::future::Future<
        Output = Result<
            (
                ReadingChapter,
                PromptBundle,
                (String, String, bool, Option<u32>, Option<u32>),
            ),
            GenerationError,
        >,
    >,
{
    let mut last_err = initial_err;
    for attempt in 0..MAX_MIN_WORDS_REPAIR_ATTEMPTS {
        let repair = length_repair_from_error(&last_err, chapter);
        tracing::info!(
            run_id,
            chapter = %chapter.code,
            attempt = attempt + 1,
            "chapter below min_words, attempting repair"
        );
        match regenerate(Some(repair)).await {
            Ok(ok) => return Ok(ok),
            Err(e) if is_min_words_violation(&e) && attempt + 1 < MAX_MIN_WORDS_REPAIR_ATTEMPTS => {
                last_err = e;
            }
            Err(e) => return Err(e),
        }
    }
    audit.record_chapter_step(
        &chapter.code,
        engine.provider.as_str(),
        &engine.model,
        ChapterGenerationStatus::Failed,
        None,
        None,
        started.elapsed().as_millis() as u64,
        Some(GenerationErrorCode::SchemaValidationFailed.as_str().to_string()),
    );
    Err(last_err)
}

pub fn is_min_words_violation(err: &GenerationError) -> bool {
    matches!(
        err.detail().code,
        GenerationErrorCode::SchemaValidationFailed
    ) && err.detail().message.contains("below min_words")
}

pub async fn maybe_repair_repetition<F, Fut>(
    chapter: &ReadingPlanChapter,
    reading_chapter: ReadingChapter,
    bundle: PromptBundle,
    route_meta: (String, String, bool, Option<u32>, Option<u32>),
    quality_thresholds: &PremiumQualityThresholds,
    locale: &str,
    run_id: &str,
    engine: &ResolvedEngineParams,
    started: Instant,
    audit: &mut ExecutionAudit,
    regenerate: F,
) -> Result<ChapterOutcome, GenerationError>
where
    F: Fn(Option<ChapterRepairKind>) -> Fut,
    Fut: std::future::Future<
        Output = Result<
            (
                ReadingChapter,
                PromptBundle,
                (String, String, bool, Option<u32>, Option<u32>),
            ),
            GenerationError,
        >,
    >,
{
    if !ReadingQualityValidator::chapter_exceeds_repetition(
        &reading_chapter.body,
        quality_thresholds,
        locale,
    ) {
        return Ok(ChapterOutcome {
            reading_chapter,
            bundle,
            status: ChapterGenerationStatus::Generated,
            route_meta,
        });
    }

    let initial_score =
        ReadingQualityValidator::chapter_repetition_score(&reading_chapter.body, locale);
    tracing::info!(
        run_id,
        chapter = %chapter.code,
        repetition_score = initial_score,
        max_allowed = quality_thresholds.max_repeated_trigrams,
        "chapter repetition above threshold, attempting repair"
    );

    let mut best_chapter = reading_chapter;
    let mut best_bundle = bundle;
    let mut best_meta = route_meta;
    let mut best_score = initial_score;

    for attempt in 0..MAX_REPETITION_REPAIR_ATTEMPTS {
        match regenerate(Some(ChapterRepairKind::Repetition {
            score: best_score,
            max_allowed: quality_thresholds.max_repeated_trigrams,
        }))
        .await
        {
            Ok((repaired, repaired_bundle, repaired_meta)) => {
                let score =
                    ReadingQualityValidator::chapter_repetition_score(&repaired.body, locale);
                if score <= best_score {
                    best_score = score;
                    best_chapter = repaired;
                    best_bundle = repaired_bundle;
                    best_meta = repaired_meta;
                }
                if !ReadingQualityValidator::chapter_exceeds_repetition(
                    &best_chapter.body,
                    quality_thresholds,
                    locale,
                ) {
                    return Ok(ChapterOutcome {
                        reading_chapter: best_chapter,
                        bundle: best_bundle,
                        status: ChapterGenerationStatus::Repaired,
                        route_meta: best_meta,
                    });
                }
                tracing::info!(
                    run_id,
                    chapter = %chapter.code,
                    attempt = attempt + 1,
                    repetition_score = score,
                    best_score,
                    "repetition repair attempt still above threshold"
                );
            }
            Err(repair_err) => {
                audit.record_chapter_step(
                    &chapter.code,
                    engine.provider.as_str(),
                    &engine.model,
                    ChapterGenerationStatus::Failed,
                    None,
                    None,
                    started.elapsed().as_millis() as u64,
                    Some(repair_err.detail().code.as_str().to_string()),
                );
                return Err(repair_err);
            }
        }
    }

    audit.record_chapter_step(
        &chapter.code,
        engine.provider.as_str(),
        &engine.model,
        ChapterGenerationStatus::Failed,
        best_meta.3,
        best_meta.4,
        started.elapsed().as_millis() as u64,
        Some(GenerationErrorCode::ReadingQualityFailed.as_str().to_string()),
    );
    Err(GenerationError::with_details(
        GenerationErrorCode::ReadingQualityFailed,
        "chapter repetition still above threshold after repair",
        serde_json::json!({
            "chapter": chapter.code,
            "initial_score": initial_score,
            "repetition_score": best_score,
            "max_allowed": quality_thresholds.max_repeated_trigrams,
            "repair_attempts": MAX_REPETITION_REPAIR_ATTEMPTS,
        }),
    ))
}

pub fn append_repair_instructions(
    bundle: &mut PromptBundle,
    chapter: &ReadingPlanChapter,
    repair: ChapterRepairKind,
) {
    match repair {
        ChapterRepairKind::TooShort {
            words,
            min_words,
            max_words,
        } => {
            bundle.task_instructions.push_str(&format!(
                "\n\nREPAIR: Chapter '{}' is only {words} words; expand the body to at least {min_words} words \
                 (target near {max_words}, but do not shorten if already long enough). Keep all fact_ids valid.",
                chapter.code
            ));
        }
        ChapterRepairKind::Repetition { score, max_allowed } => {
            bundle.task_instructions.push_str(&format!(
                "\n\nREPAIR: Rewrite chapter '{}' with varied vocabulary and sentence openings. \
                 Do not reuse the same three-word phrases. Current repetition score is {score} (max {max_allowed}). \
                 Vary transitions (cependant, par ailleurs, en revanche). Keep fact_ids valid and interpretive framing.",
                chapter.code
            ));
        }
        ChapterRepairKind::EvidenceCoherence {
            missing_pack_fact_ids,
            orphan_object_codes,
        } => {
            bundle.task_instructions.push_str(&format!(
                "\n\nREPAIR (evidence coherence): Chapter '{}'. \
                 Include EVERY CORE and SUPPORTING fact_id from the chapter evidence pack in astro_basis \
                 (matching interpretive_role). Missing in astro_basis: {:?}. \
                 Remove or replace body passages that cite celestial objects not listed in astro_basis. \
                 Orphan mentions (not backed by astro_basis): {:?}. \
                 Do not develop placements or planets absent from the pack.",
                chapter.code, missing_pack_fact_ids, orphan_object_codes
            ));
        }
        ChapterRepairKind::OpeningDiversity { .. } => {
            bundle.task_instructions.push_str(&format!(
                "\n\nREPAIR (opening diversity): Chapter '{}' — rewrite the full body; \
                 follow the detailed banned-opening list below.",
                chapter.code
            ));
        }
    }
}

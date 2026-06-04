use std::time::Instant;

use astral_llm_domain::{
    chapter_orchestration::{ChapterGenerationStatus, ReadingPlanChapter},
    generation_response::ReadingChapter,
    GenerationError, GenerationErrorCode,
};

use crate::engine_defaults::ResolvedEngineParams;
use crate::execution_audit::ExecutionAudit;
use crate::prompt_compiler::PromptBundle;
use crate::reading_quality_validator::{PremiumQualityThresholds, ReadingQualityValidator};

#[derive(Debug, Clone, Copy)]
pub enum ChapterRepairKind {
    Length,
    Repetition { score: usize, max_allowed: usize },
}

pub struct ChapterOutcome {
    pub reading_chapter: ReadingChapter,
    pub bundle: PromptBundle,
    pub status: ChapterGenerationStatus,
    pub route_meta: (String, String, bool, Option<u32>, Option<u32>),
}

const MAX_REPETITION_REPAIR_ATTEMPTS: usize = 2;

pub async fn maybe_repair_repetition<F, Fut>(
    chapter: &ReadingPlanChapter,
    reading_chapter: ReadingChapter,
    bundle: PromptBundle,
    route_meta: (String, String, bool, Option<u32>, Option<u32>),
    quality_thresholds: &PremiumQualityThresholds,
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
    ) {
        return Ok(ChapterOutcome {
            reading_chapter,
            bundle,
            status: ChapterGenerationStatus::Generated,
            route_meta,
        });
    }

    let initial_score = ReadingQualityValidator::chapter_repetition_score(&reading_chapter.body);
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
                let score = ReadingQualityValidator::chapter_repetition_score(&repaired.body);
                if score <= best_score {
                    best_score = score;
                    best_chapter = repaired;
                    best_bundle = repaired_bundle;
                    best_meta = repaired_meta;
                }
                if !ReadingQualityValidator::chapter_exceeds_repetition(
                    &best_chapter.body,
                    quality_thresholds,
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
        ChapterRepairKind::Length => {
            bundle.task_instructions.push_str(&format!(
                "\n\nREPAIR: Adjust chapter '{}' to between {} and {} words. Keep fact_ids valid.",
                chapter.code, chapter.min_words, chapter.max_words
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
    }
}

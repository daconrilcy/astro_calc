use super::*;
use crate::domain::BasicAccidentalDignityEvaluation;
use crate::shared::error::RuntimeError;

pub(super) fn build_strengths(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmStrengths, RuntimeError> {
    let mut dignity_rows: Vec<_> = payload.dignities.iter().collect();
    dignity_rows.sort_by(|a, b| {
        b.strength_score
            .partial_cmp(&a.strength_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let essential_dignities = dignity_rows
        .into_iter()
        .take(payload.dignities.len().max(1))
        .map(|d| -> Result<LlmEssentialDignity, RuntimeError> {
            Ok(LlmEssentialDignity {
                object: d.object_name.clone(),
                dignity: d.dignity_label.clone(),
                sign: d.sign_name.clone(),
                meaning: super::super::humanize::dignity_meaning(&d.dignity_type, resolver)?,
                strength_score: profile.include_scores.then_some(d.strength_score),
            })
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    let accidental_conditions = if profile.include_accidental_conditions {
        payload
            .accidental_dignities
            .iter()
            .map(|entry| accidental_to_llm(entry, payload, profile, resolver))
            .collect::<Result<Vec<_>, RuntimeError>>()?
    } else {
        Vec::new()
    };

    Ok(LlmStrengths {
        essential_dignities,
        accidental_conditions,
    })
}

fn accidental_to_llm(
    entry: &BasicAccidentalDignityEvaluation,
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmAccidentalCondition, RuntimeError> {
    Ok(LlmAccidentalCondition {
        object: entry.object_name.clone(),
        overall: super::super::humanize::accidental_overall_label(
            &entry.expression_quality,
            &entry.overall_polarity,
        ),
        conditions: {
            let mut out = Vec::new();
            for condition in &entry.conditions {
                push_unique(
                    &mut out,
                    humanize_condition(&condition.condition_code, chart_sect(payload), resolver)?,
                );
                if out.len() >= profile.max_accidental_conditions_per_object {
                    break;
                }
            }
            out
        },
        overall_score: profile.include_scores.then_some(entry.overall_score),
    })
}

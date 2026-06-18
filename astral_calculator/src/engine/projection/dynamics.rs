//! Module astral_calculator\src\engine\projection\dynamics.rs du moteur astral_calculator.

use super::clean_text::{
    clean_semantic_tags, humanize_dynamic_quality, humanize_phase, humanize_valence,
    limit_keywords, title_case_sign, ProjectionTextCatalog,
};
use super::types::{LlmDynamics, LlmLunarPhase, LlmMajorAspect, LlmProjectionProfile};
use crate::domain::{BasicPayload, BasicSignal};
use crate::shared::error::RuntimeError;

/// Fonction build_dynamics.
pub fn build_dynamics(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmDynamics, RuntimeError> {
    let lunar_phase = payload.lunar_phase_context.as_ref().map(|phase| {
        let keywords = clean_semantic_tags(&phase.semantic_tags, profile.max_keywords_per_item);
        LlmLunarPhase {
            phase: phase.phase_label.clone(),
            cycle: title_case_sign(&phase.cycle_family),
            sun_moon_angle_degrees: phase.sun_moon_angle_deg,
            keywords,
        }
    });

    let mut aspect_signals: Vec<&BasicSignal> = payload
        .signals
        .iter()
        .filter(|signal| is_active_major_aspect_signal(signal))
        .collect();
    aspect_signals.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let major_aspects = aspect_signals
        .into_iter()
        .take(profile.max_aspects)
        .map(|signal| {
            aspect_signal_to_llm(signal, payload, profile.max_keywords_per_item, resolver)
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    Ok(LlmDynamics {
        lunar_phase,
        major_aspects,
    })
}

/// Fonction is_active_major_aspect_signal.
pub fn is_active_major_aspect_signal(signal: &BasicSignal) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return false;
    }
    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };
    if evidence.get("fact_type").and_then(|v| v.as_str()) != Some("aspect") {
        return false;
    }
    if signal.aspect_context.is_none() {
        return false;
    }
    let family = evidence
        .get("aspect_family")
        .and_then(|v| v.as_str())
        .or_else(|| {
            signal
                .aspect_context
                .as_ref()?
                .get("aspect_family")
                .and_then(|v| v.as_str())
        });
    family == Some("major")
}

/// Fonction aspect_signal_to_llm.
fn aspect_signal_to_llm(
    signal: &BasicSignal,
    payload: &BasicPayload,
    keyword_limit: usize,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmMajorAspect, RuntimeError> {
    let evidence = signal.evidence.as_ref().ok_or_else(|| {
        RuntimeError::InvalidProjectionLabelDefinition(format!(
            "missing aspect evidence for signal '{}'",
            signal.signal_key
        ))
    })?;
    let ctx = signal.aspect_context.as_ref().ok_or_else(|| {
        RuntimeError::InvalidProjectionLabelDefinition(format!(
            "missing aspect_context for signal '{}'",
            signal.signal_key
        ))
    })?;
    let orb = evidence
        .get("orb_deg")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            RuntimeError::InvalidProjectionLabelDefinition(format!(
                "missing orb_deg for signal '{}'",
                signal.signal_key
            ))
        })?;
    let phase = evidence
        .get("phase_state")
        .and_then(|v| v.as_str())
        .or_else(|| ctx.get("phase_state").and_then(|v| v.as_str()))
        .ok_or_else(|| {
            RuntimeError::InvalidProjectionLabelDefinition(format!(
                "missing phase_state for signal '{}'",
                signal.signal_key
            ))
        })?;
    let quality_code = ctx
        .get("dynamic_quality")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            RuntimeError::InvalidProjectionLabelDefinition(format!(
                "missing dynamic_quality for signal '{}'",
                signal.signal_key
            ))
        })?;

    let quality = humanize_dynamic_quality(quality_code, resolver)?;

    let valence_code = ctx
        .get("primary_valence")
        .and_then(|v| v.as_str())
        .or_else(|| ctx.get("intensity_modifier").and_then(|v| v.as_str()))
        .ok_or_else(|| {
            RuntimeError::InvalidProjectionLabelDefinition(format!(
                "missing valence code for signal '{}'",
                signal.signal_key
            ))
        })?;
    let valence = humanize_valence(valence_code, resolver)?;

    let source_name = evidence
        .get("source_object_name")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let target_name = evidence
        .get("target_object_name")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let objects = match (source_name, target_name) {
        (Some(a), Some(b)) => vec![a, b],
        _ => Vec::new(),
    };

    let mut keywords = clean_semantic_tags(&signal.semantic_tags, keyword_limit);
    const MAX_OBJECT_KEYWORDS: usize = 2;
    for object in &objects {
        if keywords.len() >= keyword_limit {
            break;
        }
        let Some(position) = payload.positions.iter().find(|p| p.object_name == *object) else {
            continue;
        };
        let object_kws: Vec<String> = position
            .sign_context
            .as_ref()
            .and_then(|ctx| ctx.get("keywords"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        for kw in limit_keywords(&object_kws, MAX_OBJECT_KEYWORDS) {
            keywords.push(kw);
            if keywords.len() >= keyword_limit {
                break;
            }
        }
    }
    keywords = limit_keywords(&keywords, keyword_limit);

    Ok(LlmMajorAspect {
        aspect: signal.title.clone(),
        objects,
        quality,
        valence,
        orb_degrees: (orb * 100.0).round() / 100.0,
        phase: humanize_phase(phase, resolver)?,
        keywords,
    })
}

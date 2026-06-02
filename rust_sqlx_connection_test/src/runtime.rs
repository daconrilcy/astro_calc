use std::collections::{HashMap, HashSet};

use chrono::Utc;
use sqlx::PgPool;
use thiserror::Error;

use crate::domain::{
    BasicPayload, BasicSignal, CalculatedChartFacts, CalculationReferenceData, NatalChartInput,
    RuntimeOptions,
};
use crate::ephemeris::EphemerisEngine;
use crate::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use crate::models::{ChartCalculationRow, ChartObject};
use crate::payload::build_basic_payload;
use crate::repositories::RuntimeRepository;
use crate::signals::aggregate_basic_signals;

const SIGN_HOUSE_EMPHASIS_MIN_SCORE: f64 = 0.35;
const OBJECT_EMPHASIS_MIN_SCORE: f64 = 0.5;
const MAX_DOMINANT_SIGNS: usize = 3;
const MAX_DOMINANT_HOUSES: usize = 3;
const MAX_DOMINANT_OBJECTS: usize = 5;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ephemeris error: {0}")]
    Ephemeris(String),
    #[error("invalid runtime table: {0}")]
    InvalidRuntimeTable(String),
    #[error("calculation is already running for idempotency key {idempotency_key}")]
    RunningCalculationInProgress {
        idempotency_key: String,
        chart_calculation_id: i32,
    },
}

impl RuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "database_error",
            Self::Json(_) => "json_error",
            Self::Ephemeris(_) => "ephemeris_error",
            Self::InvalidRuntimeTable(_) => "invalid_runtime_table",
            Self::RunningCalculationInProgress { .. } => "running_calculation_in_progress",
        }
    }
}

pub struct ChartCalculationRuntimeService<E> {
    repository: RuntimeRepository,
    ephemeris: E,
    options: RuntimeOptions,
}

impl<E> ChartCalculationRuntimeService<E>
where
    E: EphemerisEngine,
{
    pub fn new(pool: PgPool, ephemeris: E, options: RuntimeOptions) -> Self {
        Self {
            repository: RuntimeRepository::new(pool),
            ephemeris,
            options,
        }
    }

    pub async fn calculate_natal_basic(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicPayload, RuntimeError> {
        let product_code = input.product_code().to_string();
        let payload_language_id = self.repository.language_id_for_code("en").await?;
        let input_hash = input_hash(&input)?;
        let idempotency_key = idempotency_key(&input, &self.options)?;
        let lock_key = advisory_lock_key(&idempotency_key);

        let chart_objects = self
            .repository
            .active_chart_objects(input.reference_version_id)
            .await?;
        validate_chart_object_signal_profiles(&chart_objects)?;
        let aspect_definitions = self.repository.aspect_definitions().await?;
        let house_system = self.repository.house_system(input.house_system_id).await?;
        let references = CalculationReferenceData {
            signs: self.repository.sign_references().await?,
            houses: self.repository.house_references().await?,
            motion_states: self.repository.motion_state_references().await?,
            angle_points: self.repository.angle_point_references().await?,
        };
        validate_calculation_references(&references)?;

        let mut tx = self.repository.pool().begin().await?;
        RuntimeRepository::lock_idempotency(&mut tx, lock_key).await?;

        let existing = RuntimeRepository::calculations_for_key(&mut tx, &idempotency_key).await?;
        if let Some(completed) = existing.iter().find(|row| row.status == "completed") {
            let completed_id = completed.id;
            if let Some(payload) = self
                .repository
                .existing_basic_payload(completed_id, &product_code, Some(payload_language_id))
                .await?
            {
                if is_current_basic_payload(&payload) {
                    tx.commit().await?;
                    return Ok(payload);
                }
            }
            let positions = self.repository.positions_for_payload(completed_id).await?;
            if has_required_angle_positions(&positions, &references) {
                let aspects = self.repository.aspects_for_payload(completed_id).await?;
                let signal_drafts = aggregate_basic_signals(&CalculatedChartFacts {
                    positions: positions.clone(),
                    house_cusps: Vec::new(),
                    aspects,
                });
                tx.commit().await?;
                let mut payload_tx = self.repository.pool().begin().await?;
                let signals = RuntimeRepository::persist_signals(
                    &mut payload_tx,
                    completed_id,
                    input.reference_version_id,
                    &signal_drafts,
                )
                .await?;
                let payload = build_basic_payload(completed_id, &input, &positions, &signals);
                RuntimeRepository::persist_basic_payload(
                    &mut payload_tx,
                    &input,
                    Some(payload_language_id),
                    &payload,
                )
                .await?;
                payload_tx.commit().await?;
                return Ok(payload);
            }
        } else if let Some(running) = existing.iter().find(|row| row.status == "running") {
            if is_stale(running, self.options.stale_after_seconds) {
                RuntimeRepository::mark_stale_failed(&mut tx, running.id).await?;
            } else {
                let chart_calculation_id = running.id;
                tx.commit().await?;
                return Err(RuntimeError::RunningCalculationInProgress {
                    idempotency_key,
                    chart_calculation_id,
                });
            }
        }

        let next_attempt = existing
            .first()
            .map(|row| row.execution_attempt + 1)
            .unwrap_or(1);
        let chart_calculation_id = RuntimeRepository::insert_running_calculation(
            &mut tx,
            &input,
            &self.options,
            &input_hash,
            &idempotency_key,
            next_attempt,
        )
        .await?;
        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "calculating_facts").await?;

        let facts = match self.ephemeris.calculate_natal(
            &input,
            &chart_objects,
            &aspect_definitions,
            &house_system,
            &references,
        ) {
            Ok(value) => value,
            Err(error) => {
                RuntimeRepository::mark_failed(&mut tx, chart_calculation_id, &error).await?;
                tx.commit().await?;
                return Err(error);
            }
        };

        RuntimeRepository::persist_facts(&mut tx, chart_calculation_id, &facts).await?;
        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "aggregating_signals").await?;
        let aspects =
            RuntimeRepository::aspects_for_payload_in_tx(&mut tx, chart_calculation_id).await?;
        let enriched_facts = CalculatedChartFacts {
            positions: facts.positions,
            house_cusps: Vec::new(),
            aspects,
        };
        let signal_drafts = aggregate_basic_signals(&enriched_facts);
        let signal_rows = RuntimeRepository::persist_signals(
            &mut tx,
            chart_calculation_id,
            input.reference_version_id,
            &signal_drafts,
        )
        .await?;

        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "building_payload").await?;
        let payload = build_basic_payload(
            chart_calculation_id,
            &input,
            &enriched_facts.positions,
            &signal_rows,
        );
        RuntimeRepository::persist_basic_payload(
            &mut tx,
            &input,
            Some(payload_language_id),
            &payload,
        )
        .await?;
        RuntimeRepository::mark_completed(&mut tx, chart_calculation_id).await?;
        tx.commit().await?;

        Ok(payload)
    }
}

pub fn validate_calculation_references(
    references: &CalculationReferenceData,
) -> Result<(), RuntimeError> {
    if references.signs.len() != 12 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 12 sign references, found {}",
            references.signs.len()
        )));
    }
    if references.houses.len() != 12 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 12 house references, found {}",
            references.houses.len()
        )));
    }
    if references.motion_states.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected motion state references".to_string(),
        ));
    }
    if references.angle_points.len() != 4 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 4 angle point references, found {}",
            references.angle_points.len()
        )));
    }

    let mut sign_ids = HashSet::new();
    for sign in &references.signs {
        if !sign_ids.insert(sign.id) || sign.code.trim().is_empty() || sign.name.trim().is_empty() {
            return Err(RuntimeError::Ephemeris(
                "invalid sign references: duplicate IDs or empty labels".to_string(),
            ));
        }
    }

    let mut house_ids = HashSet::new();
    let mut house_numbers = HashSet::new();
    for house in &references.houses {
        if !house_ids.insert(house.id)
            || !house_numbers.insert(house.number)
            || !(1..=12).contains(&house.number)
            || house.name.trim().is_empty()
            || house.modality_code.is_none()
            || house.modality_priority_delta.is_none()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid house references: duplicate IDs, invalid numbers, empty labels, or missing modality scoring".to_string(),
            ));
        }
    }

    let mut motion_state_ids = HashSet::new();
    for motion_state in &references.motion_states {
        if !motion_state_ids.insert(motion_state.id)
            || motion_state.code.trim().is_empty()
            || motion_state.label.trim().is_empty()
            || motion_state.motion_family.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid motion state references: duplicate IDs or empty labels".to_string(),
            ));
        }
    }

    let mut angle_ids = HashSet::new();
    let mut angle_object_ids = HashSet::new();
    for angle in &references.angle_points {
        if !angle_ids.insert(angle.id)
            || !angle_object_ids.insert(angle.chart_object_id)
            || angle.code.trim().is_empty()
            || angle.short_label.trim().is_empty()
            || angle.full_name.trim().is_empty()
            || angle.axis.trim().is_empty()
            || !(1..=12).contains(&angle.associated_house)
            || angle.chart_object_code.trim().is_empty()
            || angle.chart_object_name.trim().is_empty()
        {
            return Err(RuntimeError::Ephemeris(
                "invalid angle point references: duplicate IDs, invalid houses, or empty labels"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

pub fn validate_chart_object_signal_profiles(
    chart_objects: &[ChartObject],
) -> Result<(), RuntimeError> {
    if chart_objects.is_empty() {
        return Err(RuntimeError::Ephemeris(
            "expected active chart object references".to_string(),
        ));
    }

    for object in chart_objects {
        let has_base_priority = object
            .position_priority_base
            .is_some_and(|value| (0.0..=100.0).contains(&value));
        let has_source_weight = object.source_weight.is_some_and(|value| value >= 0.0);
        let angle_requires_base = object.role_code.as_deref() == Some("angle");
        let has_angle_base = object
            .angle_priority_base
            .is_some_and(|value| (0.0..=100.0).contains(&value));

        if object.code.trim().is_empty()
            || !has_base_priority
            || !has_source_weight
            || (angle_requires_base && !has_angle_base)
        {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid signal scoring profile for chart object {}",
                object.code
            )));
        }
    }

    Ok(())
}

fn has_required_angle_positions(
    positions: &[crate::domain::ObjectPositionFact],
    references: &CalculationReferenceData,
) -> bool {
    let position_object_ids: HashSet<i32> = positions
        .iter()
        .map(|position| position.chart_object_id)
        .collect();

    references
        .angle_points
        .iter()
        .all(|angle| position_object_ids.contains(&angle.chart_object_id))
}

fn is_stale(row: &ChartCalculationRow, default_stale_after_seconds: i32) -> bool {
    let Some(heartbeat_at) = row.heartbeat_at else {
        return true;
    };
    let threshold = row
        .stale_after_seconds
        .unwrap_or(default_stale_after_seconds)
        .max(1);
    Utc::now().signed_duration_since(heartbeat_at).num_seconds() > i64::from(threshold)
}

pub fn is_current_basic_payload(payload: &BasicPayload) -> bool {
    let structural_axis_pairs = structural_axis_pairs_from_payload(payload);
    let angle_object_codes = angle_object_codes_from_payload(payload);

    !payload.signals.is_empty()
        && payload.signals.len() <= 12
        && has_current_llm_handoff_contract(payload)
        && has_current_angles(payload)
        && has_current_dignities(payload)
        && has_current_chart_emphasis(payload)
        && has_current_reading_plan(payload)
        && has_current_drafting_plan(payload)
        && payload.positions.iter().all(has_current_position_context)
        && payload.signals.iter().all(|signal| {
            signal.evidence.is_some()
                && has_text(&signal.theme_code)
                && has_text(&signal.interpretive_hint)
                && !signal.semantic_tags.is_empty()
                && signal
                    .semantic_tags
                    .iter()
                    .all(|tag| !tag.trim().is_empty())
                && has_text(&signal.aggregation_group)
                && has_text(&signal.writing_guidance)
                && has_current_aspect_hint(&signal.interpretive_hint)
                && has_current_placement_context(signal)
                && has_current_angle_evidence(payload, signal)
                && has_current_aspect_context(signal)
                && !is_structural_axis_aspect_signal(signal, &structural_axis_pairs)
                && !is_angle_to_angle_aspect_signal(signal, &angle_object_codes)
        })
}

fn has_current_llm_handoff_contract(payload: &BasicPayload) -> bool {
    let Some(contract) = payload.llm_handoff_contract.as_ref() else {
        return false;
    };

    contract.contract_version == "basic_natal_structured_v8"
        && contract.payload_language_code == "en"
        && contract.target_language_policy == "provided_by_llm_service"
        && contract.audience_level == "beginner"
        && contract.tone == "clear, warm, non fatalistic"
        && contract.must_use.as_slice()
            == [
            "chart_emphasis",
            "dignities",
            "angles",
            "signals",
                "reading_plan",
                "drafting_plan",
            ]
        && contract.must_not.as_slice()
            == [
                "invent facts not present in source signals",
                "mention technical IDs",
                "list placements mechanically",
                "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group",
                "expose raw evidence unless explicitly requested",
                "treat chart_emphasis as a standalone section instead of weighting context",
                "make deterministic or fatalistic predictions",
            ]
        && contract.output_format == "structured_sections"
}

fn has_current_angles(payload: &BasicPayload) -> bool {
    let angles_by_code: HashMap<&str, &crate::domain::BasicAngleFact> = payload
        .angles
        .iter()
        .map(|angle| (angle.angle_code.as_str(), angle))
        .collect();

    angles_by_code.len() == 4
        && canonical_angle_is_valid(&angles_by_code, "ascendant", "descendant", "horizontal")
        && canonical_angle_is_valid(&angles_by_code, "descendant", "ascendant", "horizontal")
        && canonical_angle_is_valid(&angles_by_code, "mc", "ic", "vertical")
        && canonical_angle_is_valid(&angles_by_code, "ic", "mc", "vertical")
        && payload.angles.iter().all(|angle| {
            !angle.angle_code.trim().is_empty()
                && !angle.angle_name.trim().is_empty()
                && !angle.axis.trim().is_empty()
                && !angle.opposite_angle_code.trim().is_empty()
                && !angle.sign_code.trim().is_empty()
                && !angle.sign_name.trim().is_empty()
                && (1..=12).contains(&angle.house_number)
                && angle.longitude_deg >= 0.0
                && angle.longitude_deg < 360.0
        })
        && payload.signals.iter().any(|signal| {
            signal.signal_key.starts_with("angle:ascendant:sign:")
                && signal
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.get("fact_type"))
                    .and_then(|value| value.as_str())
                    == Some("chart_angle")
        })
}

fn canonical_angle_is_valid(
    angles_by_code: &HashMap<&str, &crate::domain::BasicAngleFact>,
    angle_code: &str,
    opposite_angle_code: &str,
    axis: &str,
) -> bool {
    angles_by_code
        .get(angle_code)
        .is_some_and(|angle| angle.opposite_angle_code == opposite_angle_code && angle.axis == axis)
}

fn has_current_angle_evidence(payload: &BasicPayload, signal: &BasicSignal) -> bool {
    if !signal.signal_key.starts_with("angle:") {
        return true;
    }

    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };
    if evidence.get("fact_type").and_then(|value| value.as_str()) != Some("chart_angle") {
        return false;
    }

    let Some(angle_code) = evidence.get("angle_code").and_then(|value| value.as_str()) else {
        return false;
    };
    let Some(expected_opposite) = payload
        .angles
        .iter()
        .find(|angle| angle.angle_code == angle_code)
        .map(|angle| angle.opposite_angle_code.as_str())
    else {
        return false;
    };

    if evidence
        .get("opposite_angle_code")
        .and_then(|value| value.as_str())
        .is_none_or(|code| code.trim().is_empty())
    {
        return false;
    }

    evidence
        .get("opposite_angle_object_code")
        .and_then(|value| value.as_str())
        == Some(expected_opposite)
}

fn structural_axis_pairs_from_payload(payload: &BasicPayload) -> HashSet<(String, String)> {
    let angle_positions = payload
        .angles
        .iter()
        .map(|angle| (angle.axis.clone(), angle.angle_code.clone()))
        .collect::<Vec<_>>();
    let mut pairs = HashSet::new();

    for left_index in 0..angle_positions.len() {
        for right_index in (left_index + 1)..angle_positions.len() {
            let (left_axis, left_code) = &angle_positions[left_index];
            let (right_axis, right_code) = &angle_positions[right_index];
            if !left_axis.trim().is_empty() && left_axis == right_axis {
                pairs.insert(normalized_pair(left_code, right_code));
            }
        }
    }

    pairs
}

fn angle_object_codes_from_payload(payload: &BasicPayload) -> HashSet<String> {
    payload
        .angles
        .iter()
        .map(|angle| angle.angle_code.clone())
        .collect()
}

fn is_structural_axis_aspect_signal(
    signal: &BasicSignal,
    structural_axis_pairs: &HashSet<(String, String)>,
) -> bool {
    signal.signal_key.starts_with("aspect:")
        && (signal
            .aspect_context
            .as_ref()
            .and_then(|context| context.get("is_structural_axis"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            || signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("is_structural_axis"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false))
        || (signal.signal_key.starts_with("aspect:")
            && aspect_code(signal) == Some("opposition")
            && object_pair_from_aspect_signal(signal)
                .is_some_and(|pair| structural_axis_pairs.contains(&pair)))
}

fn aspect_code(signal: &BasicSignal) -> Option<&str> {
    signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("aspect_code"))
        .and_then(|value| value.as_str())
        .or_else(|| signal.signal_key.split(':').nth(3))
}

fn object_pair_from_aspect_signal(signal: &BasicSignal) -> Option<(String, String)> {
    let evidence_pair = signal.evidence.as_ref().and_then(|evidence| {
        let source = evidence
            .get("source_object_code")
            .and_then(|value| value.as_str())?;
        let target = evidence
            .get("target_object_code")
            .and_then(|value| value.as_str())?;
        Some(normalized_pair(source, target))
    });
    if evidence_pair.is_some() {
        return evidence_pair;
    }

    let parts = signal.signal_key.split(':').collect::<Vec<_>>();
    if parts.len() >= 4 {
        Some(normalized_pair(parts[1], parts[2]))
    } else {
        None
    }
}

fn is_angle_to_angle_aspect_signal(
    signal: &BasicSignal,
    angle_object_codes: &HashSet<String>,
) -> bool {
    signal.signal_key.starts_with("aspect:")
        && object_pair_from_aspect_signal(signal).is_some_and(|(source, target)| {
            angle_object_codes.contains(&source) && angle_object_codes.contains(&target)
        })
}

fn normalized_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn has_current_chart_emphasis(payload: &BasicPayload) -> bool {
    !payload.chart_emphasis.dominant_signs.is_empty()
        && !payload.chart_emphasis.dominant_houses.is_empty()
        && !payload.chart_emphasis.dominant_objects.is_empty()
        && payload.chart_emphasis.dominant_signs.len() <= MAX_DOMINANT_SIGNS
        && payload.chart_emphasis.dominant_houses.len() <= MAX_DOMINANT_HOUSES
        && payload.chart_emphasis.dominant_objects.len() <= MAX_DOMINANT_OBJECTS
        && payload
            .chart_emphasis
            .dominant_signs
            .windows(2)
            .all(|window| window[0].score >= window[1].score)
        && payload
            .chart_emphasis
            .dominant_houses
            .windows(2)
            .all(|window| window[0].score >= window[1].score)
        && payload
            .chart_emphasis
            .dominant_objects
            .windows(2)
            .all(|window| window[0].score >= window[1].score)
        && payload.chart_emphasis.dominant_signs.iter().all(|entry| {
            !entry.sign_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_signs.len() == 1
                    || entry.score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE)
        })
        && payload.chart_emphasis.dominant_houses.iter().all(|entry| {
            (1..=12).contains(&entry.house_number)
                && !entry.theme_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_houses.len() == 1
                    || entry.score >= SIGN_HOUSE_EMPHASIS_MIN_SCORE)
        })
        && payload.chart_emphasis.dominant_objects.iter().all(|entry| {
            !entry.object_code.trim().is_empty()
                && valid_emphasis_score(entry.score)
                && valid_emphasis_reasons(&entry.reasons)
                && (payload.chart_emphasis.dominant_objects.len() == 1
                    || (entry.score >= OBJECT_EMPHASIS_MIN_SCORE
                        && has_non_placement_emphasis_reason(&entry.reasons)))
        })
}

fn valid_emphasis_score(score: f64) -> bool {
    score > 0.0 && score <= 1.0
}

fn valid_emphasis_reasons(reasons: &[String]) -> bool {
    !reasons.is_empty() && reasons.iter().all(|reason| !reason.trim().is_empty())
}

fn has_non_placement_emphasis_reason(reasons: &[String]) -> bool {
    reasons.iter().any(|reason| reason != "placement")
}

fn has_current_aspect_context(signal: &crate::domain::BasicSignal) -> bool {
    if !signal.signal_key.starts_with("aspect:") {
        return true;
    }

    let Some(context) = signal.aspect_context.as_ref() else {
        return false;
    };

    has_text_value(context.get("aspect_family"))
        && context.get("primary_valence").is_some()
        && context.get("intensity_modifier").is_some()
        && context.get("secondary_effect").is_some()
        && has_any_aspect_effect(context)
        && has_text_value(context.get("dynamic_quality"))
        && has_text_value(context.get("phase_state"))
        && has_text_value(context.get("valence_family"))
        && has_bool_value(context.get("is_tonal_valence"))
        && has_bool_value(context.get("is_intensity_modifier"))
        && has_text_value(context.get("writing_guidance"))
}

fn has_any_aspect_effect(context: &serde_json::Value) -> bool {
    ["primary_valence", "intensity_modifier", "secondary_effect"]
        .into_iter()
        .any(|key| has_text_value(context.get(key)))
}

fn has_current_dignities(payload: &BasicPayload) -> bool {
    let all_dignities_are_valid = payload.dignities.iter().all(|dignity| {
        !dignity.object_code.trim().is_empty()
            && !dignity.object_name.trim().is_empty()
            && !dignity.sign_code.trim().is_empty()
            && !dignity.sign_name.trim().is_empty()
            && !dignity.dignity_type.trim().is_empty()
            && !dignity.dignity_label.trim().is_empty()
            && matches!(dignity.polarity.as_str(), "dignity" | "debility")
            && dignity.strength_score > 0.0
            && dignity.signal_key.as_deref().is_none_or(|signal_key| {
                payload.signals.iter().any(|signal| {
                    signal.signal_key == signal_key
                        && signal_matches_structured_dignity(signal, dignity)
                })
            })
    });

    all_dignities_are_valid
        && payload
            .signals
            .iter()
            .filter(|signal| signal.signal_key.starts_with("dignity:"))
            .all(|signal| {
                payload.dignities.iter().any(|dignity| {
                    dignity.signal_key.as_deref() == Some(&signal.signal_key)
                        && signal_matches_structured_dignity(signal, dignity)
                }) && signal
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.get("fact_type"))
                    .and_then(|value| value.as_str())
                    == Some("essential_dignity")
            })
}

fn signal_matches_structured_dignity(
    signal: &crate::domain::BasicSignal,
    dignity: &crate::domain::BasicDignity,
) -> bool {
    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };

    evidence
        .get("chart_object")
        .and_then(|value| value.as_str())
        == Some(dignity.object_code.as_str())
        && evidence.get("sign_code").and_then(|value| value.as_str())
            == Some(dignity.sign_code.as_str())
        && evidence
            .get("dignity_type")
            .and_then(|value| value.as_str())
            == Some(dignity.dignity_type.as_str())
}

fn has_current_reading_plan(payload: &BasicPayload) -> bool {
    if payload.reading_plan.is_empty() {
        return false;
    }

    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let primary_signal_slots: HashMap<&str, &str> = payload
        .reading_plan
        .iter()
        .flat_map(|item| {
            item.source_signal_keys
                .iter()
                .map(move |signal_key| (signal_key.as_str(), item.slot.as_str()))
        })
        .collect();
    let mut slots = HashSet::new();
    let mut primary_signal_keys = HashSet::new();
    let mut previous_slot_order = None;

    payload.reading_plan.iter().all(|item| {
        let slot = item.slot.trim();
        let Some(slot_order) = reading_slot_order(slot) else {
            return false;
        };
        let is_in_order = previous_slot_order.is_none_or(|previous| previous < slot_order);
        previous_slot_order = Some(slot_order);

        !slot.is_empty()
            && slots.insert(slot)
            && is_in_order
            && !item.title.trim().is_empty()
            && !item.source_signal_keys.is_empty()
            && item.primary_signal_keys == item.source_signal_keys
            && item
                .source_signal_keys
                .iter()
                .all(|signal_key| primary_signal_keys.insert(signal_key.as_str()))
            && secondary_candidates_are_valid(item, &signal_keys, &primary_signal_slots)
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
    })
}

fn secondary_candidates_are_valid(
    item: &crate::domain::BasicReadingPlanItem,
    signal_keys: &HashSet<&str>,
    primary_signal_slots: &HashMap<&str, &str>,
) -> bool {
    item.secondary_slot_candidates.iter().all(|candidate| {
        !candidate.signal_key.trim().is_empty()
            && signal_keys.contains(candidate.signal_key.as_str())
            && primary_signal_slots
                .get(candidate.signal_key.as_str())
                .is_some_and(|primary_slot| *primary_slot == candidate.primary_slot)
            && candidate.candidate_slot == item.slot
            && !item.source_signal_keys.contains(&candidate.signal_key)
    })
}

fn reading_slot_order(slot: &str) -> Option<usize> {
    match slot {
        "core_identity" => Some(0),
        "dominant_cluster" => Some(1),
        "main_tension_or_support" => Some(2),
        "expression_style" => Some(3),
        "background_factors" => Some(4),
        _ => None,
    }
}

fn has_current_drafting_plan(payload: &BasicPayload) -> bool {
    if payload.drafting_plan.is_empty() || payload.drafting_plan.len() != payload.reading_plan.len()
    {
        return false;
    }

    let reading_sources_by_slot: HashMap<&str, &[String]> = payload
        .reading_plan
        .iter()
        .map(|item| (item.slot.as_str(), item.source_signal_keys.as_slice()))
        .collect();
    let reading_items_by_slot: HashMap<&str, &crate::domain::BasicReadingPlanItem> = payload
        .reading_plan
        .iter()
        .map(|item| (item.slot.as_str(), item))
        .collect();
    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let mut slots = HashSet::new();
    let has_dominant_cluster = payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "dominant_cluster");

    payload.drafting_plan.iter().all(|item| {
        let slot = item.slot.trim();
        !slot.is_empty()
            && slots.insert(slot)
            && reading_sources_by_slot
                .get(slot)
                .is_some_and(|reading_sources| {
                    *reading_sources == item.source_signal_keys.as_slice()
                })
            && reading_items_by_slot.get(slot).is_some_and(|reading_item| {
                reading_item.primary_signal_keys == item.primary_signal_keys
                    && reading_item.secondary_slot_candidates == item.secondary_slot_candidates
            })
            && reading_items_by_slot.get(slot).is_some_and(|reading_item| {
                item.emphasis_refs
                    == expected_emphasis_refs_for_slot(reading_item, payload, has_dominant_cluster)
            })
            && !item.section_title.trim().is_empty()
            && !item.writing_objective.trim().is_empty()
            && has_current_drafting_language(item)
            && item.max_words > 0
            && !item.avoid.is_empty()
            && item.avoid.iter().all(|rule| !rule.trim().is_empty())
            && item
                .avoid
                .contains(&"turn chart_emphasis into a standalone section".to_string())
            && !item.source_signal_keys.is_empty()
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
    })
}

fn expected_emphasis_refs_for_slot(
    item: &crate::domain::BasicReadingPlanItem,
    payload: &BasicPayload,
    has_dominant_cluster: bool,
) -> crate::domain::BasicEmphasisRefs {
    let should_attach =
        item.slot == "dominant_cluster" || (item.slot == "core_identity" && !has_dominant_cluster);
    if !should_attach {
        return crate::domain::BasicEmphasisRefs::default();
    }

    let (dominant_signs, dominant_houses) = if item.slot == "dominant_cluster" {
        let cluster_signs = cluster_sign_refs(item, payload);
        let cluster_houses = cluster_house_refs(item, payload);
        (
            filtered_or_all_sign_refs(payload, &cluster_signs),
            filtered_or_all_house_refs(payload, &cluster_houses),
        )
    } else {
        (
            payload
                .chart_emphasis
                .dominant_signs
                .iter()
                .map(|entry| entry.sign_code.clone())
                .collect(),
            payload
                .chart_emphasis
                .dominant_houses
                .iter()
                .map(|entry| entry.house_number)
                .collect(),
        )
    };

    let slot_objects = emphasis_object_scope(item);
    let dominant_objects = if slot_objects.is_empty() {
        payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .map(|entry| entry.object_code.clone())
            .collect()
    } else {
        payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .filter(|entry| slot_objects.contains(&entry.object_code))
            .map(|entry| entry.object_code.clone())
            .collect()
    };

    crate::domain::BasicEmphasisRefs {
        dominant_signs,
        dominant_houses,
        dominant_objects,
    }
}

fn cluster_sign_refs(
    item: &crate::domain::BasicReadingPlanItem,
    payload: &BasicPayload,
) -> Vec<String> {
    signals_for_plan_item(item, payload)
        .into_iter()
        .filter(|signal| signal.signal_key.starts_with("cluster:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("sign_code"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn cluster_house_refs(
    item: &crate::domain::BasicReadingPlanItem,
    payload: &BasicPayload,
) -> Vec<i32> {
    signals_for_plan_item(item, payload)
        .into_iter()
        .filter(|signal| signal.signal_key.starts_with("cluster:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("house_number"))
                .and_then(|value| value.as_i64())
                .and_then(|value| i32::try_from(value).ok())
        })
        .collect()
}

fn signals_for_plan_item<'a>(
    item: &crate::domain::BasicReadingPlanItem,
    payload: &'a BasicPayload,
) -> Vec<&'a crate::domain::BasicSignal> {
    item.source_signal_keys
        .iter()
        .filter_map(|key| {
            payload
                .signals
                .iter()
                .find(|signal| signal.signal_key == *key)
        })
        .collect()
}

fn filtered_or_all_sign_refs(payload: &BasicPayload, allowed_signs: &[String]) -> Vec<String> {
    let refs = payload
        .chart_emphasis
        .dominant_signs
        .iter()
        .filter(|entry| allowed_signs.contains(&entry.sign_code))
        .map(|entry| entry.sign_code.clone())
        .collect::<Vec<_>>();
    if refs.is_empty() {
        payload
            .chart_emphasis
            .dominant_signs
            .iter()
            .map(|entry| entry.sign_code.clone())
            .collect()
    } else {
        refs
    }
}

fn filtered_or_all_house_refs(payload: &BasicPayload, allowed_houses: &[i32]) -> Vec<i32> {
    let refs = payload
        .chart_emphasis
        .dominant_houses
        .iter()
        .filter(|entry| allowed_houses.contains(&entry.house_number))
        .map(|entry| entry.house_number)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        payload
            .chart_emphasis
            .dominant_houses
            .iter()
            .map(|entry| entry.house_number)
            .collect()
    } else {
        refs
    }
}

fn emphasis_object_scope(item: &crate::domain::BasicReadingPlanItem) -> Vec<String> {
    let mut object_codes = Vec::new();
    for signal_key in item.source_signal_keys.iter().chain(
        item.secondary_slot_candidates
            .iter()
            .map(|candidate| &candidate.signal_key),
    ) {
        if let Some(object_code) = object_code_from_signal_key(signal_key) {
            if !object_codes.contains(&object_code) {
                object_codes.push(object_code);
            }
        }
    }
    object_codes
}

fn object_code_from_signal_key(signal_key: &str) -> Option<String> {
    if let Some(object_code) = signal_key.strip_prefix("object_position:") {
        return Some(object_code.to_string());
    }
    signal_key
        .strip_prefix("dignity:")
        .and_then(|tail| tail.split(':').next())
        .filter(|object_code| !object_code.is_empty())
        .map(ToString::to_string)
}

fn has_current_drafting_language(item: &crate::domain::BasicDraftingPlanItem) -> bool {
    let fields = std::iter::once(item.section_title.as_str())
        .chain(std::iter::once(item.writing_objective.as_str()))
        .chain(item.avoid.iter().map(String::as_str));

    fields
        .into_iter()
        .all(|field| !contains_non_ascii_letter(field))
}

fn contains_non_ascii_letter(text: &str) -> bool {
    text.chars()
        .any(|character| character.is_alphabetic() && !character.is_ascii())
}

fn has_text(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

fn has_current_aspect_hint(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| {
        !text.contains(" by a opposition") && !text.contains(" are connected by ")
    })
}

fn has_current_position_context(position: &crate::domain::BasicObjectPosition) -> bool {
    let is_angle = position
        .object_context
        .as_ref()
        .and_then(|context| context.get("role"))
        .and_then(|value| value.as_str())
        == Some("angle")
        || position
            .object_context
            .as_ref()
            .and_then(|context| context.get("role_label"))
            .and_then(|value| value.as_str())
            == Some("Angle");

    !position.sign_code.is_empty()
        && !position.sign_name.is_empty()
        && position.dignity_context.is_array()
        && option_json_has_text(&position.sign_context, "element")
        && option_json_has_text(&position.sign_context, "modality")
        && option_json_has_text(&position.sign_context, "polarity")
        && option_json_has_text(&position.house_context, "theme_code")
        && option_json_has_text(&position.house_modality, "code")
        && option_json_has_text(&position.object_context, "role")
        && (is_angle || option_json_has_text(&position.motion_context, "motion_state"))
}

fn has_current_placement_context(signal: &crate::domain::BasicSignal) -> bool {
    if !signal.signal_key.starts_with("object_position:") {
        return true;
    }

    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };

    let Some(context) = evidence.get("placement_context") else {
        return false;
    };

    evidence
        .get("essential_dignities")
        .is_some_and(|value| value.is_array())
        && nested_json_has_text(context, "sign_context", "element")
        && nested_json_has_text(context, "sign_context", "modality")
        && nested_json_has_text(context, "sign_context", "polarity")
        && nested_json_has_text(context, "house_context", "theme_code")
        && nested_json_has_text(context, "house_modality", "code")
        && nested_json_has_text(context, "object_context", "role")
        && nested_json_has_text(context, "motion_context", "motion_state")
}

fn option_json_has_text(value: &Option<serde_json::Value>, key: &str) -> bool {
    value
        .as_ref()
        .and_then(|value| value.get(key))
        .is_some_and(json_value_has_text)
}

fn nested_json_has_text(value: &serde_json::Value, context_key: &str, key: &str) -> bool {
    value
        .get(context_key)
        .and_then(|context| context.get(key))
        .is_some_and(json_value_has_text)
}

fn has_text_value(value: Option<&serde_json::Value>) -> bool {
    value.is_some_and(json_value_has_text)
}

fn has_bool_value(value: Option<&serde_json::Value>) -> bool {
    value.is_some_and(serde_json::Value::is_boolean)
}

fn json_value_has_text(value: &serde_json::Value) -> bool {
    value.as_str().is_some_and(|text| !text.trim().is_empty())
}

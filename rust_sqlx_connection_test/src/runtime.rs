use std::collections::{HashMap, HashSet};

use chrono::Utc;
use sqlx::PgPool;
use thiserror::Error;

use crate::domain::{
    BasicPayload, CalculatedChartFacts, CalculationReferenceData, NatalChartInput, RuntimeOptions,
};
use crate::ephemeris::EphemerisEngine;
use crate::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use crate::models::ChartCalculationRow;
use crate::payload::build_basic_payload;
use crate::repositories::RuntimeRepository;
use crate::signals::aggregate_basic_signals;

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

        let chart_objects = self.repository.active_chart_objects().await?;
        let aspect_definitions = self.repository.aspect_definitions().await?;
        let house_system = self.repository.house_system(input.house_system_id).await?;
        let references = CalculationReferenceData {
            signs: self.repository.sign_references().await?,
            houses: self.repository.house_references().await?,
            motion_states: self.repository.motion_state_references().await?,
        };
        validate_calculation_references(&references)?;

        let mut tx = self.repository.pool().begin().await?;
        RuntimeRepository::lock_idempotency(&mut tx, lock_key).await?;

        let existing = RuntimeRepository::calculations_for_key(&mut tx, &idempotency_key).await?;
        if let Some(completed) = existing.iter().find(|row| row.status == "completed") {
            let completed_id = completed.id;
            tx.commit().await?;
            if let Some(payload) = self
                .repository
                .existing_basic_payload(completed_id, &product_code, Some(payload_language_id))
                .await?
            {
                if is_current_basic_payload(&payload) {
                    return Ok(payload);
                }
            }
            let positions = self.repository.positions_for_payload(completed_id).await?;
            let aspects = self.repository.aspects_for_payload(completed_id).await?;
            let signal_drafts = aggregate_basic_signals(&CalculatedChartFacts {
                positions: positions.clone(),
                house_cusps: Vec::new(),
                aspects,
            });
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

fn validate_calculation_references(
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
        {
            return Err(RuntimeError::Ephemeris(
                "invalid house references: duplicate IDs, invalid numbers, or empty labels"
                    .to_string(),
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

    Ok(())
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

fn is_current_basic_payload(payload: &BasicPayload) -> bool {
    !payload.signals.is_empty()
        && payload.signals.len() <= 12
        && has_current_llm_handoff_contract(payload)
        && has_current_dignities(payload)
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
                && has_current_aspect_context(signal)
        })
}

fn has_current_llm_handoff_contract(payload: &BasicPayload) -> bool {
    let Some(contract) = payload.llm_handoff_contract.as_ref() else {
        return false;
    };

    contract.contract_version == "basic_natal_structured_v4"
        && contract.payload_language_code == "en"
        && contract.target_language_policy == "provided_by_llm_service"
        && contract.audience_level == "beginner"
        && contract.tone == "clear, warm, non fatalistic"
        && contract.must_use.as_slice() == ["dignities", "signals", "reading_plan", "drafting_plan"]
        && contract.must_not.as_slice()
            == [
                "invent facts not present in source signals",
                "mention technical IDs",
                "list placements mechanically",
                "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group",
                "expose raw evidence unless explicitly requested",
                "make deterministic or fatalistic predictions",
            ]
        && contract.output_format == "structured_sections"
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
            && !item.section_title.trim().is_empty()
            && !item.writing_objective.trim().is_empty()
            && has_current_drafting_language(item)
            && item.max_words > 0
            && !item.avoid.is_empty()
            && item.avoid.iter().all(|rule| !rule.trim().is_empty())
            && !item.source_signal_keys.is_empty()
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
    })
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
    !position.sign_code.is_empty()
        && !position.sign_name.is_empty()
        && position.dignity_context.is_array()
        && option_json_has_text(&position.sign_context, "element")
        && option_json_has_text(&position.sign_context, "modality")
        && option_json_has_text(&position.sign_context, "polarity")
        && option_json_has_text(&position.house_modality, "code")
        && option_json_has_text(&position.object_context, "role")
        && option_json_has_text(&position.motion_context, "motion_state")
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::domain::{
        BasicDignity, BasicDraftingPlanItem, BasicLlmHandoffContract, BasicObjectPosition,
        BasicReadingPlanItem, BasicSecondarySlotCandidate, BasicSignal,
    };
    use crate::models::{HouseReference, SignReference};

    fn current_payload() -> BasicPayload {
        BasicPayload {
            product_code: "basic".to_string(),
            chart_calculation_id: 1,
            reference_version_id: 1,
            subject_label: None,
            birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            llm_handoff_contract: Some(BasicLlmHandoffContract {
                contract_version: "basic_natal_structured_v4".to_string(),
                payload_language_code: "en".to_string(),
                target_language_policy: "provided_by_llm_service".to_string(),
                audience_level: "beginner".to_string(),
                tone: "clear, warm, non fatalistic".to_string(),
                must_use: vec![
                    "dignities".to_string(),
                    "signals".to_string(),
                    "reading_plan".to_string(),
                    "drafting_plan".to_string(),
                ],
                must_not: vec![
                    "invent facts not present in source signals".to_string(),
                    "mention technical IDs".to_string(),
                    "list placements mechanically".to_string(),
                    "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group".to_string(),
                    "expose raw evidence unless explicitly requested".to_string(),
                    "make deterministic or fatalistic predictions".to_string(),
                ],
                output_format: "structured_sections".to_string(),
            }),
            positions: vec![BasicObjectPosition {
                object_code: "sun".to_string(),
                object_name: "Sun".to_string(),
                longitude_deg: 84.0,
                sign_id: 3,
                sign_code: "gemini".to_string(),
                sign_name: "Gemini".to_string(),
                house_id: Some(9),
                house_number: Some(9),
                house_name: Some("Beliefs".to_string()),
                motion_state_id: Some(1),
                sign_context: Some(json!({
                    "element": "air",
                    "modality": "mutable",
                    "polarity": "yang",
                    "keywords": ["communication"]
                })),
                house_modality: Some(json!({
                    "code": "cadent",
                    "accidental_strength": "weak_or_background",
                    "interpretation_weight": "lower_for_external_manifestation"
                })),
                object_context: Some(json!({
                    "role": "luminary",
                    "nature": ["luminary"],
                    "is_luminary": true
                })),
                motion_context: Some(json!({
                    "motion_state": "direct",
                    "label": "Direct",
                    "motion_family": "forward"
                })),
                dignity_context: json!([]),
            }],
            dignities: Vec::new(),
            signals: vec![BasicSignal {
                signal_key: "object_position:sun".to_string(),
                theme_code: Some("beliefs".to_string()),
                title: "Sun in Gemini, house 9".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 100.0,
                confidence_score: Some(0.95),
                interpretive_hint: Some("hint".to_string()),
                semantic_tags: vec!["placement".to_string()],
                source_weight: Some(1.0),
                aggregation_group: Some("gemini:house_9".to_string()),
                writing_guidance: Some("guidance".to_string()),
                aspect_context: None,
                evidence: Some(json!({
                    "fact_type": "object_position",
                    "essential_dignities": [],
                    "placement_context": {
                        "sign_context": {
                            "element": "air",
                            "modality": "mutable",
                            "polarity": "yang"
                        },
                        "house_modality": {"code": "cadent"},
                        "object_context": {"role": "luminary"},
                        "motion_context": {"motion_state": "direct"}
                    }
                })),
            }],
            reading_plan: vec![BasicReadingPlanItem {
                slot: "core_identity".to_string(),
                title: "Core identity markers".to_string(),
                source_signal_keys: vec!["object_position:sun".to_string()],
                primary_signal_keys: vec!["object_position:sun".to_string()],
                secondary_slot_candidates: Vec::new(),
            }],
            drafting_plan: vec![BasicDraftingPlanItem {
                slot: "core_identity".to_string(),
                section_title: "Core chart markers".to_string(),
                source_signal_keys: vec!["object_position:sun".to_string()],
                primary_signal_keys: vec!["object_position:sun".to_string()],
                secondary_slot_candidates: Vec::new(),
                writing_objective: "Explain the central markers.".to_string(),
                max_words: 110,
                avoid: vec!["use technical IDs".to_string()],
            }],
        }
    }

    #[test]
    fn current_payload_requires_canonical_llm_handoff_contract() {
        let mut payload = current_payload();
        payload
            .llm_handoff_contract
            .as_mut()
            .expect("llm handoff contract")
            .payload_language_code = "fr".to_string();

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_requires_llm_handoff_contract() {
        let mut payload = current_payload();
        payload.llm_handoff_contract = None;

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_requires_signals() {
        let mut payload = current_payload();
        payload.signals.clear();

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_empty_semantic_contract_fields() {
        let mut payload = current_payload();
        payload.signals[0].interpretive_hint = Some(" ".to_string());

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_aspect_context_without_reference_effect() {
        let mut payload = current_payload();
        payload.signals.push(BasicSignal {
            signal_key: "aspect:sun:mercury:conjunction".to_string(),
            theme_code: Some("aspect".to_string()),
            title: "Sun conjunction Mercury".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 70.0,
            confidence_score: Some(0.85),
            interpretive_hint: Some("Sun and Mercury are connected by a conjunction.".to_string()),
            semantic_tags: vec![
                "aspect".to_string(),
                "conjunction".to_string(),
                "contextual".to_string(),
            ],
            source_weight: Some(1.75),
            aggregation_group: Some("aspect:conjunction".to_string()),
            writing_guidance: Some(
                "Use the aspect as a relationship between two chart factors.".to_string(),
            ),
            aspect_context: Some(json!({
                "aspect_family": "major",
                "primary_valence": null,
                "intensity_modifier": null,
                "secondary_effect": null,
                "dynamic_quality": "contextual",
                "phase_state": "separating",
                "writing_guidance": "Use the aspect as a relationship between two chart factors."
            })),
            evidence: Some(json!({
                "fact_type": "aspect",
                "aspect_code": "conjunction",
                "strength_score": 0.875
            })),
        });

        assert!(!is_current_basic_payload(&payload));

        payload
            .signals
            .last_mut()
            .expect("aspect signal")
            .aspect_context = Some(json!({
            "aspect_family": "major",
            "primary_valence": null,
            "intensity_modifier": "amplifying",
            "secondary_effect": null,
            "dynamic_quality": "intensification",
            "phase_state": "separating",
            "valence_family": "intensity",
            "is_tonal_valence": false,
            "is_intensity_modifier": true,
            "writing_guidance": "Treat amplifying as an intensity modifier."
        }));

        assert!(!is_current_basic_payload(&payload));

        payload
            .signals
            .last_mut()
            .expect("aspect signal")
            .interpretive_hint = Some(
            "Read this conjunction as an amplifying contact between Sun and Mercury, with attention to the separating phase."
                .to_string(),
        );

        assert!(is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_incomplete_placement_context() {
        let mut payload = current_payload();
        payload.positions[0].sign_context = Some(json!({
            "element": "air",
            "modality": "mutable"
        }));

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_non_array_dignity_context() {
        let mut payload = current_payload();
        payload.positions[0].dignity_context = serde_json::Value::Null;

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_incomplete_signal_placement_context() {
        let mut payload = current_payload();
        payload.signals[0].evidence = Some(json!({
            "fact_type": "object_position",
            "essential_dignities": [],
            "placement_context": {
                "sign_context": {
                    "element": "air",
                    "modality": "mutable"
                },
                "house_modality": {"code": "cadent"},
                "object_context": {"role": "luminary"},
                "motion_context": {"motion_state": "direct"}
            }
        }));

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_placement_signal_without_dignity_array() {
        let mut payload = current_payload();
        payload.signals[0].evidence = Some(json!({
            "fact_type": "object_position",
            "placement_context": {
                "sign_context": {
                    "element": "air",
                    "modality": "mutable",
                    "polarity": "yang"
                },
                "house_modality": {"code": "cadent"},
                "object_context": {"role": "luminary"},
                "motion_context": {"motion_state": "direct"}
            }
        }));

        assert!(!is_current_basic_payload(&payload));

        payload.signals[0].evidence = Some(json!({
            "fact_type": "object_position",
            "essential_dignities": [],
            "placement_context": {
                "sign_context": {
                    "element": "air",
                    "modality": "mutable",
                    "polarity": "yang"
                },
                "house_modality": {"code": "cadent"},
                "object_context": {"role": "luminary"},
                "motion_context": {"motion_state": "direct"}
            }
        }));

        assert!(is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_dignity_signal_without_structured_dignity() {
        let mut payload = current_payload();
        payload.signals.push(BasicSignal {
            signal_key: "dignity:saturn:domicile:capricorn".to_string(),
            theme_code: Some("functional_strength".to_string()),
            title: "Saturn strongly placed in Capricorn".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 88.0,
            confidence_score: Some(0.95),
            interpretive_hint: Some("hint".to_string()),
            semantic_tags: vec!["dignity".to_string(), "saturn".to_string()],
            source_weight: Some(0.75),
            aggregation_group: Some("dignity:saturn".to_string()),
            writing_guidance: Some("guidance".to_string()),
            aspect_context: None,
            evidence: Some(json!({
                "fact_type": "essential_dignity",
                "chart_object": "saturn",
                "sign_code": "capricorn",
                "dignity_type": "domicile"
            })),
        });

        assert!(!is_current_basic_payload(&payload));

        payload.dignities.push(BasicDignity {
            object_code: "saturn".to_string(),
            object_name: "Saturn".to_string(),
            sign_id: 10,
            sign_code: "capricorn".to_string(),
            sign_name: "Capricorn".to_string(),
            dignity_type: "domicile".to_string(),
            dignity_label: "Domicile".to_string(),
            polarity: "dignity".to_string(),
            strength_score: 1.0,
            signal_key: Some("dignity:saturn:domicile:capricorn".to_string()),
        });

        assert!(is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_dignity_signal_mismatched_with_structured_dignity() {
        let mut payload = current_payload();
        payload.signals.push(BasicSignal {
            signal_key: "dignity:saturn:domicile:capricorn".to_string(),
            theme_code: Some("functional_strength".to_string()),
            title: "Saturn strongly placed in Capricorn".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 88.0,
            confidence_score: Some(0.95),
            interpretive_hint: Some("hint".to_string()),
            semantic_tags: vec!["dignity".to_string(), "saturn".to_string()],
            source_weight: Some(0.75),
            aggregation_group: Some("dignity:saturn".to_string()),
            writing_guidance: Some("guidance".to_string()),
            aspect_context: None,
            evidence: Some(json!({
                "fact_type": "essential_dignity",
                "chart_object": "jupiter",
                "sign_code": "cancer",
                "dignity_type": "exaltation"
            })),
        });
        payload.dignities.push(BasicDignity {
            object_code: "saturn".to_string(),
            object_name: "Saturn".to_string(),
            sign_id: 10,
            sign_code: "capricorn".to_string(),
            sign_name: "Capricorn".to_string(),
            dignity_type: "domicile".to_string(),
            dignity_label: "Domicile".to_string(),
            polarity: "dignity".to_string(),
            strength_score: 1.0,
            signal_key: Some("dignity:saturn:domicile:capricorn".to_string()),
        });

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_requires_reading_plan() {
        let mut payload = current_payload();
        payload.reading_plan.clear();

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_requires_drafting_plan() {
        let mut payload = current_payload();
        payload.drafting_plan.clear();

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_reading_plan_with_missing_signal_key() {
        let mut payload = current_payload();
        payload.reading_plan[0]
            .source_signal_keys
            .push("object_position:moon".to_string());

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_repeated_primary_source_signal() {
        let mut payload = current_payload();
        payload.reading_plan.push(BasicReadingPlanItem {
            slot: "dominant_cluster".to_string(),
            title: "Dominant cluster".to_string(),
            source_signal_keys: vec!["object_position:sun".to_string()],
            primary_signal_keys: vec!["object_position:sun".to_string()],
            secondary_slot_candidates: Vec::new(),
        });
        payload.drafting_plan.push(BasicDraftingPlanItem {
            slot: "dominant_cluster".to_string(),
            section_title: "Dominant cluster".to_string(),
            source_signal_keys: vec!["object_position:sun".to_string()],
            primary_signal_keys: vec!["object_position:sun".to_string()],
            secondary_slot_candidates: Vec::new(),
            writing_objective: "Explain the repeated primary signal.".to_string(),
            max_words: 120,
            avoid: vec!["repeat".to_string()],
        });

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_secondary_candidate_without_primary_source() {
        let mut payload = current_payload();
        payload.signals.push(BasicSignal {
            signal_key: "object_position:moon".to_string(),
            theme_code: Some("emotional_style".to_string()),
            title: "Moon in Pisces, house 4".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 90.0,
            confidence_score: Some(0.95),
            interpretive_hint: Some("hint".to_string()),
            semantic_tags: vec!["placement".to_string()],
            source_weight: Some(0.75),
            aggregation_group: Some("pisces:house_4".to_string()),
            writing_guidance: Some("guidance".to_string()),
            aspect_context: None,
            evidence: Some(json!({
                "fact_type": "object_position",
                "object_code": "moon",
                "placement_context": {
                    "sign_context": {
                        "element": "water",
                        "modality": "mutable",
                        "polarity": "yin"
                    },
                    "house_modality": {"code": "angular"},
                    "object_context": {"role": "luminary"},
                    "motion_context": {"motion_state": "direct"}
                },
                "essential_dignities": []
            })),
        });

        let candidate = BasicSecondarySlotCandidate {
            signal_key: "object_position:moon".to_string(),
            primary_slot: "dominant_cluster".to_string(),
            candidate_slot: "core_identity".to_string(),
        };
        payload.reading_plan[0]
            .secondary_slot_candidates
            .push(candidate.clone());
        payload.drafting_plan[0]
            .secondary_slot_candidates
            .push(candidate);

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_duplicate_reading_plan_slots() {
        let mut payload = current_payload();
        payload.reading_plan.push(BasicReadingPlanItem {
            slot: "core_identity".to_string(),
            title: "Duplicate".to_string(),
            source_signal_keys: vec!["object_position:sun".to_string()],
            primary_signal_keys: vec!["object_position:sun".to_string()],
            secondary_slot_candidates: Vec::new(),
        });

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_unknown_reading_plan_slot() {
        let mut payload = current_payload();
        payload.reading_plan[0].slot = "custom_slot".to_string();
        payload.drafting_plan[0].slot = "custom_slot".to_string();

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_out_of_order_reading_plan_slots() {
        let mut payload = current_payload();
        payload.signals.push(BasicSignal {
            signal_key: "object_position:mercury".to_string(),
            theme_code: Some("communication".to_string()),
            title: "Mercury in Gemini, house 9".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 85.0,
            confidence_score: Some(0.95),
            interpretive_hint: Some("hint".to_string()),
            semantic_tags: vec!["placement".to_string()],
            source_weight: Some(0.75),
            aggregation_group: Some("gemini:house_9".to_string()),
            writing_guidance: Some("guidance".to_string()),
            aspect_context: None,
            evidence: Some(json!({"fact_type": "object_position"})),
        });
        payload.reading_plan.insert(
            0,
            BasicReadingPlanItem {
                slot: "expression_style".to_string(),
                title: "Expression style".to_string(),
                source_signal_keys: vec!["object_position:mercury".to_string()],
                primary_signal_keys: vec!["object_position:mercury".to_string()],
                secondary_slot_candidates: Vec::new(),
            },
        );
        payload.drafting_plan.insert(
            0,
            BasicDraftingPlanItem {
                slot: "expression_style".to_string(),
                section_title: "Expression and action style".to_string(),
                source_signal_keys: vec!["object_position:mercury".to_string()],
                primary_signal_keys: vec!["object_position:mercury".to_string()],
                secondary_slot_candidates: Vec::new(),
                writing_objective: "Show how the person thinks and acts.".to_string(),
                max_words: 110,
                avoid: vec!["use technical IDs".to_string()],
            },
        );

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_drafting_plan_with_missing_signal_key() {
        let mut payload = current_payload();
        payload.drafting_plan[0]
            .source_signal_keys
            .push("object_position:moon".to_string());

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_rejects_old_opposition_hint_template() {
        let mut payload = current_payload();
        payload.signals[0].interpretive_hint = Some(
            "Jupiter and Uranus are connected by a opposition, so their functions should be read together."
                .to_string(),
        );

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn reference_validation_requires_twelve_signs() {
        let mut references = reference_data();
        references.signs.pop();

        assert!(validate_calculation_references(&references).is_err());
    }

    #[test]
    fn reference_validation_rejects_duplicate_house_numbers() {
        let mut references = reference_data();
        references.houses[1].number = 1;

        assert!(validate_calculation_references(&references).is_err());
    }

    fn reference_data() -> CalculationReferenceData {
        CalculationReferenceData {
            signs: (1..=12)
                .map(|id| SignReference {
                    id,
                    code: format!("sign_{id}"),
                    name: format!("Sign {id}"),
                    element_code: Some("earth".to_string()),
                    element_label: Some("Earth".to_string()),
                    modality_code: Some("cardinal".to_string()),
                    modality_name: Some("Cardinal".to_string()),
                    polarity_code: Some("yin".to_string()),
                    polarity_name: Some("Yin".to_string()),
                    keywords_json: Some(json!(["structure"])),
                    shadow_keywords_json: None,
                })
                .collect(),
            houses: (1..=12)
                .map(|number| HouseReference {
                    id: number + 100,
                    number,
                    name: format!("House {number}"),
                    modality_code: Some("angular".to_string()),
                    modality_label: Some("Angular".to_string()),
                    accidental_strength: Some("strong".to_string()),
                    interpretation_weight: Some("high".to_string()),
                })
                .collect(),
            motion_states: vec![crate::models::MotionStateReference {
                id: 1,
                code: "direct".to_string(),
                label: "Direct".to_string(),
                motion_family: "forward".to_string(),
            }],
        }
    }
}

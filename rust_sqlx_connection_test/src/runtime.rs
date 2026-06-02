use std::collections::{HashMap, HashSet};

use chrono::Utc;
use sqlx::PgPool;
use thiserror::Error;

use crate::domain::{
    BasicControlledGenerationOutput, BasicGeneratedReadingPayload, BasicPayload,
    CalculatedChartFacts, CalculationReferenceData, NatalChartInput, RuntimeOptions,
};
use crate::ephemeris::EphemerisEngine;
use crate::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use crate::payload::{
    build_basic_payload, build_fake_generated_reading, is_valid_fake_generated_reading,
};
use crate::repositories::{ChartCalculationRow, RuntimeRepository};
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
    #[error("invalid generated reading payload: {0}")]
    InvalidGeneratedPayload(String),
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
            Self::InvalidGeneratedPayload(_) => "invalid_generated_payload",
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
        let input_hash = input_hash(&input)?;
        let idempotency_key = idempotency_key(&input, &self.options)?;
        let lock_key = advisory_lock_key(&idempotency_key);

        let chart_objects = self.repository.active_chart_objects().await?;
        let aspect_definitions = self.repository.aspect_definitions().await?;
        let house_system = self.repository.house_system(input.house_system_id).await?;
        let references = CalculationReferenceData {
            signs: self.repository.sign_references().await?,
            houses: self.repository.house_references().await?,
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
                .existing_basic_payload(completed_id, &product_code, input.language_id)
                .await?
            {
                if is_current_basic_payload(&payload) {
                    let generated_payload = build_validated_fake_generated_reading(&payload)?;
                    let mut generated_tx = self.repository.pool().begin().await?;
                    RuntimeRepository::persist_generated_reading_payload(
                        &mut generated_tx,
                        input.language_id,
                        &generated_payload,
                    )
                    .await?;
                    generated_tx.commit().await?;
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
            RuntimeRepository::persist_basic_payload(&mut payload_tx, &input, &payload).await?;
            let generated_payload = build_validated_fake_generated_reading(&payload)?;
            RuntimeRepository::persist_generated_reading_payload(
                &mut payload_tx,
                input.language_id,
                &generated_payload,
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

        let result = (|| {
            let facts = self.ephemeris.calculate_natal(
                &input,
                &chart_objects,
                &aspect_definitions,
                &house_system,
                &references,
            )?;
            let signal_drafts = aggregate_basic_signals(&facts);
            Ok((facts, signal_drafts))
        })();

        let (facts, signal_drafts) = match result {
            Ok(value) => value,
            Err(error) => {
                RuntimeRepository::mark_failed(&mut tx, chart_calculation_id, &error).await?;
                tx.commit().await?;
                return Err(error);
            }
        };

        RuntimeRepository::persist_facts(&mut tx, chart_calculation_id, &facts).await?;
        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "aggregating_signals").await?;
        let signal_rows = RuntimeRepository::persist_signals(
            &mut tx,
            chart_calculation_id,
            input.reference_version_id,
            &signal_drafts,
        )
        .await?;

        RuntimeRepository::heartbeat(&mut tx, chart_calculation_id, "building_payload").await?;
        let payload =
            build_basic_payload(chart_calculation_id, &input, &facts.positions, &signal_rows);
        RuntimeRepository::persist_basic_payload(&mut tx, &input, &payload).await?;
        let generated_payload = build_validated_fake_generated_reading(&payload)?;
        RuntimeRepository::persist_generated_reading_payload(
            &mut tx,
            input.language_id,
            &generated_payload,
        )
        .await?;
        RuntimeRepository::mark_completed(&mut tx, chart_calculation_id).await?;
        tx.commit().await?;

        Ok(payload)
    }

    pub async fn calculate_natal_basic_with_fake_generation(
        &self,
        input: NatalChartInput,
    ) -> Result<BasicControlledGenerationOutput, RuntimeError> {
        let source_payload = self.calculate_natal_basic(input).await?;
        let generated_payload = build_validated_fake_generated_reading(&source_payload)?;

        Ok(BasicControlledGenerationOutput {
            source_payload,
            generated_payload,
        })
    }
}

fn build_validated_fake_generated_reading(
    payload: &BasicPayload,
) -> Result<BasicGeneratedReadingPayload, RuntimeError> {
    let generated_payload = build_fake_generated_reading(payload);
    if is_valid_fake_generated_reading(payload, &generated_payload) {
        Ok(generated_payload)
    } else {
        Err(RuntimeError::InvalidGeneratedPayload(
            "fake provider output does not satisfy the Basic generation contract".to_string(),
        ))
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
        && has_current_writing_contract(payload)
        && has_current_reading_plan(payload)
        && has_current_drafting_plan(payload)
        && payload
            .positions
            .iter()
            .all(|position| !position.sign_code.is_empty() && !position.sign_name.is_empty())
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
                && has_current_aspect_article(&signal.interpretive_hint)
        })
}

fn has_current_writing_contract(payload: &BasicPayload) -> bool {
    let Some(contract) = payload.writing_contract.as_ref() else {
        return false;
    };

    contract.audience_level == "beginner"
        && contract.tone == "clear, warm, non fatalistic"
        && contract.language == "fr"
        && contract.max_total_words == 650
        && contract.must_not.as_slice()
            == [
                "list placements mechanically",
                "mention internal IDs",
                "invent facts not present in source signals",
                "use deterministic or fatalistic wording",
            ]
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
    let mut slots = HashSet::new();
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
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
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

    fields.into_iter().all(|field| {
        !contains_legacy_french_drafting_text(field) && !contains_non_ascii_letter(field)
    })
}

fn contains_legacy_french_drafting_text(text: &str) -> bool {
    const LEGACY_FRENCH_FRAGMENTS: &[&str] = &[
        "Les reperes",
        "Les dynamiques",
        "Les facteurs",
        "Une dominante",
        "Presenter en langage",
        "Expliquer en langage",
        "Expliquer les principales",
        "Montrer comment",
        "Situer les facteurs",
        "Rediger une section",
        "repeter chaque",
        "utiliser des IDs",
        "faire une prediction",
        "ajouter des informations",
        "presenter un aspect",
        "donner trop de poids",
        "en regroupant",
        "sans lister",
    ];

    LEGACY_FRENCH_FRAGMENTS
        .iter()
        .any(|fragment| text.contains(fragment))
}

fn contains_non_ascii_letter(text: &str) -> bool {
    text.chars()
        .any(|character| character.is_alphabetic() && !character.is_ascii())
}

fn has_text(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

fn has_current_aspect_article(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_none_or(|text| !text.contains(" by a opposition"))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::domain::{
        BasicDraftingPlanItem, BasicObjectPosition, BasicReadingPlanItem, BasicSignal,
        BasicWritingContract, HouseReference, SignReference,
    };

    fn current_payload() -> BasicPayload {
        BasicPayload {
            product_code: "basic".to_string(),
            chart_calculation_id: 1,
            reference_version_id: 1,
            subject_label: None,
            birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            writing_contract: Some(BasicWritingContract {
                audience_level: "beginner".to_string(),
                tone: "clear, warm, non fatalistic".to_string(),
                language: "fr".to_string(),
                max_total_words: 650,
                must_not: vec![
                    "list placements mechanically".to_string(),
                    "mention internal IDs".to_string(),
                    "invent facts not present in source signals".to_string(),
                    "use deterministic or fatalistic wording".to_string(),
                ],
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
            }],
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
                evidence: Some(json!({"fact_type": "object_position"})),
            }],
            reading_plan: vec![BasicReadingPlanItem {
                slot: "core_identity".to_string(),
                title: "Core identity markers".to_string(),
                source_signal_keys: vec!["object_position:sun".to_string()],
            }],
            drafting_plan: vec![BasicDraftingPlanItem {
                slot: "core_identity".to_string(),
                section_title: "Core chart markers".to_string(),
                source_signal_keys: vec!["object_position:sun".to_string()],
                writing_objective: "Explain the central markers.".to_string(),
                max_words: 110,
                avoid: vec!["use technical IDs".to_string()],
            }],
        }
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
    fn current_payload_requires_reading_plan() {
        let mut payload = current_payload();
        payload.reading_plan.clear();

        assert!(!is_current_basic_payload(&payload));
    }

    #[test]
    fn current_payload_requires_writing_contract() {
        let mut payload = current_payload();
        payload.writing_contract = None;

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
    fn current_payload_rejects_duplicate_reading_plan_slots() {
        let mut payload = current_payload();
        payload.reading_plan.push(BasicReadingPlanItem {
            slot: "core_identity".to_string(),
            title: "Duplicate".to_string(),
            source_signal_keys: vec!["object_position:sun".to_string()],
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
            evidence: Some(json!({"fact_type": "object_position"})),
        });
        payload.reading_plan.insert(
            0,
            BasicReadingPlanItem {
                slot: "expression_style".to_string(),
                title: "Expression style".to_string(),
                source_signal_keys: vec!["object_position:mercury".to_string()],
            },
        );
        payload.drafting_plan.insert(
            0,
            BasicDraftingPlanItem {
                slot: "expression_style".to_string(),
                section_title: "Expression and action style".to_string(),
                source_signal_keys: vec!["object_position:mercury".to_string()],
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
    fn current_payload_rejects_legacy_french_drafting_plan() {
        let mut payload = current_payload();
        payload.drafting_plan[0].section_title = "Les reperes centraux du theme".to_string();
        payload.drafting_plan[0].writing_objective =
            "Presenter en langage simple les marqueurs centraux.".to_string();
        payload.drafting_plan[0].avoid = vec!["utiliser des IDs techniques".to_string()];

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
                })
                .collect(),
            houses: (1..=12)
                .map(|number| HouseReference {
                    id: number + 100,
                    number,
                    name: format!("House {number}"),
                })
                .collect(),
        }
    }
}

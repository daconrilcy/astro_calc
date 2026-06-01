use std::collections::HashSet;

use chrono::Utc;
use sqlx::PgPool;
use thiserror::Error;

use crate::domain::{BasicPayload, CalculatedChartFacts, NatalChartInput, RuntimeOptions};
use crate::ephemeris::EphemerisEngine;
use crate::idempotency::{advisory_lock_key, idempotency_key, input_hash};
use crate::payload::build_basic_payload;
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
        let input_hash = input_hash(&input)?;
        let idempotency_key = idempotency_key(&input, &self.options)?;
        let lock_key = advisory_lock_key(&idempotency_key);

        let chart_objects = self.repository.active_chart_objects().await?;
        let aspect_definitions = self.repository.aspect_definitions().await?;
        let house_system = self.repository.house_system(input.house_system_id).await?;

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
        RuntimeRepository::mark_completed(&mut tx, chart_calculation_id).await?;
        tx.commit().await?;

        Ok(payload)
    }
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
        && has_current_reading_plan(payload)
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

    payload.reading_plan.iter().all(|item| {
        let slot = item.slot.trim();
        !slot.is_empty()
            && slots.insert(slot)
            && !item.title.trim().is_empty()
            && !item.source_signal_keys.is_empty()
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
    })
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
    use crate::domain::{BasicObjectPosition, BasicReadingPlanItem, BasicSignal};

    fn current_payload() -> BasicPayload {
        BasicPayload {
            product_code: "basic".to_string(),
            chart_calculation_id: 1,
            reference_version_id: 1,
            subject_label: None,
            birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
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
    fn current_payload_rejects_old_opposition_hint_template() {
        let mut payload = current_payload();
        payload.signals[0].interpretive_hint = Some(
            "Jupiter and Uranus are connected by a opposition, so their functions should be read together."
                .to_string(),
        );

        assert!(!is_current_basic_payload(&payload));
    }
}

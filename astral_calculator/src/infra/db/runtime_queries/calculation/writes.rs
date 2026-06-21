use super::super::*;
use crate::application::ports::CalculationProgressState;

const STATUS_RUNNING: &str = "running";
const STATUS_COMPLETED: &str = "completed";
const STATUS_FAILED: &str = "failed";

impl RuntimeQueries {
    pub async fn lock_idempotency(
        tx: &mut Transaction<'_, Postgres>,
        lock_key: i64,
    ) -> Result<(), RuntimeError> {
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(lock_key)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub async fn calculations_for_key(
        tx: &mut Transaction<'_, Postgres>,
        idempotency_key: &str,
    ) -> Result<Vec<ChartCalculationRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, ChartCalculationRow>(
            r#"
            SELECT id, status, execution_attempt, heartbeat_at, stale_after_seconds
            FROM astral_chart_calculations
            WHERE idempotency_key = $1
            ORDER BY execution_attempt DESC
            FOR UPDATE
            "#,
        )
        .bind(idempotency_key)
        .fetch_all(&mut **tx)
        .await?)
    }

    pub async fn mark_stale_failed(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE astral_chart_calculations
            SET status = $2,
                finished_at = now(),
                error_code = 'stale_running_timeout',
                error_message = 'Running calculation heartbeat exceeded stale threshold.'
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
        .bind(STATUS_FAILED)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn insert_running_calculation(
        tx: &mut Transaction<'_, Postgres>,
        input: &NatalChartInput,
        options: &RuntimeOptions,
        input_hash: &str,
        idempotency_key: &str,
        execution_attempt: i32,
    ) -> Result<i32, RuntimeError> {
        let input_json = serde_json::to_value(input)?;
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO astral_chart_calculations (
                reference_version_id, calculation_profile_id, chart_type, status,
                subject_label, input_hash, idempotency_key, execution_attempt,
                input_data_json, engine_version, ephemeris_version, started_at,
                heartbeat_at, progress_state, stale_after_seconds
            )
            VALUES (
                $1, $2, 'natal', $3,
                $4, $5, $6, $7,
                $8, $9, $10, now(),
                now(), 'started', $11
            )
            RETURNING id
            "#,
        )
        .bind(input.reference_version_id)
        .bind(input.calculation_profile_id)
        .bind(STATUS_RUNNING)
        .bind(&input.subject_label)
        .bind(input_hash)
        .bind(idempotency_key)
        .bind(execution_attempt)
        .bind(input_json)
        .bind(&options.engine_version)
        .bind(&options.ephemeris_version)
        .bind(options.stale_after_seconds)
        .fetch_one(&mut **tx)
        .await?;

        Ok(id)
    }

    pub async fn heartbeat(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        progress_state: CalculationProgressState,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE astral_chart_calculations
            SET heartbeat_at = now(), progress_state = $2
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
        .bind(progress_state.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn persist_facts(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError> {
        for cusp in &facts.house_cusps {
            insert_house_cusp(tx, chart_calculation_id, cusp).await?;
        }
        for position in &facts.positions {
            insert_position(tx, chart_calculation_id, position).await?;
        }
        for aspect in &facts.aspects {
            insert_aspect(tx, chart_calculation_id, aspect).await?;
        }
        Ok(())
    }

    pub async fn persist_signals(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        reference_version_id: i32,
        signals: &[InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        sqlx::query(
            r#"
            UPDATE astral_interpretation_signals
            SET suppression_state = 'suppressed'
            WHERE chart_calculation_id = $1
            "#,
        )
        .bind(chart_calculation_id)
        .execute(&mut **tx)
        .await?;

        for signal in signals {
            sqlx::query(
                r#"
                INSERT INTO astral_interpretation_signals (
                    chart_calculation_id, reference_version_id, signal_key,
                    signal_type_id, theme_code, title, summary, priority_score,
                    confidence_score, suppression_state, payload_json
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                ON CONFLICT (chart_calculation_id, signal_key) DO UPDATE
                SET title = EXCLUDED.title,
                    signal_type_id = EXCLUDED.signal_type_id,
                    theme_code = EXCLUDED.theme_code,
                    summary = EXCLUDED.summary,
                    priority_score = EXCLUDED.priority_score,
                    confidence_score = EXCLUDED.confidence_score,
                    suppression_state = EXCLUDED.suppression_state,
                    payload_json = EXCLUDED.payload_json
                "#,
            )
            .bind(chart_calculation_id)
            .bind(reference_version_id)
            .bind(&signal.signal_key)
            .bind(signal.signal_type_id)
            .bind(&signal.theme_code)
            .bind(&signal.title)
            .bind(&signal.summary)
            .bind(signal.priority_score)
            .bind(signal.confidence_score)
            .bind(&signal.suppression_state)
            .bind(&signal.payload_json)
            .execute(&mut **tx)
            .await?;
        }

        Ok(sqlx::query_as::<_, InterpretationSignalRow>(
            r#"
            SELECT id, signal_key, theme_code, title, summary,
                   priority_score::float8 AS priority_score,
                   confidence_score::float8 AS confidence_score, payload_json
            FROM astral_interpretation_signals
            WHERE chart_calculation_id = $1 AND suppression_state = 'active'
            ORDER BY priority_score DESC, id
            LIMIT 12
            "#,
        )
        .bind(chart_calculation_id)
        .fetch_all(&mut **tx)
        .await?)
    }

    pub async fn persist_basic_payload(
        tx: &mut Transaction<'_, Postgres>,
        input: &NatalChartInput,
        payload_language_id: Option<i32>,
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        let payload_json = serde_json::to_value(payload)?;
        sqlx::query(
            r#"
            INSERT INTO astral_interpretation_generation_payloads (
                chart_calculation_id, reference_version_id, product_code,
                language_id, payload_json, created_at
            )
            VALUES ($1,$2,$3,$4,$5,now())
            ON CONFLICT (chart_calculation_id, product_code, language_id) DO UPDATE
            SET payload_json = EXCLUDED.payload_json,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(payload.chart_calculation_id)
        .bind(input.reference_version_id)
        .bind(input.product_code())
        .bind(payload_language_id)
        .bind(payload_json)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn mark_completed(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE astral_chart_calculations
            SET status = $2,
                heartbeat_at = now(),
                progress_state = $3,
                finished_at = now()
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
        .bind(STATUS_COMPLETED)
        .bind(CalculationProgressState::Completed.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        error: &RuntimeError,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE astral_chart_calculations
            SET status = $2,
                heartbeat_at = now(),
                progress_state = $3,
                finished_at = now(),
                error_code = $4,
                error_message = $5
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
        .bind(STATUS_FAILED)
        .bind(CalculationProgressState::Failed.as_str())
        .bind(error.code())
        .bind(error.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

async fn insert_house_cusp(
    tx: &mut Transaction<'_, Postgres>,
    chart_calculation_id: i32,
    cusp: &HouseCuspFact,
) -> Result<(), RuntimeError> {
    sqlx::query(
        r#"
        INSERT INTO astral_calculated_house_cusps (
            chart_calculation_id, house_id, sign_id, longitude_deg
        )
        VALUES ($1,$2,$3,$4)
        ON CONFLICT (chart_calculation_id, house_id) DO UPDATE
        SET sign_id = EXCLUDED.sign_id,
            longitude_deg = EXCLUDED.longitude_deg
        "#,
    )
    .bind(chart_calculation_id)
    .bind(cusp.house_id)
    .bind(cusp.sign_id)
    .bind(cusp.longitude_deg)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_position(
    tx: &mut Transaction<'_, Postgres>,
    chart_calculation_id: i32,
    position: &ObjectPositionFact,
) -> Result<(), RuntimeError> {
    sqlx::query(
        r#"
        INSERT INTO astral_calculated_chart_object_positions (
            chart_calculation_id, chart_object_id, zodiacal_reference_system_id,
            coordinate_reference_system_id, sign_id, house_id, motion_state_id,
            horizon_position_id, longitude_deg, latitude_deg, apparent_speed_deg_per_day,
            altitude_deg, is_visible, facts_json
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (
            chart_calculation_id, chart_object_id, zodiacal_reference_system_id,
            coordinate_reference_system_id
        ) DO UPDATE
        SET sign_id = EXCLUDED.sign_id,
            house_id = EXCLUDED.house_id,
            motion_state_id = EXCLUDED.motion_state_id,
            horizon_position_id = EXCLUDED.horizon_position_id,
            longitude_deg = EXCLUDED.longitude_deg,
            latitude_deg = EXCLUDED.latitude_deg,
            apparent_speed_deg_per_day = EXCLUDED.apparent_speed_deg_per_day,
            altitude_deg = EXCLUDED.altitude_deg,
            is_visible = EXCLUDED.is_visible,
            facts_json = EXCLUDED.facts_json
        "#,
    )
    .bind(chart_calculation_id)
    .bind(position.chart_object_id)
    .bind(position.zodiacal_reference_system_id)
    .bind(position.coordinate_reference_system_id)
    .bind(position.sign_id)
    .bind(position.house_id)
    .bind(position.motion_state_id)
    .bind(position.horizon_position_id)
    .bind(position.longitude_deg)
    .bind(position.latitude_deg)
    .bind(position.apparent_speed_deg_per_day)
    .bind(position.altitude_deg)
    .bind(position.is_visible)
    .bind(&position.facts_json)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_aspect(
    tx: &mut Transaction<'_, Postgres>,
    chart_calculation_id: i32,
    aspect: &AspectFact,
) -> Result<(), RuntimeError> {
    sqlx::query(
        r#"
        INSERT INTO astral_calculated_aspects (
            chart_calculation_id, source_chart_object_id, target_chart_object_id,
            aspect_id, aspect_definition_id, orb_deg, phase_state, is_applying,
            is_exact, strength_score, calculation_notes_json
        )
        VALUES ($1,$2,$3,$4,NULL,$5,$6,$7,$8,$9,$10)
        ON CONFLICT (
            chart_calculation_id, source_chart_object_id, target_chart_object_id, aspect_id
        ) DO UPDATE
        SET orb_deg = EXCLUDED.orb_deg,
            phase_state = EXCLUDED.phase_state,
            is_applying = EXCLUDED.is_applying,
            is_exact = EXCLUDED.is_exact,
            strength_score = EXCLUDED.strength_score,
            calculation_notes_json = EXCLUDED.calculation_notes_json
        "#,
    )
    .bind(chart_calculation_id)
    .bind(aspect.source_chart_object_id)
    .bind(aspect.target_chart_object_id)
    .bind(aspect.aspect_id)
    .bind(aspect.orb_deg)
    .bind(&aspect.phase_state)
    .bind(aspect.is_applying)
    .bind(aspect.is_exact)
    .bind(aspect.strength_score)
    .bind(&aspect.calculation_notes_json)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

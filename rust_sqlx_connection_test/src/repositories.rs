use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::domain::{
    AspectDefinition, AspectFact, BasicPayload, CalculatedChartFacts, ChartObject, HouseCuspFact,
    HouseSystem, InterpretationSignalDraft, InterpretationSignalRow, NatalChartInput,
    ObjectPositionFact, RuntimeOptions,
};
use crate::runtime::RuntimeError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChartCalculationRow {
    pub id: i32,
    pub status: String,
    pub execution_attempt: i32,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub stale_after_seconds: Option<i32>,
}

pub struct RuntimeRepository {
    pool: PgPool,
}

impl RuntimeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn active_chart_objects(&self) -> Result<Vec<ChartObject>, RuntimeError> {
        Ok(sqlx::query_as::<_, ChartObject>(
            r#"
            SELECT id, code, name, swe_id
            FROM astral_chart_objects
            WHERE is_active = true AND is_calculable = true
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError> {
        Ok(sqlx::query_as::<_, AspectDefinition>(
            r#"
            SELECT id, code, name, angle::float8 AS angle
            FROM astral_aspects
            WHERE family = 'major'
            ORDER BY id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseSystem>(
            r#"
            SELECT id, code, calculation_engine_code
            FROM astral_house_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn existing_basic_payload(
        &self,
        chart_calculation_id: i32,
        product_code: &str,
        language_id: Option<i32>,
    ) -> Result<Option<BasicPayload>, RuntimeError> {
        let row = sqlx::query_scalar::<_, Value>(
            r#"
            SELECT payload_json
            FROM astral_interpretation_generation_payloads
            WHERE chart_calculation_id = $1
              AND product_code IS NOT DISTINCT FROM $2
              AND language_id IS NOT DISTINCT FROM $3
            "#,
        )
        .bind(chart_calculation_id)
        .bind(product_code)
        .bind(language_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(serde_json::from_value)
            .transpose()
            .map_err(Into::into)
    }

    pub async fn positions_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<ObjectPositionFact>, RuntimeError> {
        Ok(sqlx::query_as::<_, PersistedObjectPositionFact>(
            r#"
            SELECT p.chart_object_id,
                   o.code AS object_code,
                   o.name AS object_name,
                   p.zodiacal_reference_system_id,
                   p.coordinate_reference_system_id,
                   p.sign_id,
                   s.code AS sign_code,
                   s.name AS sign_name,
                   p.house_id,
                   h.number AS house_number,
                   h.name AS house_name,
                   p.motion_state_id,
                   p.horizon_position_id,
                   p.longitude_deg::float8 AS longitude_deg,
                   p.latitude_deg::float8 AS latitude_deg,
                   p.apparent_speed_deg_per_day::float8 AS apparent_speed_deg_per_day,
                   p.altitude_deg::float8 AS altitude_deg,
                   p.is_visible,
                   p.facts_json
            FROM astral_calculated_chart_object_positions p
            JOIN astral_chart_objects o ON o.id = p.chart_object_id
            JOIN astral_signs s ON s.id = p.sign_id
            LEFT JOIN astral_houses h ON h.id = p.house_id
            WHERE p.chart_calculation_id = $1
            ORDER BY o.sort_order, o.id
            "#,
        )
        .bind(chart_calculation_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    pub async fn active_signals(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, InterpretationSignalRow>(
            r#"
            SELECT id, signal_key, title, summary, priority_score::float8 AS priority_score,
                   confidence_score::float8 AS confidence_score, payload_json
            FROM astral_interpretation_signals
            WHERE chart_calculation_id = $1 AND suppression_state = 'active'
            ORDER BY priority_score DESC, id
            LIMIT 12
            "#,
        )
        .bind(chart_calculation_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn aspects_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        Ok(sqlx::query_as::<_, PersistedAspectFact>(
            r#"
            SELECT a.source_chart_object_id,
                   source.code AS source_object_code,
                   source.name AS source_object_name,
                   a.target_chart_object_id,
                   target.code AS target_object_code,
                   target.name AS target_object_name,
                   a.aspect_id,
                   aspect.code AS aspect_code,
                   aspect.name AS aspect_name,
                   a.orb_deg::float8 AS orb_deg,
                   a.phase_state,
                   a.is_applying,
                   a.is_exact,
                   a.strength_score::float8 AS strength_score,
                   a.calculation_notes_json
            FROM astral_calculated_aspects a
            JOIN astral_chart_objects source ON source.id = a.source_chart_object_id
            JOIN astral_chart_objects target ON target.id = a.target_chart_object_id
            JOIN astral_aspects aspect ON aspect.id = a.aspect_id
            WHERE a.chart_calculation_id = $1
            ORDER BY a.strength_score DESC NULLS LAST, a.orb_deg, a.id
            "#,
        )
        .bind(chart_calculation_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

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
            SET status = 'failed',
                finished_at = now(),
                error_code = 'stale_running_timeout',
                error_message = 'Running calculation heartbeat exceeded stale threshold.'
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
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
        let id = next_id(tx, "astral_chart_calculations").await?;
        let input_json = serde_json::to_value(input)?;

        sqlx::query(
            r#"
            INSERT INTO astral_chart_calculations (
                id, reference_version_id, calculation_profile_id, chart_type, status,
                subject_label, input_hash, idempotency_key, execution_attempt,
                input_data_json, engine_version, ephemeris_version, started_at,
                heartbeat_at, progress_state, stale_after_seconds
            )
            VALUES (
                $1, $2, $3, 'natal', 'running',
                $4, $5, $6, $7,
                $8, $9, $10, now(),
                now(), 'started', $11
            )
            "#,
        )
        .bind(id)
        .bind(input.reference_version_id)
        .bind(input.calculation_profile_id)
        .bind(&input.subject_label)
        .bind(input_hash)
        .bind(idempotency_key)
        .bind(execution_attempt)
        .bind(input_json)
        .bind(&options.engine_version)
        .bind(&options.ephemeris_version)
        .bind(options.stale_after_seconds)
        .execute(&mut **tx)
        .await?;

        Ok(id)
    }

    pub async fn heartbeat(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
        progress_state: &str,
    ) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            UPDATE astral_chart_calculations
            SET heartbeat_at = now(), progress_state = $2
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
        .bind(progress_state)
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
            let id = next_id(tx, "astral_interpretation_signals").await?;
            sqlx::query(
                r#"
                INSERT INTO astral_interpretation_signals (
                    id, chart_calculation_id, reference_version_id, signal_key,
                    signal_type_id, theme_code, title, summary, priority_score,
                    confidence_score, suppression_state, payload_json
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
                ON CONFLICT (chart_calculation_id, signal_key) DO UPDATE
                SET title = EXCLUDED.title,
                    summary = EXCLUDED.summary,
                    priority_score = EXCLUDED.priority_score,
                    confidence_score = EXCLUDED.confidence_score,
                    suppression_state = EXCLUDED.suppression_state,
                    payload_json = EXCLUDED.payload_json
                "#,
            )
            .bind(id)
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
            SELECT id, signal_key, title, summary, priority_score::float8 AS priority_score,
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
        payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        let id = next_id(tx, "astral_interpretation_generation_payloads").await?;
        let payload_json = serde_json::to_value(payload)?;
        sqlx::query(
            r#"
            INSERT INTO astral_interpretation_generation_payloads (
                id, chart_calculation_id, reference_version_id, product_code,
                language_id, payload_json, created_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,now())
            ON CONFLICT (chart_calculation_id, product_code, language_id) DO UPDATE
            SET payload_json = EXCLUDED.payload_json,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(id)
        .bind(payload.chart_calculation_id)
        .bind(input.reference_version_id)
        .bind(input.product_code())
        .bind(input.language_id)
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
            SET status = 'completed',
                heartbeat_at = now(),
                progress_state = 'completed',
                finished_at = now()
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
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
            SET status = 'failed',
                heartbeat_at = now(),
                progress_state = 'failed',
                finished_at = now(),
                error_code = $2,
                error_message = $3
            WHERE id = $1
            "#,
        )
        .bind(chart_calculation_id)
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
    let id = next_id(tx, "astral_calculated_house_cusps").await?;
    sqlx::query(
        r#"
        INSERT INTO astral_calculated_house_cusps (
            id, chart_calculation_id, house_id, sign_id, longitude_deg
        )
        VALUES ($1,$2,$3,$4,$5)
        ON CONFLICT (chart_calculation_id, house_id) DO UPDATE
        SET sign_id = EXCLUDED.sign_id,
            longitude_deg = EXCLUDED.longitude_deg
        "#,
    )
    .bind(id)
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
    let id = next_id(tx, "astral_calculated_chart_object_positions").await?;
    sqlx::query(
        r#"
        INSERT INTO astral_calculated_chart_object_positions (
            id, chart_calculation_id, chart_object_id, zodiacal_reference_system_id,
            coordinate_reference_system_id, sign_id, house_id, motion_state_id,
            horizon_position_id, longitude_deg, latitude_deg, apparent_speed_deg_per_day,
            altitude_deg, is_visible, facts_json
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
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
    .bind(id)
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
    let id = next_id(tx, "astral_calculated_aspects").await?;
    sqlx::query(
        r#"
        INSERT INTO astral_calculated_aspects (
            id, chart_calculation_id, source_chart_object_id, target_chart_object_id,
            aspect_id, aspect_definition_id, orb_deg, phase_state, is_applying,
            is_exact, strength_score, calculation_notes_json
        )
        VALUES ($1,$2,$3,$4,$5,NULL,$6,$7,$8,$9,$10,$11)
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
    .bind(id)
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

async fn next_id(
    tx: &mut Transaction<'_, Postgres>,
    table_name: &str,
) -> Result<i32, RuntimeError> {
    ensure_runtime_table_name(table_name)?;
    let lock_sql = format!("LOCK TABLE {table_name} IN EXCLUSIVE MODE");
    sqlx::query(&lock_sql).execute(&mut **tx).await?;
    let sql = format!("SELECT COALESCE(MAX(id), 0) + 1 FROM {table_name}");
    Ok(sqlx::query_scalar::<_, i32>(&sql)
        .fetch_one(&mut **tx)
        .await?)
}

fn ensure_runtime_table_name(table_name: &str) -> Result<(), RuntimeError> {
    match table_name {
        "astral_chart_calculations"
        | "astral_calculated_house_cusps"
        | "astral_calculated_chart_object_positions"
        | "astral_calculated_aspects"
        | "astral_interpretation_signals"
        | "astral_interpretation_generation_payloads" => Ok(()),
        _ => Err(RuntimeError::InvalidRuntimeTable(table_name.to_string())),
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PersistedObjectPositionFact {
    chart_object_id: i32,
    object_code: String,
    object_name: String,
    zodiacal_reference_system_id: i32,
    coordinate_reference_system_id: i32,
    sign_id: i32,
    sign_code: String,
    sign_name: String,
    house_id: Option<i32>,
    house_number: Option<i32>,
    house_name: Option<String>,
    motion_state_id: Option<i32>,
    horizon_position_id: Option<i32>,
    longitude_deg: f64,
    latitude_deg: Option<f64>,
    apparent_speed_deg_per_day: Option<f64>,
    altitude_deg: Option<f64>,
    is_visible: Option<bool>,
    facts_json: Option<Value>,
}

impl From<PersistedObjectPositionFact> for ObjectPositionFact {
    fn from(row: PersistedObjectPositionFact) -> Self {
        Self {
            chart_object_id: row.chart_object_id,
            object_code: row.object_code,
            object_name: row.object_name,
            zodiacal_reference_system_id: row.zodiacal_reference_system_id,
            coordinate_reference_system_id: row.coordinate_reference_system_id,
            sign_id: row.sign_id,
            sign_code: row.sign_code,
            sign_name: row.sign_name,
            house_id: row.house_id,
            house_number: row.house_number,
            house_name: row.house_name,
            motion_state_id: row.motion_state_id,
            horizon_position_id: row.horizon_position_id,
            longitude_deg: row.longitude_deg,
            latitude_deg: row.latitude_deg,
            apparent_speed_deg_per_day: row.apparent_speed_deg_per_day,
            altitude_deg: row.altitude_deg,
            is_visible: row.is_visible,
            facts_json: row.facts_json,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PersistedAspectFact {
    source_chart_object_id: i32,
    source_object_code: String,
    source_object_name: String,
    target_chart_object_id: i32,
    target_object_code: String,
    target_object_name: String,
    aspect_id: i32,
    aspect_code: String,
    aspect_name: String,
    orb_deg: f64,
    phase_state: String,
    is_applying: bool,
    is_exact: bool,
    strength_score: Option<f64>,
    calculation_notes_json: Option<Value>,
}

impl From<PersistedAspectFact> for AspectFact {
    fn from(row: PersistedAspectFact) -> Self {
        Self {
            source_chart_object_id: row.source_chart_object_id,
            source_object_code: row.source_object_code,
            source_object_name: row.source_object_name,
            target_chart_object_id: row.target_chart_object_id,
            target_object_code: row.target_object_code,
            target_object_name: row.target_object_name,
            aspect_id: row.aspect_id,
            aspect_code: row.aspect_code,
            aspect_name: row.aspect_name,
            orb_deg: row.orb_deg,
            phase_state: row.phase_state,
            is_applying: row.is_applying,
            is_exact: row.is_exact,
            strength_score: row.strength_score,
            calculation_notes_json: row.calculation_notes_json,
        }
    }
}

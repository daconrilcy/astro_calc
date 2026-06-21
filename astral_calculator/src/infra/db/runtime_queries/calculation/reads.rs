use super::super::*;

const STATUS_COMPLETED: &str = "completed";

impl RuntimeQueries {
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

        row.map(parse_existing_basic_payload_value)
            .transpose()
            .map(Option::flatten)
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
                   COALESCE(p.facts_json, '{}'::jsonb)
                   || jsonb_build_object(
                       'sign_context', jsonb_strip_nulls(jsonb_build_object(
                           'element', element.code,
                           'element_label', element.label,
                           'modality', sign_modality.code,
                           'modality_label', sign_modality.name,
                           'polarity', polarity.code,
                           'polarity_label', polarity.name,
                           'keywords', sign_keywords.keywords_json
                       )),
                       'house_modality', CASE
                           WHEN house_modality.id IS NULL THEN NULL
                           ELSE jsonb_strip_nulls(jsonb_build_object(
                               'code', house_modality.name,
                               'label', house_modality.label,
                               'accidental_strength', house_modality.accidental_strength,
                               'priority_delta', house_modality.priority_delta,
                               'interpretation_weight', house_modality.interpretation_weight
                           ))
                       END,
                       'house_context', CASE
                           WHEN h.id IS NULL THEN NULL
                           ELSE jsonb_strip_nulls(jsonb_build_object(
                               'theme_code', h.theme_code
                           ))
                       END,
                       'object_context', jsonb_strip_nulls(jsonb_build_object(
                           'role', role.code,
                           'role_label', role.label,
                           'nature', (
                               SELECT jsonb_agg(nt.code ORDER BY nt.sort_order, nt.id)
                               FROM astral_object_nature_assignments na
                               JOIN astral_object_nature_types nt ON nt.id = na.nature_type_id
                               WHERE na.chart_object_id = o.id
                                 AND na.is_primary = true
                           ),
                           'is_luminary', object_definition.is_luminary,
                           'is_planet_symbolic', object_definition.is_planet_symbolic,
                           'is_visible_to_naked_eye', object_definition.is_visible_to_naked_eye,
                           'signal_scoring', CASE
                               WHEN signal_profile.id IS NULL THEN NULL
                               ELSE jsonb_strip_nulls(jsonb_build_object(
                                   'position_priority_base', signal_profile.position_priority_base,
                                   'angle_priority_base', signal_profile.angle_priority_base,
                                   'source_weight', signal_profile.source_weight
                               ))
                           END
                       )),
                       'motion_context', CASE
                           WHEN motion.id IS NULL THEN NULL
                           ELSE jsonb_strip_nulls(jsonb_build_object(
                               'motion_state', motion.code,
                               'label', motion.label,
                               'motion_family', motion.motion_family
                           ))
                       END
                   ) AS facts_json
            FROM astral_calculated_chart_object_positions p
            JOIN astral_chart_calculations c ON c.id = p.chart_calculation_id
            JOIN astral_chart_objects o ON o.id = p.chart_object_id
            JOIN astral_signs s ON s.id = p.sign_id
            LEFT JOIN astral_sign_profiles sign_profile ON sign_profile.astral_sign_id = s.id
            LEFT JOIN astral_elements element ON element.id = sign_profile.astral_element_id
            LEFT JOIN astral_modalities sign_modality ON sign_modality.id = sign_profile.astral_modality_id
            LEFT JOIN astral_polarities polarity ON polarity.id = sign_profile.astral_polarity_id
            LEFT JOIN astral_sign_keywords sign_keywords ON sign_keywords.astral_sign_id = s.id
            LEFT JOIN astral_houses h ON h.id = p.house_id
            LEFT JOIN astral_house_modalities house_modality ON house_modality.id = h.house_modality_id
            LEFT JOIN astral_chart_object_definitions object_definition ON object_definition.chart_object_id = o.id
            LEFT JOIN astral_astrological_roles role ON role.id = object_definition.astrological_role_id
            LEFT JOIN astral_chart_object_signal_profiles signal_profile
              ON signal_profile.chart_object_id = o.id
             AND signal_profile.reference_version_id = c.reference_version_id
            LEFT JOIN astral_object_motion_states motion ON motion.id = p.motion_state_id
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

    pub async fn natal_input_for_calculation(
        &self,
        chart_calculation_id: i32,
    ) -> Result<NatalChartInput, RuntimeError> {
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT input_data_json
            FROM astral_chart_calculations
            WHERE id = $1
              AND chart_type = 'natal'
              AND status = $2
            "#,
        )
        .bind(chart_calculation_id)
        .bind(STATUS_COMPLETED)
        .fetch_optional(&self.pool)
        .await?;
        let Some(value) = row else {
            return Err(RuntimeError::InvalidEngineRequest(
                "completed natal calculation input not found".to_string(),
            ));
        };
        Ok(serde_json::from_value(value)?)
    }

    pub async fn aspects_for_payload(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        read_aspects_for_payload(chart_calculation_id, &self.pool).await
    }

    pub async fn aspects_for_payload_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        read_aspects_for_payload(chart_calculation_id, &mut **tx).await
    }
}

async fn read_aspects_for_payload<'e, E>(
    chart_calculation_id: i32,
    executor: E,
) -> Result<Vec<AspectFact>, RuntimeError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
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
               aspect.family AS aspect_family,
               a.orb_deg::float8 AS orb_deg,
               a.phase_state,
               a.is_applying,
               a.is_exact,
               a.strength_score::float8 AS strength_score,
               primary_valence.name AS primary_valence,
               intensity_modifier.name AS intensity_modifier,
               secondary_effect.name AS secondary_effect,
               COALESCE(
                   primary_valence.interpretive_family,
                   intensity_modifier.interpretive_family,
                   secondary_effect.interpretive_family
               ) AS valence_family,
               COALESCE(
                   primary_valence.is_tonal_valence,
                   intensity_modifier.is_tonal_valence,
                   secondary_effect.is_tonal_valence
               ) AS valence_is_tonal,
               COALESCE(
                   primary_valence.is_intensity_modifier,
                   intensity_modifier.is_intensity_modifier,
                   secondary_effect.is_intensity_modifier
               ) AS valence_is_intensity_modifier,
               a.calculation_notes_json
        FROM astral_calculated_aspects a
        JOIN astral_chart_objects source ON source.id = a.source_chart_object_id
        JOIN astral_chart_objects target ON target.id = a.target_chart_object_id
        JOIN astral_aspects aspect ON aspect.id = a.aspect_id
        LEFT JOIN astral_aspect_profiles profile
          ON profile.aspect_id = aspect.id
         AND profile.reference_version_id = (
             SELECT reference_version_id
             FROM astral_chart_calculations
             WHERE id = $1
         )
        LEFT JOIN LATERAL (
            SELECT valence.name,
                   valence.interpretive_family,
                   valence.is_tonal_valence,
                   valence.is_intensity_modifier
            FROM astral_aspect_interpretive_effects effect
            JOIN astral_interpretive_valence valence
              ON valence.id = effect.interpretive_valence_id
             AND valence.is_active = true
            WHERE effect.aspect_profile_id = profile.id
              AND effect.reference_version_id = profile.reference_version_id
              AND effect.effect_role = 'primary_valence'
            ORDER BY effect.weight DESC, effect.sort_order, effect.id
            LIMIT 1
        ) primary_valence ON true
        LEFT JOIN LATERAL (
            SELECT valence.name,
                   valence.interpretive_family,
                   valence.is_tonal_valence,
                   valence.is_intensity_modifier
            FROM astral_aspect_interpretive_effects effect
            JOIN astral_interpretive_valence valence
              ON valence.id = effect.interpretive_valence_id
             AND valence.is_active = true
            WHERE effect.aspect_profile_id = profile.id
              AND effect.reference_version_id = profile.reference_version_id
              AND effect.effect_role = 'intensity_modifier'
            ORDER BY effect.weight DESC, effect.sort_order, effect.id
            LIMIT 1
        ) intensity_modifier ON true
        LEFT JOIN LATERAL (
            SELECT valence.name,
                   valence.interpretive_family,
                   valence.is_tonal_valence,
                   valence.is_intensity_modifier
            FROM astral_aspect_interpretive_effects effect
            JOIN astral_interpretive_valence valence
              ON valence.id = effect.interpretive_valence_id
             AND valence.is_active = true
            WHERE effect.aspect_profile_id = profile.id
              AND effect.reference_version_id = profile.reference_version_id
              AND effect.effect_role = 'secondary_effect'
            ORDER BY effect.weight DESC, effect.sort_order, effect.id
            LIMIT 1
        ) secondary_effect ON true
        WHERE a.chart_calculation_id = $1
        ORDER BY a.strength_score DESC NULLS LAST, a.orb_deg, a.id
        "#,
    )
    .bind(chart_calculation_id)
    .fetch_all(executor)
    .await?
    .into_iter()
    .map(Into::into)
    .collect())
}

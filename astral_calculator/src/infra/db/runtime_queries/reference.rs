//! Requetes SQL runtime specialisees.

use super::*;

impl RuntimeQueries {
    pub async fn active_chart_objects(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<ChartObject>, RuntimeError> {
        Ok(sqlx::query_as::<_, ChartObject>(
            r#"
            SELECT o.id, o.code, o.name, o.swe_id,
                   role.code AS role_code,
                   role.label AS role_label,
                   def.is_luminary,
                   def.is_planet_symbolic,
                   def.is_visible_to_naked_eye,
                   (
                       SELECT jsonb_agg(nt.code ORDER BY nt.sort_order, nt.id)
                       FROM astral_object_nature_assignments na
                       JOIN astral_object_nature_types nt ON nt.id = na.nature_type_id
                       WHERE na.chart_object_id = o.id
                         AND na.is_primary = true
                   ) AS nature_codes,
                   signal_profile.position_priority_base::float8 AS position_priority_base,
                   signal_profile.angle_priority_base::float8 AS angle_priority_base,
                   signal_profile.source_weight::float8 AS source_weight
            FROM astral_chart_objects o
            LEFT JOIN astral_chart_object_definitions def ON def.chart_object_id = o.id
            LEFT JOIN astral_astrological_roles role ON role.id = def.astrological_role_id
            LEFT JOIN astral_chart_object_signal_profiles signal_profile
              ON signal_profile.chart_object_id = o.id
             AND signal_profile.reference_version_id = $1
            WHERE o.is_active = true AND o.is_calculable = true
            ORDER BY o.sort_order, o.id
            "#,
        )
        .bind(reference_version_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction aspect_definitions.
    pub async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError> {
        Ok(sqlx::query_as::<_, AspectDefinition>(
            r#"
            SELECT a.id,
                   a.code,
                   a.name,
                   a.angle::float8 AS angle,
                   a.family,
                   a.default_orb_deg::float8 AS default_orb_deg,
                   f.max_default_orb_deg::float8 AS max_default_orb_deg
            FROM astral_aspects a
            INNER JOIN astral_aspect_families f ON f.name = a.family
            WHERE a.family = 'major'
            ORDER BY a.id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction major_aspect_family_reference.
    pub async fn major_aspect_family_reference(
        &self,
    ) -> Result<MajorAspectFamilyReference, RuntimeError> {
        let row = sqlx::query_as::<_, MajorAspectFamilyReference>(
            r#"
            SELECT expected_aspect_count, max_default_orb_deg::float8 AS max_default_orb_deg
            FROM astral_aspect_families
            WHERE name = 'major'
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(RuntimeError::Ephemeris(
                "missing major aspect family reference (astral_aspect_families.name = 'major')"
                    .to_string(),
            ));
        };
        if row.expected_aspect_count <= 0 {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid expected_aspect_count for major aspect family: {}",
                row.expected_aspect_count
            )));
        }
        if !row.max_default_orb_deg.is_finite() || row.max_default_orb_deg <= 0.0 {
            return Err(RuntimeError::Ephemeris(format!(
                "invalid max_default_orb_deg for major aspect family: {}",
                row.max_default_orb_deg
            )));
        }
        Ok(row)
    }

    /// Fonction sign_references.
    pub async fn sign_references(&self) -> Result<Vec<SignReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, SignReference>(
            r#"
            SELECT s.id, s.code, s.name,
                   element.code AS element_code,
                   element.label AS element_label,
                   modality.code AS modality_code,
                   modality.name AS modality_name,
                   polarity.code AS polarity_code,
                   polarity.name AS polarity_name,
                   keywords.keywords_json,
                   keywords.shadow_keywords_json
            FROM astral_signs s
            LEFT JOIN astral_sign_profiles profile ON profile.astral_sign_id = s.id
            LEFT JOIN astral_elements element ON element.id = profile.astral_element_id
            LEFT JOIN astral_modalities modality ON modality.id = profile.astral_modality_id
            LEFT JOIN astral_polarities polarity ON polarity.id = profile.astral_polarity_id
            LEFT JOIN astral_sign_keywords keywords ON keywords.astral_sign_id = s.id
            ORDER BY s.id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction house_references.
    pub async fn house_references(&self) -> Result<Vec<HouseReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseReference>(
            r#"
            SELECT h.id, h.number, h.name, h.theme_code,
                   modality.name AS modality_code,
                   modality.label AS modality_label,
                   modality.accidental_strength,
                   modality.priority_delta::float8 AS modality_priority_delta,
                   modality.interpretation_weight
            FROM astral_houses h
            LEFT JOIN astral_house_modalities modality ON modality.id = h.house_modality_id
            ORDER BY h.number
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction motion_state_references.
    pub async fn motion_state_references(&self) -> Result<Vec<MotionStateReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, MotionStateReference>(
            r#"
            SELECT id, code, label, motion_family
            FROM astral_object_motion_states
            WHERE is_active = true
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction horizon_position_references.
    pub async fn horizon_position_references(
        &self,
    ) -> Result<Vec<HorizonPositionReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, HorizonPositionReference>(
            r#"
            SELECT id, code, label
            FROM astral_horizon_positions
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction angle_point_references.
    pub async fn angle_point_references(&self) -> Result<Vec<AnglePointReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, AnglePointReference>(
            r#"
            SELECT angle.id,
                   angle.code,
                   angle.short_label,
                   angle.full_name,
                   angle.axis,
                   angle.opposite_angle_code,
                   angle.associated_house,
                   angle.description,
                   object.id AS chart_object_id,
                   object.code AS chart_object_code,
                   object.name AS chart_object_name,
                   object.sort_order AS chart_object_sort_order
            FROM astral_angle_points angle
            JOIN astral_chart_objects object
              ON lower(object.name) = lower(angle.full_name)
             AND object.is_active = true
             AND object.is_calculable = true
            ORDER BY object.sort_order, angle.id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction domicile_ruler_references.
    pub async fn domicile_ruler_references(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<DomicileRulerReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, DomicileRulerReference>(
            r#"
            SELECT dignity.reference_version_id,
                   dignity.astral_system_id,
                   system.name AS astral_system_code,
                   sign.id AS sign_id,
                   sign.code AS sign_code,
                   sign.name AS sign_name,
                   object.id AS chart_object_id,
                   object.code AS object_code,
                   object.name AS object_name,
                   dignity_type.code AS dignity_type,
                   dignity.weight::float8 AS weight,
                   dignity.is_primary
            FROM astral_object_sign_dignities dignity
            JOIN astral_systems system ON system.id = dignity.astral_system_id
            JOIN astral_signs sign ON sign.id = dignity.astral_sign_id
            JOIN astral_chart_objects object ON object.id = dignity.chart_object_id
            JOIN astral_dignity_type dignity_type
              ON dignity_type.id = dignity.astral_dignity_type_id
            WHERE dignity_type.code = 'domicile'
              AND dignity.reference_version_id IS NOT DISTINCT FROM $1
              AND object.is_active = true
            ORDER BY sign.id,
                     dignity.is_primary DESC,
                     dignity.weight DESC,
                     system.id,
                     object.sort_order,
                     object.id
            "#,
        )
        .bind(reference_version_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Fonction house_axis_references.
    pub async fn house_axis_references(
        &self,
    ) -> Result<Vec<crate::domain::HouseAxisReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseAxisReferenceRow>(
            r#"
            SELECT axis.key AS axis_code,
                   house_a.number AS house_a_number,
                   house_b.number AS house_b_number,
                   house_a.theme_code AS theme_a_code,
                   house_b.theme_code AS theme_b_code,
                   axis.title AS label,
                   axis.summary AS description
            FROM astral_house_axis_definitions axis
            JOIN astral_house_axis_members member_a
              ON member_a.axis_id = axis.id
            JOIN astral_houses house_a
              ON house_a.id = member_a.house_id
            JOIN astral_houses house_b
              ON house_b.id = member_a.opposite_house_id
            WHERE house_a.number < house_b.number
            ORDER BY house_a.number
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction lunar_phase_references.
    pub async fn lunar_phase_references(
        &self,
    ) -> Result<Vec<crate::domain::LunarPhaseReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, LunarPhaseReferenceRow>(
            r#"
            SELECT phase_code,
                   label,
                   cycle_family,
                   range_start_deg::float8 AS range_start_deg,
                   range_end_deg::float8 AS range_end_deg,
                   exact_anchor_deg::float8 AS exact_anchor_deg,
                   is_major_lunar_phase,
                   description
            FROM astral_lunar_phase_definitions
            WHERE is_active = true
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction accidental_dignity_condition_references.
    pub async fn accidental_dignity_condition_references(
        &self,
    ) -> Result<Vec<crate::domain::AccidentalDignityConditionReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, AccidentalDignityConditionReferenceRow>(
            r#"
            SELECT condition_code,
                   condition_family,
                   label,
                   polarity,
                   strength_score::float8 AS strength_score,
                   score_delta::float8 AS score_delta,
                   description
            FROM astral_accidental_dignity_condition_definitions
            WHERE is_active = true
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction object_sect_affinity_references.
    pub async fn object_sect_affinity_references(
        &self,
    ) -> Result<Vec<crate::domain::ObjectSectAffinityReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, ObjectSectAffinityReferenceRow>(
            r#"
            SELECT object_code,
                   sect_affinity_code,
                   is_variable,
                   description
            FROM astral_object_sect_affinities
            WHERE is_active = true
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction basic_product_scoring_profile.
    pub async fn basic_product_scoring_profile(
        &self,
        product_code: &str,
        payload_contract_version: &str,
    ) -> Result<BasicProductScoringProfile, RuntimeError> {
        let row = sqlx::query_as::<_, BasicProductScoringProfileRow>(
            r#"
            SELECT product_code,
                   payload_contract_version,
                   default_major_orb_deg::float8 AS default_major_orb_deg,
                   sign_emphasis_full_score::float8 AS sign_emphasis_full_score,
                   house_emphasis_full_score::float8 AS house_emphasis_full_score,
                   object_emphasis_full_score::float8 AS object_emphasis_full_score,
                   sign_house_emphasis_min_score::float8 AS sign_house_emphasis_min_score,
                   object_emphasis_min_score::float8 AS object_emphasis_min_score,
                   house_axis_full_score::float8 AS house_axis_full_score,
                   axis_min_score::float8 AS axis_min_score,
                   axis_secondary_weight::float8 AS axis_secondary_weight,
                   axis_polarity_dominance_delta::float8 AS axis_polarity_dominance_delta,
                   axis_balanced_min_score::float8 AS axis_balanced_min_score,
                   max_dominant_signs,
                   max_dominant_houses,
                   max_dominant_objects,
                   max_active_signals,
                   aspect_min_strength::float8 AS aspect_min_strength,
                   max_house_axis_emphasis,
                   accidental_scoring_params_id,
                   essential_dignity_score_profile_id
            FROM astral_basic_product_scoring_profiles
            WHERE product_code = $1
              AND payload_contract_version = $2
              AND is_active = true
            "#,
        )
        .bind(product_code)
        .bind(payload_contract_version)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    /// Fonction essential_dignity_rule_references.
    pub async fn essential_dignity_rule_references(
        &self,
        reference_version_id: i32,
        score_profile_id: i32,
    ) -> Result<Vec<EssentialDignityRuleReference>, RuntimeError> {
        Ok(sqlx::query_as::<_, EssentialDignityRuleReferenceRow>(
            r#"
            SELECT object.code AS object_code,
                   sign.code AS sign_code,
                   dignity_type.code AS dignity_type,
                   dignity_type.label AS dignity_label,
                   CASE
                       WHEN functional_effect.code = 'weakening' THEN 'debility'
                       ELSE 'dignity'
                   END AS polarity,
                   weight.expression_weight::float8 AS strength_score,
                   weight.priority_delta::float8 AS priority_delta,
                   weight.signal_weight_delta::float8 AS signal_weight_delta,
                   weight.signal_worthy_min_strength::float8 AS signal_worthy_min_strength,
                   weight.emphasis_weight::float8 AS emphasis_weight
            FROM astral_essential_dignity_rules rule
            JOIN astral_chart_objects object ON object.id = rule.chart_object_id
            JOIN astral_signs sign ON sign.id = rule.sign_id
            JOIN astral_essential_dignity_types dignity_type
              ON dignity_type.id = rule.essential_dignity_types_id
            JOIN astral_dignity_functional_effects functional_effect
              ON functional_effect.id = dignity_type.functional_effect_id
            JOIN astral_essential_dignity_score_weights weight
              ON weight.essential_dignity_types_id = dignity_type.id
             AND weight.score_profile_id = $2
            WHERE dignity_type.code IN ('domicile', 'exaltation', 'detriment', 'fall')
              AND rule.reference_version_id IS NOT DISTINCT FROM $1
            ORDER BY object.sort_order, sign.id, dignity_type.sort_order
            "#,
        )
        .bind(reference_version_id)
        .bind(score_profile_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction accidental_condition_triggers.
    pub async fn accidental_condition_triggers(
        &self,
    ) -> Result<Vec<AccidentalConditionTrigger>, RuntimeError> {
        Ok(sqlx::query_as::<_, AccidentalConditionTriggerRow>(
            r#"
            SELECT trigger_family,
                   source_code,
                   angle_object_code,
                   condition_code
            FROM astral_accidental_condition_triggers
            WHERE is_active = true
            ORDER BY sort_order, id
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction accidental_scoring_params.
    pub async fn accidental_scoring_params(
        &self,
        params_id: i32,
    ) -> Result<AccidentalScoringParams, RuntimeError> {
        let row = sqlx::query_as::<_, AccidentalScoringParamsRow>(
            r#"
            SELECT code,
                   overall_score_baseline::float8 AS overall_score_baseline,
                   overall_score_min::float8 AS overall_score_min,
                   overall_score_max::float8 AS overall_score_max,
                   angle_proximity_max_orb_deg::float8 AS angle_proximity_max_orb_deg
            FROM astral_accidental_scoring_params
            WHERE id = $1
              AND is_active = true
            "#,
        )
        .bind(params_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    /// Fonction accidental_overall_polarity_bands.
    pub async fn accidental_overall_polarity_bands(
        &self,
        params_id: i32,
    ) -> Result<Vec<AccidentalPolarityBand>, RuntimeError> {
        Ok(sqlx::query_as::<_, AccidentalPolarityBandRow>(
            r#"
            SELECT polarity_code,
                   expression_quality_code,
                   min_score::float8 AS min_score,
                   max_score::float8 AS max_score,
                   sort_order
            FROM astral_accidental_overall_polarity_bands
            WHERE accidental_scoring_params_id = $1
            ORDER BY sort_order, id
            "#,
        )
        .bind(params_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    /// Fonction basic_payload_catalog.
    pub async fn house_system_id_by_code(&self, code: &str) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_house_systems
            WHERE code = $1 AND is_active = true
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!("unknown house_system: {code}"))
        })
    }

    /// Fonction zodiacal_reference_system_id_by_key.
    pub async fn zodiacal_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_zodiacal_reference_systems
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!("unknown zodiacal_reference_system: {key}"))
        })
    }

    /// Fonction zodiacal_reference_systems.
    pub async fn zodiacal_reference_systems(
        &self,
    ) -> Result<Vec<crate::infra::db::models::ZodiacalReferenceSystemRow>, RuntimeError> {
        Ok(
            sqlx::query_as::<_, crate::infra::db::models::ZodiacalReferenceSystemRow>(
                r#"
            SELECT id,
                   key,
                   display_name,
                   category_id,
                   description,
                   requires_ayanamsha,
                   usage_note
            FROM astral_zodiacal_reference_systems
            ORDER BY id
            "#,
            )
            .fetch_all(&self.pool)
            .await?,
        )
    }

    /// Fonction coordinate_reference_system_id_by_key.
    pub async fn coordinate_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_coordinate_reference_systems
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!(
                "unknown coordinate_reference_system: {key}"
            ))
        })
    }

    /// Fonction coordinate_reference_systems.
    pub async fn coordinate_reference_systems(
        &self,
    ) -> Result<Vec<crate::infra::db::models::CoordinateReferenceSystemRow>, RuntimeError> {
        Ok(
            sqlx::query_as::<_, crate::infra::db::models::CoordinateReferenceSystemRow>(
                r#"
            SELECT id,
                   key,
                   display_name,
                   category_id,
                   description,
                   usage_note
            FROM astral_coordinate_reference_systems
            ORDER BY id
            "#,
            )
            .fetch_all(&self.pool)
            .await?,
        )
    }

    /// Fonction zodiacal_reference_system_display_name.
    pub async fn zodiacal_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        let name = sqlx::query_scalar::<_, String>(
            r#"
            SELECT display_name
            FROM astral_zodiacal_reference_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        name.ok_or_else(|| RuntimeError::InvalidRuntimeTable(format!("zodiac system {id} missing")))
    }

    /// Fonction coordinate_reference_system_display_name.
    pub async fn coordinate_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        let name = sqlx::query_scalar::<_, String>(
            r#"
            SELECT display_name
            FROM astral_coordinate_reference_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        name.ok_or_else(|| {
            RuntimeError::InvalidRuntimeTable(format!("coordinate system {id} missing"))
        })
    }
}

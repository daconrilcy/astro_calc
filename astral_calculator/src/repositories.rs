use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::catalog::BasicPayloadCatalog;
use crate::domain::{
    AccidentalConditionTrigger, AccidentalPolarityBand, AccidentalScoringParams,
    BasicProductScoringProfile, EssentialDignityRuleReference,
};
use crate::domain::{
    AspectFact, BasicPayload, CalculatedChartFacts, HouseCuspFact, InterpretationSignalDraft,
    NatalChartInput, ObjectPositionFact, RuntimeOptions,
};
use crate::models::{
    AccidentalConditionTriggerRow, AccidentalDignityConditionReferenceRow,
    AccidentalPolarityBandRow, AccidentalScoringParamsRow, AnglePointReference, AspectDefinition,
    AstralTimePeriodProfileRow, BasicProductScoringProfileRow, ChartCalculationRow, ChartObject,
    DomicileRulerReference, EssentialDignityRuleReferenceRow, HorizonPositionReference,
    HoroscopeOrbWeightBandRow, HoroscopeScanProfileRow, HoroscopeServiceRow,
    HoroscopeTimeSlotProfileRow, HouseAxisReferenceRow, HouseReference, HouseSystem,
    InterpretationSignalRow, LlmProjectionProfileRow, LunarPhaseReferenceRow,
    MajorAspectFamilyReference, MotionStateReference, ObjectSectAffinityReferenceRow,
    PersistedAspectFact, PersistedObjectPositionFact, SignReference,
};
use crate::runtime::RuntimeError;

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

    pub async fn basic_payload_catalog(
        &self,
        product_code: &str,
        payload_contract_version: &str,
        reference_version_id: i32,
    ) -> Result<BasicPayloadCatalog, RuntimeError> {
        let product_scoring = self
            .basic_product_scoring_profile(product_code, payload_contract_version)
            .await?;
        let essential_dignity_rules = self
            .essential_dignity_rule_references(
                reference_version_id,
                product_scoring.essential_dignity_score_profile_id,
            )
            .await?;
        let accidental_triggers = self.accidental_condition_triggers().await?;
        let accidental_scoring = self
            .accidental_scoring_params(product_scoring.accidental_scoring_params_id)
            .await?;
        let accidental_polarity_bands = self
            .accidental_overall_polarity_bands(product_scoring.accidental_scoring_params_id)
            .await?;
        Ok(BasicPayloadCatalog::build(
            product_scoring,
            essential_dignity_rules,
            accidental_triggers,
            accidental_scoring,
            accidental_polarity_bands,
        ))
    }

    pub async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseSystem>(
            r#"
            SELECT id, code, name, calculation_engine_code
            FROM astral_house_systems
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn house_systems(&self) -> Result<Vec<HouseSystem>, RuntimeError> {
        Ok(sqlx::query_as::<_, HouseSystem>(
            r#"
            SELECT id, code, name, calculation_engine_code
            FROM astral_house_systems
            WHERE is_active = true
            ORDER BY id
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn horoscope_services(&self) -> Result<Vec<HoroscopeServiceRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeServiceRow>(
            r#"
            SELECT service_code,
                   product_level_code,
                   shortlist_profile_code,
                   time_slot_profile_code,
                   slot_mode,
                   requires_natal_chart,
                   requires_location,
                   requires_timezone,
                   requires_inline_birth_data,
                   house_system_code,
                   period_profile_code,
                   detail_profile_code,
                   scan_profile_code,
                   detail_level,
                   generation_mode,
                   max_words_target,
                   max_words_hard_limit
            FROM horoscope_services
            ORDER BY service_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfileRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeTimeSlotProfileRow>(
            r#"
            SELECT service_code,
                   slot_code,
                   start_local_time,
                   end_local_time,
                   reference_local_time,
                   slot_label,
                   is_public,
                   sort_order
            FROM horoscope_time_slot_profiles
            ORDER BY service_code, sort_order, slot_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<AstralTimePeriodProfileRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, AstralTimePeriodProfileRow>(
            r#"
            SELECT period_profile_code,
                   resolution_strategy,
                   duration_days,
                   week_offset,
                   included_days,
                   is_enabled,
                   sort_order
            FROM astral_time_period_profiles
            ORDER BY sort_order, period_profile_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeScanProfileRow>(
            r#"
            SELECT scan_profile_code,
                   granularity,
                   reference_time_local,
                   expected_snapshots_per_day,
                   is_enabled,
                   sort_order
            FROM horoscope_scan_profiles
            ORDER BY sort_order, scan_profile_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn horoscope_orb_weight_bands(
        &self,
    ) -> Result<Vec<HoroscopeOrbWeightBandRow>, RuntimeError> {
        Ok(sqlx::query_as::<_, HoroscopeOrbWeightBandRow>(
            r#"
            SELECT band_code,
                   min_orb_deg::float8 AS min_orb_deg,
                   max_orb_deg::float8 AS max_orb_deg,
                   weight::float8 AS weight
            FROM horoscope_orb_weight_bands
            ORDER BY min_orb_deg, band_code
            "#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

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

    pub async fn zodiacal_reference_systems(
        &self,
    ) -> Result<Vec<crate::models::ZodiacalReferenceSystemRow>, RuntimeError> {
        Ok(
            sqlx::query_as::<_, crate::models::ZodiacalReferenceSystemRow>(
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

    pub async fn coordinate_reference_systems(
        &self,
    ) -> Result<Vec<crate::models::CoordinateReferenceSystemRow>, RuntimeError> {
        Ok(
            sqlx::query_as::<_, crate::models::CoordinateReferenceSystemRow>(
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

    pub async fn default_reference_version_id(&self) -> Result<i32, RuntimeError> {
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM astral_reference_versions
            WHERE status IN ('published', 'draft')
            ORDER BY CASE status WHEN 'published' THEN 0 ELSE 1 END, id ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        id.ok_or_else(|| {
            RuntimeError::InvalidRuntimeTable("no active astral_reference_versions row".to_string())
        })
    }

    pub async fn llm_projection_profile(
        &self,
        contract_version: &str,
        level_code: &str,
    ) -> Result<crate::llm_projection::LlmProjectionProfile, RuntimeError> {
        let row = sqlx::query_as::<_, LlmProjectionProfileRow>(
            r#"
            SELECT id,
                   contract_version,
                   level_code,
                   max_keywords_per_item,
                   max_core_placements,
                   max_supporting_placements,
                   max_dominant_signs,
                   max_dominant_houses,
                   max_dominant_objects,
                   max_house_axes,
                   max_aspects,
                   include_accidental_conditions,
                   include_rulership_details,
                   include_minor_evidence,
                   include_degrees,
                   include_scores
            FROM astral_llm_projection_profiles
            WHERE contract_version = $1
              AND level_code = $2
              AND is_active = true
            "#,
        )
        .bind(contract_version)
        .bind(level_code)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Into::into).ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(format!(
                "unknown llm projection profile: {contract_version}/{level_code}"
            ))
        })
    }

    pub async fn language_id_for_code(&self, code: &str) -> Result<i32, RuntimeError> {
        Ok(sqlx::query_scalar::<_, i32>(
            r#"
            SELECT id
            FROM languages
            WHERE code = $1
            "#,
        )
        .bind(code)
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
              AND status = 'completed'
            "#,
        )
        .bind(chart_calculation_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(value) = row else {
            return Err(RuntimeError::InvalidEngineRequest(
                "completed natal calculation input not found".to_string(),
            ));
        };
        Ok(serde_json::from_value(value)?)
    }

    pub async fn active_signals(
        &self,
        chart_calculation_id: i32,
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
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
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
    }

    pub async fn aspects_for_payload_in_tx(
        tx: &mut Transaction<'_, Postgres>,
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
        .fetch_all(&mut **tx)
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
                    signal_type_id = EXCLUDED.signal_type_id,
                    theme_code = EXCLUDED.theme_code,
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

pub fn parse_existing_basic_payload_value(
    value: Value,
) -> Result<Option<BasicPayload>, RuntimeError> {
    match serde_json::from_value(value) {
        Ok(payload) => Ok(Some(payload)),
        Err(error) if is_stale_basic_payload_shape(&error) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn is_stale_basic_payload_shape(error: &serde_json::Error) -> bool {
    error.is_data() && error.to_string().contains("missing field")
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

impl From<HouseAxisReferenceRow> for crate::domain::HouseAxisReference {
    fn from(row: HouseAxisReferenceRow) -> Self {
        Self {
            axis_code: row.axis_code,
            house_a_number: row.house_a_number,
            house_b_number: row.house_b_number,
            theme_a_code: row.theme_a_code,
            theme_b_code: row.theme_b_code,
            label: row.label,
            description: row.description,
        }
    }
}

impl From<LunarPhaseReferenceRow> for crate::domain::LunarPhaseReference {
    fn from(row: LunarPhaseReferenceRow) -> Self {
        Self {
            phase_code: row.phase_code,
            label: row.label,
            cycle_family: row.cycle_family,
            range_start_deg: row.range_start_deg,
            range_end_deg: row.range_end_deg,
            exact_anchor_deg: row.exact_anchor_deg,
            is_major_lunar_phase: row.is_major_lunar_phase,
            description: row.description,
        }
    }
}

impl From<AccidentalDignityConditionReferenceRow>
    for crate::domain::AccidentalDignityConditionReference
{
    fn from(row: AccidentalDignityConditionReferenceRow) -> Self {
        Self {
            condition_code: row.condition_code,
            condition_family: row.condition_family,
            label: row.label,
            polarity: row.polarity,
            strength_score: row.strength_score,
            score_delta: row.score_delta,
            description: row.description,
        }
    }
}

impl From<ObjectSectAffinityReferenceRow> for crate::domain::ObjectSectAffinityReference {
    fn from(row: ObjectSectAffinityReferenceRow) -> Self {
        Self {
            object_code: row.object_code,
            sect_affinity_code: row.sect_affinity_code,
            is_variable: row.is_variable,
            description: row.description,
        }
    }
}

impl From<LlmProjectionProfileRow> for crate::llm_projection::LlmProjectionProfile {
    fn from(row: LlmProjectionProfileRow) -> Self {
        let level_code = row.level_code.clone();
        Self {
            contract_version: row.contract_version,
            level_code: row.level_code,
            max_keywords_per_item: row.max_keywords_per_item as usize,
            max_core_placements: row.max_core_placements as usize,
            max_supporting_placements: row.max_supporting_placements as usize,
            max_dominant_signs: row.max_dominant_signs as usize,
            max_dominant_houses: row.max_dominant_houses as usize,
            max_dominant_objects: row.max_dominant_objects as usize,
            max_house_axes: row.max_house_axes as usize,
            max_aspects: row.max_aspects as usize,
            max_background_placements: crate::llm_projection::default_max_background_placements(
                &level_code,
            ),
            max_accidental_conditions_per_object:
                crate::llm_projection::default_max_accidental_conditions(&level_code),
            include_accidental_conditions: row.include_accidental_conditions,
            include_rulership_details: row.include_rulership_details,
            include_minor_evidence: row.include_minor_evidence,
            include_degrees: row.include_degrees,
            include_scores: row.include_scores,
        }
    }
}

impl From<BasicProductScoringProfileRow> for BasicProductScoringProfile {
    fn from(row: BasicProductScoringProfileRow) -> Self {
        Self {
            product_code: row.product_code,
            payload_contract_version: row.payload_contract_version,
            essential_dignity_score_profile_id: row.essential_dignity_score_profile_id,
            accidental_scoring_params_id: row.accidental_scoring_params_id,
            default_major_orb_deg: row.default_major_orb_deg,
            sign_emphasis_full_score: row.sign_emphasis_full_score,
            house_emphasis_full_score: row.house_emphasis_full_score,
            object_emphasis_full_score: row.object_emphasis_full_score,
            sign_house_emphasis_min_score: row.sign_house_emphasis_min_score,
            object_emphasis_min_score: row.object_emphasis_min_score,
            house_axis_full_score: row.house_axis_full_score,
            axis_min_score: row.axis_min_score,
            axis_secondary_weight: row.axis_secondary_weight,
            axis_polarity_dominance_delta: row.axis_polarity_dominance_delta,
            axis_balanced_min_score: row.axis_balanced_min_score,
            max_dominant_signs: row.max_dominant_signs as usize,
            max_dominant_houses: row.max_dominant_houses as usize,
            max_dominant_objects: row.max_dominant_objects as usize,
            max_active_signals: row.max_active_signals as usize,
            aspect_min_strength: row.aspect_min_strength,
            max_house_axis_emphasis: row.max_house_axis_emphasis as usize,
        }
    }
}

impl From<EssentialDignityRuleReferenceRow> for EssentialDignityRuleReference {
    fn from(row: EssentialDignityRuleReferenceRow) -> Self {
        Self {
            object_code: row.object_code,
            sign_code: row.sign_code,
            dignity_type: row.dignity_type,
            dignity_label: row.dignity_label,
            polarity: row.polarity,
            strength_score: row.strength_score,
            priority_delta: row.priority_delta.unwrap_or(0.0),
            signal_weight_delta: row.signal_weight_delta.unwrap_or(0.0),
            signal_worthy_min_strength: row.signal_worthy_min_strength.unwrap_or(0.7),
            emphasis_weight: row.emphasis_weight.unwrap_or(0.25),
        }
    }
}

impl From<AccidentalConditionTriggerRow> for AccidentalConditionTrigger {
    fn from(row: AccidentalConditionTriggerRow) -> Self {
        Self {
            trigger_family: row.trigger_family,
            source_code: row.source_code,
            angle_object_code: row.angle_object_code,
            condition_code: row.condition_code,
        }
    }
}

impl From<AccidentalScoringParamsRow> for AccidentalScoringParams {
    fn from(row: AccidentalScoringParamsRow) -> Self {
        Self {
            code: row.code,
            overall_score_baseline: row.overall_score_baseline,
            overall_score_min: row.overall_score_min,
            overall_score_max: row.overall_score_max,
            angle_proximity_max_orb_deg: row.angle_proximity_max_orb_deg,
        }
    }
}

impl From<AccidentalPolarityBandRow> for AccidentalPolarityBand {
    fn from(row: AccidentalPolarityBandRow) -> Self {
        Self {
            polarity_code: row.polarity_code,
            expression_quality_code: row.expression_quality_code,
            min_score: row.min_score,
            max_score: row.max_score,
            sort_order: row.sort_order,
        }
    }
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
            aspect_family: row.aspect_family,
            orb_deg: row.orb_deg,
            phase_state: row.phase_state,
            is_applying: row.is_applying,
            is_exact: row.is_exact,
            strength_score: row.strength_score,
            primary_valence: row.primary_valence,
            intensity_modifier: row.intensity_modifier,
            secondary_effect: row.secondary_effect,
            valence_family: row.valence_family,
            valence_is_tonal: row.valence_is_tonal,
            valence_is_intensity_modifier: row.valence_is_intensity_modifier,
            calculation_notes_json: row.calculation_notes_json,
        }
    }
}

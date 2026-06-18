//! Conversions row SQL vers types domaine.

use super::*;

impl From<PersistedObjectPositionFact> for ObjectPositionFact {
    /// Fonction from.
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
    /// Fonction from.
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
    /// Fonction from.
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
    /// Fonction from.
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
    /// Fonction from.
    fn from(row: ObjectSectAffinityReferenceRow) -> Self {
        Self {
            object_code: row.object_code,
            sect_affinity_code: row.sect_affinity_code,
            is_variable: row.is_variable,
            description: row.description,
        }
    }
}

impl From<LlmProjectionProfileRow> for crate::engine::projection::LlmProjectionProfile {
    /// Fonction from.
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
            max_background_placements: crate::engine::projection::default_max_background_placements(
                &level_code,
            ),
            max_accidental_conditions_per_object:
                crate::engine::projection::default_max_accidental_conditions(&level_code),
            include_accidental_conditions: row.include_accidental_conditions,
            include_rulership_details: row.include_rulership_details,
            include_minor_evidence: row.include_minor_evidence,
            include_degrees: row.include_degrees,
            include_scores: row.include_scores,
        }
    }
}

impl From<BasicProductScoringProfileRow> for BasicProductScoringProfile {
    /// Fonction from.
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
    /// Fonction from.
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
    /// Fonction from.
    fn from(row: AccidentalConditionTriggerRow) -> Self {
        Self {
            trigger_family: row.trigger_family,
            source_code: row.source_code,
            angle_object_code: row.angle_object_code,
            condition_code: row.condition_code,
        }
    }
}

impl From<ProjectionReasonDefinitionRow> for crate::domain::ProjectionReasonDefinition {
    /// Fonction from.
    fn from(row: ProjectionReasonDefinitionRow) -> Self {
        Self {
            reason_code: row.reason_code,
            reason_family: row.reason_family,
            label_template_en: row.label_template_en,
            requires_object: row.requires_object,
            requires_dignity_type: row.requires_dignity_type,
            requires_sign_code: row.requires_sign_code,
            requires_house_number: row.requires_house_number,
            requires_theme_code: row.requires_theme_code,
            requires_angle_code: row.requires_angle_code,
            requires_signal_key: row.requires_signal_key,
            requires_context_key: row.requires_context_key,
            is_active: row.is_active,
            sort_order: row.sort_order,
        }
    }
}

impl From<ProjectionLabelDefinitionRow> for crate::domain::ProjectionLabelDefinition {
    /// Fonction from.
    fn from(row: ProjectionLabelDefinitionRow) -> Self {
        Self {
            label_family: row.label_family,
            label_code: row.label_code,
            label_template_en: row.label_template_en,
            is_active: row.is_active,
            sort_order: row.sort_order,
        }
    }
}

impl From<AccidentalScoringParamsRow> for AccidentalScoringParams {
    /// Fonction from.
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
    /// Fonction from.
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
    /// Fonction from.
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

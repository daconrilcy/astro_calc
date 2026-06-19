mod common;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::json;

use astral_calculator::application::ports::{
    CalculationAttempt, CalculationAttemptStore, CalculationFactStore,
    CalculationTransactionManager, LocalizationCatalog, MajorAspectFamilyReference,
    NatalReferenceStore, PayloadCatalogStore, PayloadStore, ReferenceSystemResolver, SignalStore,
};
use astral_calculator::astrology::ephemeris::EphemerisEngine;
use astral_calculator::domain::{
    AccidentalDignityConditionReference, AspectDefinition, AspectFact, BasicPayload,
    CalculatedChartFacts, CalculationReferenceData, ChartObject, DomicileRulerReference,
    HouseAxisReference, HouseReference, HouseSystem, InterpretationSignalDraft,
    InterpretationSignalRow, LunarPhaseReference, NatalChartInput, ObjectPositionFact,
    ObjectSectAffinityReference, RuntimeOptions, SignReference,
};
use astral_calculator::features::natal::application::NatalCalculationService;
use astral_calculator::features::natal::payload::validate::{
    has_current_rulership_references, is_current_basic_payload,
};
use astral_calculator::runtime::RuntimeError;
use common::json_db::{
    major_aspect_definitions_from_json_db_seed,
    major_aspect_family_expected_count_from_json_db_seed,
    major_aspect_family_max_default_orb_deg_from_json_db_seed,
};

#[derive(Clone, Default)]
struct FakeCalculationStore {
    state: Arc<Mutex<FakeCalculationState>>,
}

#[derive(Default)]
struct FakeCalculationState {
    existing: Vec<CalculationAttempt>,
    existing_payload: Option<BasicPayload>,
    positions: Vec<ObjectPositionFact>,
    aspects: Vec<AspectFact>,
    stale_marked: Vec<i32>,
    persist_signals_calls: usize,
    next_signal_error: Option<RuntimeError>,
}

#[async_trait]
impl CalculationTransactionManager for FakeCalculationStore {
    type Tx = i32;

    async fn begin(&self) -> Result<Self::Tx, RuntimeError> {
        Ok(1)
    }

    async fn commit(&self, _tx: Self::Tx) -> Result<(), RuntimeError> {
        Ok(())
    }

    async fn lock_idempotency(
        &self,
        _tx: &mut Self::Tx,
        _lock_key: i64,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[async_trait]
impl CalculationAttemptStore for FakeCalculationStore {
    async fn calculations_for_key(
        &self,
        _tx: &mut Self::Tx,
        _idempotency_key: &str,
    ) -> Result<Vec<CalculationAttempt>, RuntimeError> {
        Ok(self.state.lock().expect("state").existing.clone())
    }

    async fn mark_stale_failed(
        &self,
        _tx: &mut Self::Tx,
        chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        self.state
            .lock()
            .expect("state")
            .stale_marked
            .push(chart_calculation_id);
        Ok(())
    }

    async fn insert_running_calculation(
        &self,
        _tx: &mut Self::Tx,
        _input: &NatalChartInput,
        _options: &RuntimeOptions,
        _input_hash: &str,
        _idempotency_key: &str,
        _next_attempt: i32,
    ) -> Result<i32, RuntimeError> {
        Ok(99)
    }

    async fn heartbeat(
        &self,
        _tx: &mut Self::Tx,
        _chart_calculation_id: i32,
        _progress_state: &str,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    async fn mark_failed(
        &self,
        _tx: &mut Self::Tx,
        _chart_calculation_id: i32,
        _error: &RuntimeError,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    async fn mark_completed(
        &self,
        _tx: &mut Self::Tx,
        _chart_calculation_id: i32,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[async_trait]
impl CalculationFactStore for FakeCalculationStore {
    async fn positions_for_payload(
        &self,
        _chart_calculation_id: i32,
    ) -> Result<Vec<ObjectPositionFact>, RuntimeError> {
        Ok(self.state.lock().expect("state").positions.clone())
    }

    async fn aspects_for_payload(
        &self,
        _chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        Ok(self.state.lock().expect("state").aspects.clone())
    }

    async fn natal_input_for_calculation(
        &self,
        _chart_calculation_id: i32,
    ) -> Result<NatalChartInput, RuntimeError> {
        Err(RuntimeError::InvalidRuntimeTable(
            "not used in test".to_string(),
        ))
    }

    async fn persist_facts(
        &self,
        _tx: &mut Self::Tx,
        _chart_calculation_id: i32,
        _facts: &CalculatedChartFacts,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    async fn aspects_for_payload_in_tx(
        &self,
        _tx: &mut Self::Tx,
        _chart_calculation_id: i32,
    ) -> Result<Vec<AspectFact>, RuntimeError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl PayloadStore for FakeCalculationStore {
    async fn existing_basic_payload(
        &self,
        _chart_calculation_id: i32,
        _product_code: &str,
        _language_id: Option<i32>,
    ) -> Result<Option<BasicPayload>, RuntimeError> {
        Ok(self.state.lock().expect("state").existing_payload.clone())
    }

    async fn persist_basic_payload(
        &self,
        _tx: &mut Self::Tx,
        _input: &NatalChartInput,
        _payload_language_id: Option<i32>,
        _payload: &BasicPayload,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[async_trait]
impl SignalStore for FakeCalculationStore {
    async fn persist_signals(
        &self,
        _tx: &mut Self::Tx,
        _chart_calculation_id: i32,
        _reference_version_id: i32,
        _signals: &[InterpretationSignalDraft],
    ) -> Result<Vec<InterpretationSignalRow>, RuntimeError> {
        let mut state = self.state.lock().expect("state");
        state.persist_signals_calls += 1;
        if let Some(error) = state.next_signal_error.take() {
            return Err(error);
        }
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct FakeCatalogStore;

#[async_trait]
impl PayloadCatalogStore for FakeCatalogStore {
    async fn basic_payload_catalog(
        &self,
        _product_code: &str,
        _payload_contract_version: &str,
        _reference_version_id: i32,
    ) -> Result<astral_calculator::domain::BasicPayloadCatalog, RuntimeError> {
        Ok(astral_calculator::catalog::test_catalog())
    }

    async fn basic_product_scoring_profile(
        &self,
        _product_code: &str,
        _payload_contract_version: &str,
    ) -> Result<astral_calculator::domain::BasicProductScoringProfile, RuntimeError> {
        Ok(astral_calculator::catalog::test_catalog().product_scoring)
    }

    async fn essential_dignity_rule_references(
        &self,
        _reference_version_id: i32,
        _score_profile_id: i32,
    ) -> Result<Vec<astral_calculator::domain::EssentialDignityRuleReference>, RuntimeError> {
        Ok(astral_calculator::catalog::test_catalog().essential_dignity_rules)
    }

    async fn projection_reason_definitions(
        &self,
    ) -> Result<Vec<astral_calculator::domain::ProjectionReasonDefinition>, RuntimeError> {
        Ok(astral_calculator::catalog::test_catalog().projection_reason_definitions)
    }

    async fn projection_label_definitions(
        &self,
    ) -> Result<Vec<astral_calculator::domain::ProjectionLabelDefinition>, RuntimeError> {
        Ok(astral_calculator::catalog::test_catalog().projection_label_definitions)
    }
}

#[derive(Clone)]
struct FakeReferenceStore {
    domicile_rulers: Vec<DomicileRulerReference>,
}

#[async_trait]
impl ReferenceSystemResolver for FakeReferenceStore {
    async fn zodiacal_reference_system_id_by_key(&self, _key: &str) -> Result<i32, RuntimeError> {
        Ok(1)
    }

    async fn coordinate_reference_system_id_by_key(&self, _key: &str) -> Result<i32, RuntimeError> {
        Ok(1)
    }

    async fn house_system_id_by_code(&self, _code: &str) -> Result<i32, RuntimeError> {
        Ok(1)
    }

    async fn zodiacal_reference_system_display_name(
        &self,
        _id: i32,
    ) -> Result<String, RuntimeError> {
        Ok("Tropical".to_string())
    }

    async fn coordinate_reference_system_display_name(
        &self,
        _id: i32,
    ) -> Result<String, RuntimeError> {
        Ok("Geocentric".to_string())
    }

    async fn house_system(&self, _id: i32) -> Result<HouseSystem, RuntimeError> {
        Ok(HouseSystem {
            id: 1,
            code: "placidus".to_string(),
            name: "Placidus".to_string(),
            calculation_engine_code: "placidus".to_string(),
        })
    }
}

#[async_trait]
impl NatalReferenceStore for FakeReferenceStore {
    async fn default_reference_version_id(&self) -> Result<i32, RuntimeError> {
        Ok(1)
    }

    async fn active_chart_objects(
        &self,
        _reference_version_id: i32,
    ) -> Result<Vec<ChartObject>, RuntimeError> {
        Ok(vec![
            chart_object(11, "ascendant", Some("angle"), true),
            chart_object(12, "descendant", Some("angle"), true),
            chart_object(13, "midheaven", Some("angle"), true),
            chart_object(14, "imum_coeli", Some("angle"), true),
            chart_object(5, "mars", Some("planet"), false),
        ])
    }

    async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError> {
        Ok(major_aspect_definitions_from_json_db_seed())
    }

    async fn major_aspect_family_reference(
        &self,
    ) -> Result<MajorAspectFamilyReference, RuntimeError> {
        Ok(MajorAspectFamilyReference {
            expected_aspect_count: major_aspect_family_expected_count_from_json_db_seed() as i32,
            max_default_orb_deg: major_aspect_family_max_default_orb_deg_from_json_db_seed(),
        })
    }

    async fn sign_references(&self) -> Result<Vec<SignReference>, RuntimeError> {
        Ok((1..=12)
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
            .collect())
    }

    async fn house_references(&self) -> Result<Vec<HouseReference>, RuntimeError> {
        Ok((1..=12)
            .map(|number| HouseReference {
                id: number + 100,
                number,
                name: format!("House {number}"),
                theme_code: format!("house_{number}_theme"),
                modality_code: Some("angular".to_string()),
                modality_label: Some("Angular".to_string()),
                accidental_strength: Some("strong".to_string()),
                modality_priority_delta: Some(2.0),
                interpretation_weight: Some("high".to_string()),
            })
            .collect())
    }

    async fn motion_state_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::MotionStateReference>, RuntimeError> {
        Ok(vec![astral_calculator::domain::MotionStateReference {
            id: 1,
            code: "direct".to_string(),
            label: "Direct".to_string(),
            motion_family: "forward".to_string(),
        }])
    }

    async fn horizon_position_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::HorizonPositionReference>, RuntimeError> {
        Ok(vec![
            horizon_position(1, "above_horizon"),
            horizon_position(2, "below_horizon"),
            horizon_position(3, "on_horizon"),
        ])
    }

    async fn angle_point_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::AnglePointReference>, RuntimeError> {
        Ok(vec![
            angle_reference(1, "asc", Some("dsc"), 1, 11),
            angle_reference(2, "dsc", Some("asc"), 7, 12),
            angle_reference(3, "mc", Some("ic"), 10, 13),
            angle_reference(4, "ic", Some("mc"), 4, 14),
        ])
    }

    async fn domicile_ruler_references(
        &self,
        _reference_version_id: i32,
    ) -> Result<Vec<DomicileRulerReference>, RuntimeError> {
        Ok(self.domicile_rulers.clone())
    }

    async fn house_axis_references(&self) -> Result<Vec<HouseAxisReference>, RuntimeError> {
        Ok((1..=6)
            .map(|house_a| HouseAxisReference {
                axis_code: format!("axis_{house_a}_{}", house_a + 6),
                house_a_number: house_a,
                house_b_number: house_a + 6,
                theme_a_code: format!("house_{house_a}_theme"),
                theme_b_code: format!("house_{}_theme", house_a + 6),
                label: format!("Axis {house_a}/{}", house_a + 6),
                description: "Axis description".to_string(),
            })
            .collect())
    }

    async fn lunar_phase_references(&self) -> Result<Vec<LunarPhaseReference>, RuntimeError> {
        Ok(vec![
            lunar_phase(
                "new_moon",
                "New Moon",
                "conjunction",
                337.5,
                22.5,
                0.0,
                true,
            ),
            lunar_phase(
                "waxing_crescent",
                "Waxing Crescent",
                "waxing",
                22.5,
                67.5,
                45.0,
                false,
            ),
            lunar_phase(
                "first_quarter",
                "First Quarter",
                "waxing",
                67.5,
                112.5,
                90.0,
                true,
            ),
            lunar_phase(
                "waxing_gibbous",
                "Waxing Gibbous",
                "waxing",
                112.5,
                157.5,
                135.0,
                false,
            ),
            lunar_phase(
                "full_moon",
                "Full Moon",
                "opposition",
                157.5,
                202.5,
                180.0,
                true,
            ),
            lunar_phase(
                "waning_gibbous",
                "Waning Gibbous",
                "waning",
                202.5,
                247.5,
                225.0,
                false,
            ),
            lunar_phase(
                "last_quarter",
                "Last Quarter",
                "waning",
                247.5,
                292.5,
                270.0,
                true,
            ),
            lunar_phase(
                "waning_crescent",
                "Waning Crescent",
                "waning",
                292.5,
                337.5,
                315.0,
                false,
            ),
        ])
    }

    async fn accidental_dignity_condition_references(
        &self,
    ) -> Result<Vec<AccidentalDignityConditionReference>, RuntimeError> {
        Ok(vec![
            accidental_condition("angular_house", "house_modality", "dignity", 0.75, 0.25),
            accidental_condition("near_ascendant", "angle_proximity", "dignity", 0.8, 0.2),
            accidental_condition("sect_affinity_match", "sect_match", "contextual", 0.5, 0.1),
        ])
    }

    async fn object_sect_affinity_references(
        &self,
    ) -> Result<Vec<ObjectSectAffinityReference>, RuntimeError> {
        Ok(vec![ObjectSectAffinityReference {
            object_code: "mars".to_string(),
            sect_affinity_code: "night".to_string(),
            is_variable: false,
            description: "mars sect affinity".to_string(),
        }])
    }
}

#[async_trait]
impl LocalizationCatalog for FakeReferenceStore {
    async fn language_id_for_code(&self, _code: &str) -> Result<i32, RuntimeError> {
        Ok(1)
    }
}

#[derive(Clone, Default)]
struct FakeEphemeris;

impl EphemerisEngine for FakeEphemeris {
    fn calculate_chart(
        &self,
        _input: &NatalChartInput,
        _chart_objects: &[ChartObject],
        _aspects: &[AspectDefinition],
        _house_system: &HouseSystem,
        _references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        Err(RuntimeError::InvalidRuntimeTable(
            "ephemeris should not be used in reuse-policy tests".to_string(),
        ))
    }
}

fn chart_object(id: i32, code: &str, role_code: Option<&str>, is_angle: bool) -> ChartObject {
    ChartObject {
        id,
        code: code.to_string(),
        name: code.to_string(),
        swe_id: None,
        role_code: role_code.map(str::to_string),
        role_label: role_code.map(|role| {
            if role == "angle" {
                "Angle".to_string()
            } else {
                "Planet".to_string()
            }
        }),
        is_luminary: Some(false),
        is_planet_symbolic: Some(!is_angle),
        is_visible_to_naked_eye: Some(true),
        nature_codes: Some(json!(["test"])),
        position_priority_base: Some(1.0),
        angle_priority_base: Some(if is_angle { 1.0 } else { 0.0 }),
        source_weight: Some(1.0),
    }
}

fn horizon_position(id: i32, code: &str) -> astral_calculator::domain::HorizonPositionReference {
    astral_calculator::domain::HorizonPositionReference {
        id,
        code: code.to_string(),
        label: code.to_string(),
    }
}

fn angle_reference(
    id: i32,
    code: &str,
    opposite_angle_code: Option<&str>,
    associated_house: i32,
    chart_object_id: i32,
) -> astral_calculator::domain::AnglePointReference {
    astral_calculator::domain::AnglePointReference {
        id,
        code: code.to_string(),
        short_label: code.to_uppercase(),
        full_name: code.to_string(),
        axis: if matches!(code, "asc" | "dsc") {
            "horizontal".to_string()
        } else {
            "vertical".to_string()
        },
        opposite_angle_code: opposite_angle_code.map(str::to_string),
        associated_house,
        description: code.to_string(),
        chart_object_id,
        chart_object_code: code.to_string(),
        chart_object_name: code.to_string(),
        chart_object_sort_order: id,
    }
}

fn lunar_phase(
    phase_code: &str,
    label: &str,
    cycle_family: &str,
    range_start_deg: f64,
    range_end_deg: f64,
    exact_anchor_deg: f64,
    is_major_lunar_phase: bool,
) -> LunarPhaseReference {
    LunarPhaseReference {
        phase_code: phase_code.to_string(),
        label: label.to_string(),
        cycle_family: cycle_family.to_string(),
        range_start_deg,
        range_end_deg,
        exact_anchor_deg,
        is_major_lunar_phase,
        description: format!("{label} description"),
    }
}

fn accidental_condition(
    condition_code: &str,
    condition_family: &str,
    polarity: &str,
    strength_score: f64,
    score_delta: f64,
) -> AccidentalDignityConditionReference {
    AccidentalDignityConditionReference {
        condition_code: condition_code.to_string(),
        condition_family: condition_family.to_string(),
        label: condition_code.to_string(),
        polarity: polarity.to_string(),
        strength_score,
        score_delta,
        description: format!("{condition_code} description"),
    }
}

fn angle_position(
    chart_object_id: i32,
    object_code: &str,
    angle_point_code: &str,
    horizon_position_id: i32,
) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_code.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: Some(1),
        house_number: Some(1),
        house_name: Some("House 1".to_string()),
        motion_state_id: None,
        horizon_position_id: Some(horizon_position_id),
        longitude_deg: 10.0,
        latitude_deg: None,
        apparent_speed_deg_per_day: None,
        altitude_deg: None,
        is_visible: Some(true),
        facts_json: Some(json!({
            "object_context": { "role": "angle", "role_label": "Angle" },
            "angle_context": { "angle_point_code": angle_point_code }
        })),
    }
}

fn mobile_position(chart_object_id: i32, object_code: &str) -> ObjectPositionFact {
    ObjectPositionFact {
        chart_object_id,
        object_code: object_code.to_string(),
        object_name: object_code.to_string(),
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        sign_id: 1,
        sign_code: "aries".to_string(),
        sign_name: "Aries".to_string(),
        house_id: Some(1),
        house_number: Some(1),
        house_name: Some("House 1".to_string()),
        motion_state_id: Some(1),
        horizon_position_id: Some(1),
        longitude_deg: 12.0,
        latitude_deg: Some(0.0),
        apparent_speed_deg_per_day: Some(1.0),
        altitude_deg: Some(10.0),
        is_visible: Some(true),
        facts_json: Some(json!({
            "object_context": {
                "role": "planet",
                "role_label": "Planet",
                "signal_scoring": {
                    "position_priority_base": 1.0,
                    "angle_priority_base": 0.0,
                    "source_weight": 1.0
                }
            }
        })),
    }
}

fn reusable_positions() -> Vec<ObjectPositionFact> {
    vec![
        angle_position(11, "ascendant", "asc", 3),
        angle_position(12, "descendant", "dsc", 3),
        angle_position(13, "midheaven", "mc", 1),
        angle_position(14, "imum_coeli", "ic", 2),
        mobile_position(5, "mars"),
    ]
}

fn test_input() -> NatalChartInput {
    NatalChartInput {
        subject_label: Some("Test".to_string()),
        birth_datetime_utc: Utc::now(),
        latitude_deg: 48.8566,
        longitude_deg: 2.3522,
        altitude_m: None,
        reference_version_id: 1,
        calculation_profile_id: None,
        zodiacal_reference_system_id: 1,
        coordinate_reference_system_id: 1,
        house_system_id: 1,
        product_code: Some("basic".to_string()),
        client_idempotency_key: None,
    }
}

fn current_payload_fixture() -> BasicPayload {
    serde_json::from_str(include_str!("./golden/natal_payload_v14_paris_1990.json"))
        .expect("golden payload")
}

fn stale_payload_fixture() -> BasicPayload {
    let mut payload = current_payload_fixture();
    payload.chart_context.payload_contract.contract_version = "natal_structured_v13".to_string();
    payload
}

fn domicile_rulers_from_payload(payload: &BasicPayload) -> Vec<DomicileRulerReference> {
    let mut seen = std::collections::HashSet::new();
    let mut rulers = Vec::new();

    let mut push_sources =
        |sign_code: &str, sources: &[astral_calculator::domain::BasicRulerSource]| {
            for source in sources {
                let signature = (
                    sign_code.to_string(),
                    source.reference_version_id,
                    source.astral_system_id,
                    source.astral_system_code.clone(),
                    source.dignity_type.clone(),
                    source.object_code.clone(),
                    source.weight.to_bits(),
                    source.is_primary,
                );
                if !seen.insert(signature) {
                    continue;
                }
                rulers.push(DomicileRulerReference {
                    reference_version_id: source.reference_version_id,
                    astral_system_id: source.astral_system_id,
                    astral_system_code: source.astral_system_code.clone(),
                    sign_id: 1,
                    sign_code: sign_code.to_string(),
                    sign_name: sign_code.to_string(),
                    chart_object_id: 1,
                    object_code: source.object_code.clone(),
                    object_name: source.object_code.clone(),
                    dignity_type: source.dignity_type.clone(),
                    weight: source.weight,
                    is_primary: source.is_primary,
                });
            }
        };

    if let Some(context) = payload.rulership_context.ascendant_ruler.as_ref() {
        push_sources(&context.sign_code, &context.ruler_sources);
    }
    if let Some(context) = payload.rulership_context.descendant_ruler.as_ref() {
        push_sources(&context.sign_code, &context.ruler_sources);
    }
    if let Some(context) = payload.rulership_context.mc_ruler.as_ref() {
        push_sources(&context.sign_code, &context.ruler_sources);
    }
    for context in &payload.rulership_context.dominant_house_rulers {
        push_sources(&context.sign_code, &context.ruler_sources);
    }
    for context in &payload.rulership_context.dominant_sign_rulers {
        push_sources(&context.sign_code, &context.ruler_sources);
    }
    for link in &payload.rulership_context.dispositor_links {
        push_sources(&link.object_sign_code, &link.ruler_sources);
    }

    rulers
}

fn build_service(
    store: FakeCalculationStore,
    reference_store: FakeReferenceStore,
) -> NatalCalculationService<
    FakeCalculationStore,
    FakeCatalogStore,
    FakeReferenceStore,
    FakeEphemeris,
> {
    NatalCalculationService::new(
        store,
        FakeCatalogStore,
        reference_store,
        Arc::new(FakeEphemeris),
        RuntimeOptions::default(),
    )
}

#[tokio::test]
async fn natal_reuse_policy_reuses_current_payload() {
    let payload = current_payload_fixture();
    let store = FakeCalculationStore {
        state: Arc::new(Mutex::new(FakeCalculationState {
            existing: vec![CalculationAttempt {
                id: 42,
                status: "completed".to_string(),
                execution_attempt: 1,
                heartbeat_at: Some(Utc::now()),
                stale_after_seconds: Some(900),
            }],
            existing_payload: Some(payload.clone()),
            ..FakeCalculationState::default()
        })),
    };
    let reference_store = FakeReferenceStore {
        domicile_rulers: domicile_rulers_from_payload(&payload),
    };
    assert!(is_current_basic_payload(
        &payload,
        &astral_calculator::catalog::test_catalog().projection_reason_definitions
    ));
    assert!(has_current_rulership_references(
        &payload,
        &reference_store.domicile_rulers
    ));

    let service = build_service(store, reference_store);
    let (result, _) = service
        .calculate_basic_with_catalog(test_input())
        .await
        .expect("payload reuse");

    assert_eq!(result.chart_calculation_id, 27);
}

#[tokio::test]
async fn natal_reuse_policy_rebuilds_from_reusable_positions_when_payload_is_stale() {
    let store = FakeCalculationStore {
        state: Arc::new(Mutex::new(FakeCalculationState {
            existing: vec![CalculationAttempt {
                id: 42,
                status: "completed".to_string(),
                execution_attempt: 1,
                heartbeat_at: Some(Utc::now()),
                stale_after_seconds: Some(900),
            }],
            existing_payload: Some(stale_payload_fixture()),
            positions: reusable_positions(),
            next_signal_error: Some(RuntimeError::InvalidRuntimeTable(
                "persist_signals reached".to_string(),
            )),
            ..FakeCalculationState::default()
        })),
    };
    let reference_store = FakeReferenceStore {
        domicile_rulers: vec![DomicileRulerReference {
            reference_version_id: Some(1),
            astral_system_id: 1,
            astral_system_code: "traditional".to_string(),
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            chart_object_id: 5,
            object_code: "mars".to_string(),
            object_name: "Mars".to_string(),
            dignity_type: "domicile".to_string(),
            weight: 1.0,
            is_primary: true,
        }],
    };

    let service = build_service(store.clone(), reference_store);
    let error = service
        .calculate_basic_with_catalog(test_input())
        .await
        .expect_err("rebuild branch should attempt persist_signals");

    assert_eq!(error.code(), "invalid_runtime_table");
    assert_eq!(store.state.lock().expect("state").persist_signals_calls, 1);
}

#[tokio::test]
async fn natal_reuse_policy_rejects_non_stale_running_calculation() {
    let store = FakeCalculationStore {
        state: Arc::new(Mutex::new(FakeCalculationState {
            existing: vec![CalculationAttempt {
                id: 77,
                status: "running".to_string(),
                execution_attempt: 2,
                heartbeat_at: Some(Utc::now() - Duration::seconds(10)),
                stale_after_seconds: Some(900),
            }],
            ..FakeCalculationState::default()
        })),
    };
    let reference_store = FakeReferenceStore {
        domicile_rulers: vec![DomicileRulerReference {
            reference_version_id: Some(1),
            astral_system_id: 1,
            astral_system_code: "traditional".to_string(),
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            chart_object_id: 5,
            object_code: "mars".to_string(),
            object_name: "Mars".to_string(),
            dignity_type: "domicile".to_string(),
            weight: 1.0,
            is_primary: true,
        }],
    };

    let service = build_service(store, reference_store);
    let error = service
        .calculate_basic_with_catalog(test_input())
        .await
        .expect_err("running calculation should be rejected");

    match error {
        RuntimeError::RunningCalculationInProgress {
            idempotency_key,
            chart_calculation_id,
        } => {
            assert!(!idempotency_key.is_empty());
            assert_eq!(chart_calculation_id, 77);
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[tokio::test]
async fn natal_reuse_policy_marks_stale_running_calculation_failed() {
    let store = FakeCalculationStore {
        state: Arc::new(Mutex::new(FakeCalculationState {
            existing: vec![CalculationAttempt {
                id: 88,
                status: "running".to_string(),
                execution_attempt: 2,
                heartbeat_at: Some(Utc::now() - Duration::seconds(120)),
                stale_after_seconds: Some(30),
            }],
            next_signal_error: Some(RuntimeError::InvalidRuntimeTable(
                "persist_signals reached".to_string(),
            )),
            ..FakeCalculationState::default()
        })),
    };
    let reference_store = FakeReferenceStore {
        domicile_rulers: vec![DomicileRulerReference {
            reference_version_id: Some(1),
            astral_system_id: 1,
            astral_system_code: "traditional".to_string(),
            sign_id: 1,
            sign_code: "aries".to_string(),
            sign_name: "Aries".to_string(),
            chart_object_id: 5,
            object_code: "mars".to_string(),
            object_name: "Mars".to_string(),
            dignity_type: "domicile".to_string(),
            weight: 1.0,
            is_primary: true,
        }],
    };

    let service = build_service(store.clone(), reference_store);
    let error = service
        .calculate_basic_with_catalog(test_input())
        .await
        .expect_err("stale running should continue into the next branch");

    assert_eq!(error.code(), "invalid_runtime_table");
    assert_eq!(store.state.lock().expect("state").stale_marked, vec![88]);
}

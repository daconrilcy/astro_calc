use async_trait::async_trait;

use astral_calculator::application::calculation_references::{
    load_calculation_reference_data, load_default_calculation_reference_data,
};
use astral_calculator::application::chart_context::{
    load_chart_context, load_default_chart_context,
};
use astral_calculator::application::ports::{
    CalculationReferenceLoader, NatalReferenceStore, ReferenceSystemLookup,
    ReferenceSystemResolver, ReferenceVersionProvider,
};
use astral_calculator::domain::{
    AnglePointReference, AspectDefinition, ChartObject, HorizonPositionReference, HouseReference,
    HouseSystem, MotionStateReference, SignReference,
};
use astral_calculator::runtime::RuntimeError;

struct FakeCalculationReferenceRepository;

#[async_trait]
impl ReferenceSystemLookup for FakeCalculationReferenceRepository {
    async fn zodiacal_reference_system_id_by_key(&self, key: &str) -> Result<i32, RuntimeError> {
        match key {
            "tropical" => Ok(42),
            other => Err(RuntimeError::InvalidRuntimeTable(format!(
                "unexpected zodiac key {other}"
            ))),
        }
    }

    async fn coordinate_reference_system_id_by_key(&self, key: &str) -> Result<i32, RuntimeError> {
        match key {
            "geocentric" => Ok(84),
            other => Err(RuntimeError::InvalidRuntimeTable(format!(
                "unexpected coordinate key {other}"
            ))),
        }
    }

    async fn house_system_id_by_code(&self, code: &str) -> Result<i32, RuntimeError> {
        match code {
            "placidus" => Ok(12),
            other => Err(RuntimeError::InvalidRuntimeTable(format!(
                "unexpected house system lookup {other}"
            ))),
        }
    }
}

#[async_trait]
impl ReferenceSystemResolver for FakeCalculationReferenceRepository {
    async fn zodiacal_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        Ok(format!("zodiacal-{id}"))
    }

    async fn coordinate_reference_system_display_name(
        &self,
        id: i32,
    ) -> Result<String, RuntimeError> {
        Ok(format!("coordinate-{id}"))
    }

    async fn house_system(&self, id: i32) -> Result<HouseSystem, RuntimeError> {
        Ok(HouseSystem {
            id,
            code: format!("house_system_{id}"),
            name: format!("House System {id}"),
            calculation_engine_code: "swiss".to_string(),
        })
    }
}

#[async_trait]
impl ReferenceVersionProvider for FakeCalculationReferenceRepository {
    async fn default_reference_version_id(&self) -> Result<i32, RuntimeError> {
        Ok(7)
    }
}

#[async_trait]
impl CalculationReferenceLoader for FakeCalculationReferenceRepository {
    async fn sign_references(&self) -> Result<Vec<SignReference>, RuntimeError> {
        Ok(vec![SignReference {
            id: 1,
            code: "aries".to_string(),
            name: "Aries".to_string(),
            element_code: None,
            element_label: None,
            modality_code: None,
            modality_name: None,
            polarity_code: None,
            polarity_name: None,
            keywords_json: None,
            shadow_keywords_json: None,
        }])
    }

    async fn house_references(&self) -> Result<Vec<HouseReference>, RuntimeError> {
        Ok(vec![HouseReference {
            id: 101,
            number: 1,
            name: "House 1".to_string(),
            theme_code: "identity".to_string(),
            modality_code: None,
            modality_label: None,
            accidental_strength: None,
            modality_priority_delta: None,
            interpretation_weight: None,
        }])
    }

    async fn motion_state_references(&self) -> Result<Vec<MotionStateReference>, RuntimeError> {
        Ok(vec![MotionStateReference {
            id: 201,
            code: "direct".to_string(),
            label: "Direct".to_string(),
            motion_family: "forward".to_string(),
        }])
    }

    async fn horizon_position_references(
        &self,
    ) -> Result<Vec<HorizonPositionReference>, RuntimeError> {
        Ok(vec![HorizonPositionReference {
            id: 301,
            code: "above_horizon".to_string(),
            label: "Above horizon".to_string(),
        }])
    }

    async fn angle_point_references(&self) -> Result<Vec<AnglePointReference>, RuntimeError> {
        Ok(vec![AnglePointReference {
            id: 401,
            code: "asc".to_string(),
            short_label: "ASC".to_string(),
            full_name: "Ascendant".to_string(),
            axis: "horizontal".to_string(),
            opposite_angle_code: Some("dsc".to_string()),
            associated_house: 1,
            description: "Ascendant".to_string(),
            chart_object_id: 501,
            chart_object_code: "ascendant".to_string(),
            chart_object_name: "Ascendant".to_string(),
            chart_object_sort_order: 1,
        }])
    }
}

#[async_trait]
impl NatalReferenceStore for FakeCalculationReferenceRepository {
    async fn active_chart_objects(
        &self,
        reference_version_id: i32,
    ) -> Result<Vec<ChartObject>, RuntimeError> {
        Ok(vec![ChartObject {
            id: reference_version_id,
            code: "sun".to_string(),
            name: "Sun".to_string(),
            swe_id: Some(0),
            role_code: Some("luminary".to_string()),
            role_label: Some("Luminary".to_string()),
            is_luminary: Some(true),
            is_planet_symbolic: Some(false),
            is_visible_to_naked_eye: Some(true),
            nature_codes: None,
            position_priority_base: Some(1.0),
            angle_priority_base: None,
            source_weight: Some(1.0),
        }])
    }

    async fn aspect_definitions(&self) -> Result<Vec<AspectDefinition>, RuntimeError> {
        Ok(vec![AspectDefinition {
            id: 1,
            code: "conjunction".to_string(),
            name: "Conjunction".to_string(),
            angle: 0.0,
            family: "major".to_string(),
            default_orb_deg: Some(8.0),
            max_default_orb_deg: 8.0,
        }])
    }

    async fn major_aspect_family_reference(
        &self,
    ) -> Result<astral_calculator::application::ports::MajorAspectFamilyReference, RuntimeError>
    {
        Ok(
            astral_calculator::application::ports::MajorAspectFamilyReference {
                expected_aspect_count: 1,
                max_default_orb_deg: 8.0,
            },
        )
    }

    async fn domicile_ruler_references(
        &self,
        _reference_version_id: i32,
    ) -> Result<Vec<astral_calculator::domain::DomicileRulerReference>, RuntimeError> {
        Ok(vec![])
    }

    async fn house_axis_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::HouseAxisReference>, RuntimeError> {
        Ok(vec![])
    }

    async fn lunar_phase_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::LunarPhaseReference>, RuntimeError> {
        Ok(vec![])
    }

    async fn accidental_dignity_condition_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::AccidentalDignityConditionReference>, RuntimeError>
    {
        Ok(vec![])
    }

    async fn object_sect_affinity_references(
        &self,
    ) -> Result<Vec<astral_calculator::domain::ObjectSectAffinityReference>, RuntimeError> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn load_calculation_reference_data_uses_canonical_codes_and_preserves_rows() {
    let repository = FakeCalculationReferenceRepository;

    let references = load_calculation_reference_data(&repository)
        .await
        .expect("reference data should load");

    assert_eq!(references.tropical_zodiacal_reference_system_id, 42);
    assert_eq!(references.geocentric_coordinate_reference_system_id, 84);
    assert_eq!(references.signs[0].code, "aries");
    assert_eq!(references.houses[0].theme_code, "identity");
    assert_eq!(references.motion_states[0].code, "direct");
    assert_eq!(references.horizon_positions[0].code, "above_horizon");
    assert_eq!(references.angle_points[0].code, "asc");
}

#[tokio::test]
async fn load_default_calculation_reference_data_returns_default_version_and_rows() {
    let repository = FakeCalculationReferenceRepository;

    let (reference_version_id, references) = load_default_calculation_reference_data(&repository)
        .await
        .expect("default reference data should load");

    assert_eq!(reference_version_id, 7);
    assert_eq!(references.tropical_zodiacal_reference_system_id, 42);
    assert_eq!(references.geocentric_coordinate_reference_system_id, 84);
}

#[tokio::test]
async fn load_chart_context_loads_chart_objects_aspects_house_and_references() {
    let repository = FakeCalculationReferenceRepository;

    let context = load_chart_context(&repository, 11, 12)
        .await
        .expect("chart context should load");

    assert_eq!(context.reference_version_id, 11);
    assert_eq!(context.chart_objects[0].code, "sun");
    assert_eq!(context.aspect_definitions[0].code, "conjunction");
    assert_eq!(context.house_system.code, "house_system_12");
    assert_eq!(context.references.tropical_zodiacal_reference_system_id, 42);
}

#[tokio::test]
async fn load_default_chart_context_uses_default_reference_version_id() {
    let repository = FakeCalculationReferenceRepository;

    let context = load_default_chart_context(&repository, 12)
        .await
        .expect("default chart context should load");

    assert_eq!(context.reference_version_id, 7);
    assert_eq!(context.house_system.id, 12);
}

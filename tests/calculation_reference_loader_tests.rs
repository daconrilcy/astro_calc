use async_trait::async_trait;

use astral_calculator::application::calculation_references::{
    load_calculation_reference_data, load_default_calculation_reference_data,
};
use astral_calculator::application::ports::{
    CalculationReferenceLoader, ReferenceSystemLookup, ReferenceVersionProvider,
};
use astral_calculator::domain::{
    AnglePointReference, HorizonPositionReference, HouseReference, MotionStateReference,
    SignReference,
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

    async fn coordinate_reference_system_id_by_key(
        &self,
        key: &str,
    ) -> Result<i32, RuntimeError> {
        match key {
            "geocentric" => Ok(84),
            other => Err(RuntimeError::InvalidRuntimeTable(format!(
                "unexpected coordinate key {other}"
            ))),
        }
    }

    async fn house_system_id_by_code(&self, code: &str) -> Result<i32, RuntimeError> {
        Err(RuntimeError::InvalidRuntimeTable(format!(
            "unexpected house system lookup {code}"
        )))
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

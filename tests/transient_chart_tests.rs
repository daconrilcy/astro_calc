use chrono::{TimeZone, Utc};

use astral_calculator::application::chart_context::ChartContextData;
use astral_calculator::application::transient_chart::calculate_transient_chart_facts;
use astral_calculator::astrology::ephemeris::EphemerisEngine;
use astral_calculator::domain::{
    AspectDefinition, CalculatedChartFacts, CalculationReferenceData, ChartObject,
    HorizonPositionReference, HouseReference, HouseSystem, MotionStateReference, NatalChartInput,
    SignReference,
};
use astral_calculator::runtime::RuntimeError;

struct RecordingEphemerisEngine;

impl EphemerisEngine for RecordingEphemerisEngine {
    fn calculate_chart(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
        references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        assert_eq!(
            input.birth_datetime_utc,
            Utc.with_ymd_and_hms(2026, 6, 22, 6, 30, 0)
                .single()
                .expect("valid UTC transit datetime")
        );
        assert_eq!(
            input.product_code.as_deref(),
            Some("horoscope_daily_transit")
        );
        assert_eq!(input.subject_label.as_deref(), Some("baseline"));
        assert_eq!(input.reference_version_id, 7);
        assert_eq!(chart_objects.len(), 1);
        assert_eq!(chart_objects[0].code, "sun");
        assert_eq!(aspects.len(), 1);
        assert_eq!(aspects[0].code, "conjunction");
        assert_eq!(house_system.code, "placidus");
        assert_eq!(references.tropical_zodiacal_reference_system_id, 42);
        assert_eq!(references.geocentric_coordinate_reference_system_id, 84);

        Ok(CalculatedChartFacts {
            positions: Vec::new(),
            house_cusps: Vec::new(),
            aspects: Vec::new(),
        })
    }
}

#[test]
fn calculate_transient_chart_facts_overrides_datetime_and_product_code_only() {
    let natal_input = NatalChartInput {
        subject_label: Some("baseline".to_string()),
        birth_datetime_utc: Utc
            .with_ymd_and_hms(1990, 5, 2, 10, 15, 0)
            .single()
            .expect("valid UTC natal datetime"),
        latitude_deg: 48.8566,
        longitude_deg: 2.3522,
        altitude_m: Some(35.0),
        reference_version_id: 7,
        calculation_profile_id: Some(11),
        zodiacal_reference_system_id: 42,
        coordinate_reference_system_id: 84,
        house_system_id: 12,
        product_code: Some("simplified".to_string()),
        client_idempotency_key: Some("idempotency-key".to_string()),
    };
    let chart_context = ChartContextData {
        reference_version_id: 7,
        chart_objects: vec![ChartObject {
            id: 1,
            code: "sun".to_string(),
            name: "Sun".to_string(),
            swe_id: Some(0),
            role_code: None,
            role_label: None,
            is_luminary: Some(true),
            is_planet_symbolic: Some(false),
            is_visible_to_naked_eye: Some(true),
            nature_codes: None,
            position_priority_base: Some(1.0),
            angle_priority_base: None,
            source_weight: Some(1.0),
        }],
        aspect_definitions: vec![AspectDefinition {
            id: 1,
            code: "conjunction".to_string(),
            name: "Conjunction".to_string(),
            angle: 0.0,
            family: "major".to_string(),
            default_orb_deg: Some(8.0),
            max_default_orb_deg: 8.0,
        }],
        house_system: HouseSystem {
            id: 12,
            code: "placidus".to_string(),
            name: "Placidus".to_string(),
            calculation_engine_code: "swiss".to_string(),
        },
        references: CalculationReferenceData {
            tropical_zodiacal_reference_system_id: 42,
            geocentric_coordinate_reference_system_id: 84,
            signs: vec![SignReference {
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
            }],
            houses: vec![HouseReference {
                id: 1,
                number: 1,
                name: "House 1".to_string(),
                theme_code: "identity".to_string(),
                modality_code: None,
                modality_label: None,
                accidental_strength: None,
                modality_priority_delta: None,
                interpretation_weight: None,
            }],
            motion_states: vec![MotionStateReference {
                id: 1,
                code: "direct".to_string(),
                label: "Direct".to_string(),
                motion_family: "forward".to_string(),
            }],
            horizon_positions: vec![HorizonPositionReference {
                id: 1,
                code: "above_horizon".to_string(),
                label: "Above horizon".to_string(),
            }],
            angle_points: Vec::new(),
        },
    };
    let reference_datetime_utc = Utc
        .with_ymd_and_hms(2026, 6, 22, 6, 30, 0)
        .single()
        .expect("valid UTC transit datetime");

    let facts = calculate_transient_chart_facts(
        &RecordingEphemerisEngine,
        &natal_input,
        reference_datetime_utc,
        "horoscope_daily_transit",
        &chart_context,
    )
    .expect("transient chart facts should be delegated to the ephemeris engine");

    assert!(facts.positions.is_empty());
    assert!(facts.house_cusps.is_empty());
    assert!(facts.aspects.is_empty());
    assert_eq!(
        natal_input.birth_datetime_utc,
        Utc.with_ymd_and_hms(1990, 5, 2, 10, 15, 0)
            .single()
            .expect("valid UTC natal datetime")
    );
    assert_eq!(natal_input.product_code.as_deref(), Some("simplified"));
}

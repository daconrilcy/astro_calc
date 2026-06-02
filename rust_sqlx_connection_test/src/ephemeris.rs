use std::path::{Path, PathBuf};
#[cfg(feature = "swisseph-engine")]
use std::sync::{Mutex, OnceLock};

use crate::domain::{CalculatedChartFacts, CalculationReferenceData, NatalChartInput};
use crate::models::{AspectDefinition, ChartObject, HouseSystem};
use crate::runtime::RuntimeError;

pub trait EphemerisEngine {
    fn calculate_natal(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
        references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError>;
}

#[derive(Debug, Clone)]
pub struct SwissEphemerisEngine {
    ephemeris_path: PathBuf,
}

impl SwissEphemerisEngine {
    pub fn new(ephemeris_path: impl Into<PathBuf>) -> Self {
        Self {
            ephemeris_path: ephemeris_path.into(),
        }
    }

    pub fn default_from_workspace() -> Self {
        Self::new(Path::new("..").join("ephe").join("se-2026a"))
    }
}

impl EphemerisEngine for SwissEphemerisEngine {
    #[cfg(feature = "swisseph-engine")]
    fn calculate_natal(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
        references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        use crate::aspects::detect_aspects;
        use crate::domain::{HouseCuspFact, ObjectPositionFact};
        use crate::facts::{
            house_number_from_cusps, motion_state_id, normalize_degrees, whole_sign_house_number,
            zodiac_slot_for_longitude,
        };
        use serde_json::json;
        use swiss_eph::safe::{calc_ut, houses, set_ephe_path, CalcFlags};

        validate_supported_reference_systems(input)?;
        let _guard = swiss_ephemeris_lock()
            .lock()
            .map_err(|_| RuntimeError::Ephemeris("Swiss Ephemeris lock poisoned".to_string()))?;
        set_ephe_path(
            self.ephemeris_path
                .to_str()
                .ok_or_else(|| RuntimeError::Ephemeris("invalid ephemeris path".to_string()))?,
        );

        let jd_ut = julian_day_ut(input)?;
        let house_code = house_system_code(&house_system.calculation_engine_code)?;
        let cusps_raw = houses(jd_ut, input.latitude_deg, input.longitude_deg, house_code)
            .map_err(|error| RuntimeError::Ephemeris(error.to_string()))?;

        let mut house_cusps = Vec::with_capacity(12);
        for house_number in 1..=12 {
            let longitude = normalize_degrees(cusps_raw.cusps[(house_number - 1) as usize]);
            let house = house_reference_for_number(&references.houses, house_number)?;
            let sign = sign_reference_for_zodiac_slot(
                &references.signs,
                zodiac_slot_for_longitude(longitude),
            )?;
            house_cusps.push(HouseCuspFact {
                house_id: house.id,
                house_number,
                sign_id: sign.id,
                longitude_deg: round4(longitude),
            });
        }

        let ascendant_longitude = house_cusps
            .first()
            .map(|cusp| cusp.longitude_deg)
            .unwrap_or(0.0);
        let mut positions = Vec::new();

        add_angle_positions(
            input,
            chart_objects,
            references,
            &house_cusps,
            &mut positions,
            cusps_raw.ascendant,
            cusps_raw.mc,
        )?;

        for object in chart_objects
            .iter()
            .filter(|object| object.swe_id.is_some())
        {
            let position = calc_ut(
                jd_ut,
                object.swe_id.unwrap(),
                CalcFlags::new().with_swiss_ephemeris().with_speed().raw(),
            )
            .map_err(|error| RuntimeError::Ephemeris(error.to_string()))?;
            let longitude = round4(normalize_degrees(position.longitude));
            let latitude = round4(position.latitude);
            let speed = round4(position.longitude_speed);
            let house_number = if house_system.calculation_engine_code == "whole_sign" {
                Some(whole_sign_house_number(ascendant_longitude, longitude))
            } else {
                house_number_from_cusps(longitude, &house_cusps)
            };
            let sign = sign_reference_for_zodiac_slot(
                &references.signs,
                zodiac_slot_for_longitude(longitude),
            )?;
            let house = house_number
                .map(|number| house_reference_for_number(&references.houses, number))
                .transpose()?;
            let motion_state_id = motion_state_id(Some(speed));
            let motion_state = motion_state_id
                .and_then(|id| references.motion_states.iter().find(|state| state.id == id));

            positions.push(ObjectPositionFact {
                chart_object_id: object.id,
                object_code: object.code.clone(),
                object_name: object.name.clone(),
                zodiacal_reference_system_id: input.zodiacal_reference_system_id,
                coordinate_reference_system_id: input.coordinate_reference_system_id,
                sign_id: sign.id,
                sign_code: sign.code.clone(),
                sign_name: sign.name.clone(),
                house_id: house.map(|house| house.id),
                house_number,
                house_name: house.map(|house| house.name.clone()),
                motion_state_id,
                horizon_position_id: None,
                longitude_deg: longitude,
                latitude_deg: Some(latitude),
                apparent_speed_deg_per_day: Some(speed),
                altitude_deg: None,
                is_visible: None,
                facts_json: Some(json!({
                    "distance": position.distance,
                    "speed_in_latitude": position.latitude_speed,
                    "speed_in_distance": position.distance_speed,
                    "sign_context": sign_context(sign),
                    "house_modality": house.and_then(house_modality),
                    "house_context": house.map(house_context),
                    "object_context": object_context(object),
                    "motion_context": motion_state.map(motion_context)
                })),
            });
        }

        let aspects = detect_aspects(&positions, aspects);

        Ok(CalculatedChartFacts {
            positions,
            house_cusps,
            aspects,
        })
    }

    #[cfg(not(feature = "swisseph-engine"))]
    fn calculate_natal(
        &self,
        _input: &NatalChartInput,
        _chart_objects: &[ChartObject],
        _aspects: &[AspectDefinition],
        _house_system: &HouseSystem,
        _references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        Err(RuntimeError::Ephemeris(
            format!(
                "Swiss Ephemeris support is disabled for path {}; rebuild with --features swisseph-engine",
                self.ephemeris_path.display()
            ),
        ))
    }
}

#[cfg(feature = "swisseph-engine")]
fn sign_context(sign: &crate::models::SignReference) -> serde_json::Value {
    serde_json::json!({
        "element": &sign.element_code,
        "element_label": &sign.element_label,
        "modality": &sign.modality_code,
        "modality_label": &sign.modality_name,
        "polarity": &sign.polarity_code,
        "polarity_label": &sign.polarity_name,
        "keywords": &sign.keywords_json
    })
}

#[cfg(feature = "swisseph-engine")]
fn house_modality(house: &crate::models::HouseReference) -> Option<serde_json::Value> {
    house.modality_code.as_ref().map(|code| {
        serde_json::json!({
            "code": code,
            "label": &house.modality_label,
            "accidental_strength": &house.accidental_strength,
            "interpretation_weight": &house.interpretation_weight
        })
    })
}

#[cfg(feature = "swisseph-engine")]
fn house_context(house: &crate::models::HouseReference) -> serde_json::Value {
    serde_json::json!({
        "theme_code": &house.theme_code
    })
}

#[cfg(feature = "swisseph-engine")]
fn object_context(object: &ChartObject) -> serde_json::Value {
    serde_json::json!({
        "role": &object.role_code,
        "role_label": &object.role_label,
        "nature": &object.nature_codes,
        "is_luminary": &object.is_luminary,
        "is_planet_symbolic": &object.is_planet_symbolic,
        "is_visible_to_naked_eye": &object.is_visible_to_naked_eye
    })
}

#[cfg(feature = "swisseph-engine")]
fn add_angle_positions(
    input: &NatalChartInput,
    chart_objects: &[ChartObject],
    references: &CalculationReferenceData,
    house_cusps: &[crate::domain::HouseCuspFact],
    positions: &mut Vec<crate::domain::ObjectPositionFact>,
    ascendant_longitude: f64,
    mc_longitude: f64,
) -> Result<(), RuntimeError> {
    for angle in &references.angle_points {
        let Some(object) = chart_objects
            .iter()
            .find(|object| object.id == angle.chart_object_id)
        else {
            continue;
        };
        let longitude = round4(angle_longitude(angle, ascendant_longitude, mc_longitude)?);
        let sign = sign_reference_for_zodiac_slot(
            &references.signs,
            crate::facts::zodiac_slot_for_longitude(longitude),
        )?;
        let house = house_reference_for_number(&references.houses, angle.associated_house)?;

        positions.push(crate::domain::ObjectPositionFact {
            chart_object_id: object.id,
            object_code: object.code.clone(),
            object_name: if object.name.trim().is_empty() {
                angle.full_name.clone()
            } else {
                object.name.clone()
            },
            zodiacal_reference_system_id: input.zodiacal_reference_system_id,
            coordinate_reference_system_id: input.coordinate_reference_system_id,
            sign_id: sign.id,
            sign_code: sign.code.clone(),
            sign_name: sign.name.clone(),
            house_id: Some(house.id),
            house_number: Some(angle.associated_house),
            house_name: Some(house.name.clone()),
            motion_state_id: None,
            horizon_position_id: None,
            longitude_deg: longitude,
            latitude_deg: None,
            apparent_speed_deg_per_day: None,
            altitude_deg: None,
            is_visible: None,
            facts_json: Some(serde_json::json!({
                "sign_context": sign_context(sign),
                "house_modality": house_modality(house),
                "house_context": house_context(house),
                "object_context": object_context(object),
                "angle_context": {
                    "angle_point_id": angle.id,
                    "angle_point_code": angle.code,
                    "short_label": angle.short_label,
                    "full_name": angle.full_name,
                    "axis": angle.axis,
                    "opposite_angle_code": angle.opposite_angle_code,
                    "associated_house_number": angle.associated_house,
                    "description": angle.description,
                    "chart_object_sort_order": angle.chart_object_sort_order,
                    "house_cusp_longitude_deg": house_cusps
                        .iter()
                        .find(|cusp| cusp.house_number == angle.associated_house)
                        .map(|cusp| cusp.longitude_deg)
                }
            })),
        });
    }

    Ok(())
}

#[cfg(feature = "swisseph-engine")]
fn angle_longitude(
    angle: &crate::models::AnglePointReference,
    ascendant_longitude: f64,
    mc_longitude: f64,
) -> Result<f64, RuntimeError> {
    let longitude = match angle.code.as_str() {
        "asc" => ascendant_longitude,
        "dsc" => ascendant_longitude + 180.0,
        "mc" => mc_longitude,
        "ic" => mc_longitude + 180.0,
        other => {
            return Err(RuntimeError::Ephemeris(format!(
                "unsupported angle point code {other}"
            )))
        }
    };
    Ok(crate::facts::normalize_degrees(longitude))
}

#[cfg(feature = "swisseph-engine")]
fn motion_context(motion_state: &crate::models::MotionStateReference) -> serde_json::Value {
    serde_json::json!({
        "motion_state": motion_state.code,
        "label": motion_state.label,
        "motion_family": motion_state.motion_family
    })
}

#[cfg(feature = "swisseph-engine")]
fn sign_reference_for_zodiac_slot(
    signs: &[crate::models::SignReference],
    zodiac_slot: i32,
) -> Result<&crate::models::SignReference, RuntimeError> {
    if !(1..=12).contains(&zodiac_slot) {
        return Err(RuntimeError::Ephemeris(format!(
            "invalid zodiac slot {zodiac_slot}"
        )));
    }
    if signs.len() != 12 {
        return Err(RuntimeError::Ephemeris(format!(
            "expected 12 sign references, found {}",
            signs.len()
        )));
    }

    signs.get((zodiac_slot - 1) as usize).ok_or_else(|| {
        RuntimeError::Ephemeris(format!(
            "missing sign reference for zodiac slot {zodiac_slot}"
        ))
    })
}

#[cfg(feature = "swisseph-engine")]
fn house_reference_for_number(
    houses: &[crate::models::HouseReference],
    house_number: i32,
) -> Result<&crate::models::HouseReference, RuntimeError> {
    houses
        .iter()
        .find(|house| house.number == house_number)
        .ok_or_else(|| {
            RuntimeError::Ephemeris(format!(
                "missing house reference for house number {house_number}"
            ))
        })
}

#[cfg(feature = "swisseph-engine")]
fn swiss_ephemeris_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(feature = "swisseph-engine")]
fn validate_supported_reference_systems(input: &NatalChartInput) -> Result<(), RuntimeError> {
    if input.zodiacal_reference_system_id != 1 {
        return Err(RuntimeError::Ephemeris(format!(
            "unsupported zodiacal_reference_system_id {}; only tropical (id=1) is implemented",
            input.zodiacal_reference_system_id
        )));
    }
    if input.coordinate_reference_system_id != 1 {
        return Err(RuntimeError::Ephemeris(format!(
            "unsupported coordinate_reference_system_id {}; only geocentric (id=1) is implemented",
            input.coordinate_reference_system_id
        )));
    }
    Ok(())
}

#[cfg(feature = "swisseph-engine")]
fn julian_day_ut(input: &NatalChartInput) -> Result<f64, RuntimeError> {
    use chrono::{Datelike, Timelike};
    use swiss_eph::safe::julday;

    let datetime = input.birth_datetime_utc;
    let hour = datetime.hour() as f64
        + datetime.minute() as f64 / 60.0
        + datetime.second() as f64 / 3600.0
        + f64::from(datetime.nanosecond()) / 3_600_000_000_000.0;

    Ok(julday(
        datetime.year(),
        datetime.month() as i32,
        datetime.day() as i32,
        hour,
    ))
}

#[cfg(feature = "swisseph-engine")]
fn house_system_code(code: &str) -> Result<swiss_eph::safe::HouseSystem, RuntimeError> {
    use swiss_eph::safe::HouseSystem;

    match code {
        "placidus" => Ok(HouseSystem::Placidus),
        "whole_sign" => Ok(HouseSystem::WholeSign),
        "equal" => Ok(HouseSystem::Equal),
        "porphyry" => Ok(HouseSystem::Porphyrius),
        other => Err(RuntimeError::Ephemeris(format!(
            "unsupported house system {other}"
        ))),
    }
}

#[cfg(feature = "swisseph-engine")]
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

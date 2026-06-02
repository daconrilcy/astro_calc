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
                motion_state_id: motion_state_id(Some(speed)),
                horizon_position_id: None,
                longitude_deg: longitude,
                latitude_deg: Some(latitude),
                apparent_speed_deg_per_day: Some(speed),
                altitude_deg: None,
                is_visible: None,
                facts_json: Some(json!({
                    "distance": position.distance,
                    "speed_in_latitude": position.latitude_speed,
                    "speed_in_distance": position.distance_speed
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

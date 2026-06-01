use std::path::{Path, PathBuf};

use crate::domain::{
    AspectDefinition, CalculatedChartFacts, ChartObject, HouseSystem, NatalChartInput,
};
use crate::runtime::RuntimeError;

pub trait EphemerisEngine {
    fn calculate_natal(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
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
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        use crate::aspects::detect_aspects;
        use crate::domain::{HouseCuspFact, ObjectPositionFact};
        use crate::facts::{
            house_id_from_cusps, motion_state_id, normalize_degrees, sign_id_for_longitude,
            whole_sign_house_id,
        };
        use serde_json::json;
        use swiss_eph::safe::{calc_ut, close, houses, set_ephe_path, CalcFlags};

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
            house_cusps.push(HouseCuspFact {
                house_id: house_number,
                sign_id: sign_id_for_longitude(longitude),
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
            let house_id = if house_system.calculation_engine_code == "whole_sign" {
                Some(whole_sign_house_id(ascendant_longitude, longitude))
            } else {
                house_id_from_cusps(longitude, &house_cusps)
            };

            positions.push(ObjectPositionFact {
                chart_object_id: object.id,
                object_code: object.code.clone(),
                object_name: object.name.clone(),
                zodiacal_reference_system_id: input.zodiacal_reference_system_id,
                coordinate_reference_system_id: input.coordinate_reference_system_id,
                sign_id: sign_id_for_longitude(longitude),
                house_id,
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
        close();

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

//! Adaptateurs de calcul éphémérides et assemblage des faits astrologiques
//! bruts.

use std::path::{Path, PathBuf};
#[cfg(feature = "swisseph-engine")]
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "swisseph-engine")]
use crate::domain::{AnglePointReference, HouseReference, MotionStateReference, SignReference};
use crate::domain::{
    AspectDefinition, CalculatedChartFacts, CalculationReferenceData, ChartObject, HouseSystem,
    NatalChartInput,
};
use crate::shared::error::RuntimeError;

/// Abstraction du moteur capable de produire un thème natal calculé.
pub trait EphemerisEngine {
    /// Calcule l'ensemble des positions, cuspides et aspects d'un thème.
    fn calculate_chart(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
        references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError>;

    /// Alias explicite pour les appels métier orientés thème natal.
    fn calculate_natal(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
        references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        self.calculate_chart(input, chart_objects, aspects, house_system, references)
    }
}

#[derive(Debug, Clone)]
/// Implémentation basée sur Swiss Ephemeris et un répertoire local
/// d'éphémérides.
pub struct SwissEphemerisEngine {
    ephemeris_path: PathBuf,
}

impl SwissEphemerisEngine {
    /// Construit un moteur pointant vers un dossier d'éphémérides.
    pub fn new(ephemeris_path: impl Into<PathBuf>) -> Self {
        Self {
            ephemeris_path: ephemeris_path.into(),
        }
    }

    /// Utilise l'emplacement standard du workspace pour les fichiers `.se1`.
    pub fn default_from_workspace() -> Self {
        Self::new(Path::new("..").join("ephe").join("se-2026a"))
    }

    /// Expose le chemin effectif utilisé par le moteur.
    pub fn ephemeris_path(&self) -> &Path {
        &self.ephemeris_path
    }
}

impl EphemerisEngine for SwissEphemerisEngine {
    #[cfg(feature = "swisseph-engine")]
    /// Calcule le thème en interrogeant Swiss Ephemeris puis enrichit les faits
    /// avec les références métier chargées depuis la base.
    fn calculate_chart(
        &self,
        input: &NatalChartInput,
        chart_objects: &[ChartObject],
        aspects: &[AspectDefinition],
        house_system: &HouseSystem,
        references: &CalculationReferenceData,
    ) -> Result<CalculatedChartFacts, RuntimeError> {
        use crate::astrology::angles::normalize_degrees;
        use crate::astrology::aspects::detect_aspects;
        use crate::astrology::house_geometry::house_number_from_cusps;
        use crate::astrology::motion::motion_state_for_speed;
        use crate::astrology::zodiac::{whole_sign_house_number, zodiac_slot_for_longitude};
        use crate::domain::{HouseCuspFact, ObjectPositionFact};
        use serde_json::json;
        use swiss_eph::safe::{
            azimuth_altitude, calc_ut, houses, set_ephe_path, set_topo, CalcFlags, GeoPos,
        };
        use swiss_eph::SE_EQU2HOR;

        validate_supported_reference_systems(input)?;
        let _guard = swiss_ephemeris_lock()
            .lock()
            .map_err(|_| RuntimeError::Ephemeris("Swiss Ephemeris lock poisoned".to_string()))?;
        set_ephe_path(
            self.ephemeris_path
                .to_str()
                .ok_or_else(|| RuntimeError::Ephemeris("invalid ephemeris path".to_string()))?,
        );
        let observer_altitude_m = input.altitude_m.unwrap_or(0.0);
        set_topo(input.longitude_deg, input.latitude_deg, observer_altitude_m);

        let jd_ut = julian_day_ut(input)?;
        let geopos = GeoPos {
            longitude: input.longitude_deg,
            latitude: input.latitude_deg,
            altitude: observer_altitude_m,
        };
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
            let motion_state = motion_state_for_speed(Some(speed), &references.motion_states);
            let motion_state_id = motion_state.map(|state| state.id);
            let equatorial_position = calc_ut(
                jd_ut,
                object.swe_id.unwrap(),
                CalcFlags::new()
                    .with_swiss_ephemeris()
                    .with_speed()
                    .with_equatorial()
                    .with_topocentric()
                    .raw(),
            )
            .map_err(|error| RuntimeError::Ephemeris(error.to_string()))?;
            let (_azimuth_deg, altitude_deg) =
                azimuth_altitude(jd_ut, SE_EQU2HOR, geopos, equatorial_position)
                    .map_err(|error| RuntimeError::Ephemeris(error.to_string()))?;
            let altitude_deg = round4(altitude_deg);
            let horizon_position_code = horizon_position_code_for_altitude(altitude_deg);
            let horizon_position_id = horizon_position_id(references, horizon_position_code)?;

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
                horizon_position_id: Some(horizon_position_id),
                longitude_deg: longitude,
                latitude_deg: Some(latitude),
                apparent_speed_deg_per_day: Some(speed),
                altitude_deg: Some(altitude_deg),
                is_visible: Some(matches!(
                    horizon_position_code,
                    "above_horizon" | "on_horizon"
                )),
                facts_json: Some(json!({
                    "distance": position.distance,
                    "speed_in_latitude": position.latitude_speed,
                    "speed_in_distance": position.distance_speed,
                    "visibility_context": {
                        "horizon_position": horizon_position_code,
                        "source": "calculated_altitude"
                    },
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
    /// Signale explicitement qu'aucun calcul réel n'est disponible sans la
    /// feature `swisseph-engine`.
    fn calculate_chart(
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
/// Sérialise le contexte interprétatif d'un signe pour les faits calculés.
fn sign_context(sign: &SignReference) -> serde_json::Value {
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
/// Extrait les métadonnées de modalité de maison quand elles existent.
fn house_modality(house: &HouseReference) -> Option<serde_json::Value> {
    house.modality_code.as_ref().map(|code| {
        serde_json::json!({
            "code": code,
            "label": &house.modality_label,
            "accidental_strength": &house.accidental_strength,
            "priority_delta": &house.modality_priority_delta,
            "interpretation_weight": &house.interpretation_weight
        })
    })
}

#[cfg(feature = "swisseph-engine")]
/// Expose le thème métier associé à une maison.
fn house_context(house: &HouseReference) -> serde_json::Value {
    serde_json::json!({
        "theme_code": &house.theme_code
    })
}

#[cfg(feature = "swisseph-engine")]
/// Conserve les métadonnées d'objet utiles au scoring et à l'interprétation.
fn object_context(object: &ChartObject) -> serde_json::Value {
    serde_json::json!({
        "role": &object.role_code,
        "role_label": &object.role_label,
        "nature": &object.nature_codes,
        "is_luminary": &object.is_luminary,
        "is_planet_symbolic": &object.is_planet_symbolic,
        "is_visible_to_naked_eye": &object.is_visible_to_naked_eye,
        "signal_scoring": {
            "position_priority_base": &object.position_priority_base,
            "angle_priority_base": &object.angle_priority_base,
            "source_weight": &object.source_weight
        }
    })
}

#[cfg(feature = "swisseph-engine")]
/// Injecte les angles structurels (ASC, DSC, MC, IC) comme positions
/// synthétiques alignées sur le même contrat que les objets calculés.
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
            crate::astrology::zodiac::zodiac_slot_for_longitude(longitude),
        )?;
        let house = house_reference_for_number(&references.houses, angle.associated_house)?;
        let horizon_position_code = angle_horizon_position_code(angle.code.as_str())?;
        let horizon_position_id = horizon_position_id(references, horizon_position_code)?;

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
            horizon_position_id: Some(horizon_position_id),
            longitude_deg: longitude,
            latitude_deg: None,
            apparent_speed_deg_per_day: None,
            altitude_deg: None,
            is_visible: Some(matches!(
                horizon_position_code,
                "above_horizon" | "on_horizon"
            )),
            facts_json: Some(serde_json::json!({
                "visibility_context": {
                    "horizon_position": horizon_position_code,
                    "source": "angle_context"
                },
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
/// Convertit un code d'angle en longitude à partir de l'ASC et du MC calculés.
fn angle_longitude(
    angle: &AnglePointReference,
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
    Ok(crate::astrology::angles::normalize_degrees(longitude))
}

#[cfg(feature = "swisseph-engine")]
/// Classe la visibilité d'un objet selon son altitude apparente.
fn horizon_position_code_for_altitude(altitude_deg: f64) -> &'static str {
    if altitude_deg > 0.0 {
        "above_horizon"
    } else if altitude_deg < 0.0 {
        "below_horizon"
    } else {
        "on_horizon"
    }
}

#[cfg(feature = "swisseph-engine")]
/// Donne la position horizon canonique d'un angle structurel.
fn angle_horizon_position_code(angle_code: &str) -> Result<&'static str, RuntimeError> {
    match angle_code {
        "asc" | "dsc" => Ok("on_horizon"),
        "mc" => Ok("above_horizon"),
        "ic" => Ok("below_horizon"),
        other => Err(RuntimeError::Ephemeris(format!(
            "unsupported angle point code {other}"
        ))),
    }
}

#[cfg(feature = "swisseph-engine")]
/// Résout l'identifiant DB d'une position relative à l'horizon.
fn horizon_position_id(
    references: &CalculationReferenceData,
    code: &str,
) -> Result<i32, RuntimeError> {
    references
        .horizon_positions
        .iter()
        .find(|position| position.code == code)
        .map(|position| position.id)
        .ok_or_else(|| RuntimeError::Ephemeris(format!("missing horizon position code {code}")))
}

#[cfg(feature = "swisseph-engine")]
/// Sérialise le contexte de mouvement direct/rétrograde/stationnaire.
fn motion_context(motion_state: &MotionStateReference) -> serde_json::Value {
    serde_json::json!({
        "motion_state": motion_state.code,
        "label": motion_state.label,
        "motion_family": motion_state.motion_family
    })
}

#[cfg(feature = "swisseph-engine")]
/// Résout le signe correspondant à un slot zodiacal 1..=12.
fn sign_reference_for_zodiac_slot(
    signs: &[SignReference],
    zodiac_slot: i32,
) -> Result<&SignReference, RuntimeError> {
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
/// Résout la référence d'une maison par son numéro canonique.
fn house_reference_for_number(
    houses: &[HouseReference],
    house_number: i32,
) -> Result<&HouseReference, RuntimeError> {
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
/// Protège l'accès au moteur Swiss Ephemeris, dont l'état global n'est pas sûr
/// pour des appels concurrents non coordonnés.
fn swiss_ephemeris_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(feature = "swisseph-engine")]
/// Refuse les référentiels non encore pris en charge par l'implémentation
/// courante.
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
/// Convertit la date UTC d'entrée en jour julien utilisé par Swiss Ephemeris.
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
/// Mappe le code métier du système de maisons vers l'énumération Swiss
/// Ephemeris.
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
/// Aligne les sorties numériques sur la précision publique du contrat.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

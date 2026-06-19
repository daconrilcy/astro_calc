//! Module astral_calculator\src\features\horoscope\period.rs du moteur astral_calculator.

use std::collections::HashSet;

use crate::astrology::angles::normalize_degrees;
use crate::astrology::transits::{
    is_standard_transit_object, nearest_major_aspect_name_and_orb, nearest_major_transit_match,
    preferred_transit_position,
};
use crate::domain::AspectDefinition;
use crate::domain::ObjectPositionFact;
use crate::shared::error::RuntimeError;
use crate::shared::time::{normalize_rfc3339_utc, parse_rfc3339};

use super::{
    HoroscopePeriodCalculationRequest, HoroscopePeriodCalculationResponse, HoroscopePeriodSnapshot,
    HoroscopeSignalThemeMapping, HoroscopeSnapshotRequest, HoroscopeTransitFact,
};

/// Fonction normalize_horoscope_period_request_utc.
pub fn normalize_horoscope_period_request_utc(
    mut request: HoroscopePeriodCalculationRequest,
) -> Result<HoroscopePeriodCalculationRequest, String> {
    normalize_period_resolution_utc_field(&mut request.period_resolution, "start_datetime_utc")?;
    normalize_period_resolution_utc_field(&mut request.period_resolution, "end_datetime_utc")?;
    for snapshot in &mut request.scan_plan.snapshots {
        snapshot.reference_datetime_utc = normalize_rfc3339_utc(&snapshot.reference_datetime_utc)?;
    }
    validate_horoscope_period_request_scan_plan(&request)?;
    Ok(request)
}

/// Fonction calculate_horoscope_period.
pub fn calculate_horoscope_period(
    request: HoroscopePeriodCalculationRequest,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_from_positions(request, &[], 8.0)
}

/// Fonction calculate_horoscope_period_from_positions.
pub fn calculate_horoscope_period_from_positions(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    period_max_major_aspect_orb_deg: f64,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_from_transits(
        request,
        natal_positions,
        &[],
        period_max_major_aspect_orb_deg,
    )
}

/// Fonction calculate_horoscope_period_from_transits.
pub fn calculate_horoscope_period_from_transits(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_snapshots: &[(String, Vec<ObjectPositionFact>)],
    max_major_aspect_orb_deg: f64,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_from_transits_with_aspects(
        request,
        natal_positions,
        transit_snapshots,
        max_major_aspect_orb_deg,
        &[],
        &[],
    )
}

/// Fonction calculate_horoscope_period_from_transits_with_aspects.
pub fn calculate_horoscope_period_from_transits_with_aspects(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_snapshots: &[(String, Vec<ObjectPositionFact>)],
    max_major_aspect_orb_deg: f64,
    aspect_definitions: &[AspectDefinition],
    theme_mappings: &[HoroscopeSignalThemeMapping],
) -> HoroscopePeriodCalculationResponse {
    try_calculate_horoscope_period_from_transits_with_aspects(
        request,
        natal_positions,
        transit_snapshots,
        max_major_aspect_orb_deg,
        aspect_definitions,
        theme_mappings,
    )
    .expect("historical wrapper called with invalid horoscope period calculation request")
}

/// Fonction try_calculate_horoscope_period_from_transits_with_aspects.
pub fn try_calculate_horoscope_period_from_transits_with_aspects(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_snapshots: &[(String, Vec<ObjectPositionFact>)],
    max_major_aspect_orb_deg: f64,
    aspect_definitions: &[AspectDefinition],
    theme_mappings: &[HoroscopeSignalThemeMapping],
) -> Result<HoroscopePeriodCalculationResponse, RuntimeError> {
    let request = normalize_horoscope_period_request_utc(request).map_err(|err| {
        RuntimeError::InvalidEngineRequest(format!(
            "invalid horoscope period calculation request: {err}"
        ))
    })?;
    let usable_positions = natal_positions
        .iter()
        .filter(|position| is_standard_transit_object(position.object_code.as_str()))
        .collect::<Vec<_>>();
    let snapshots = request
        .scan_plan
        .snapshots
        .iter()
        .enumerate()
        .map(|(index, snapshot)| {
            let transit_positions = transit_snapshots
                .iter()
                .find(|(key, _)| key == &snapshot.snapshot_key)
                .map(|(_, positions)| positions.as_slice());
            real_period_snapshot(
                index,
                snapshot,
                &usable_positions,
                transit_positions,
                max_major_aspect_orb_deg,
                aspect_definitions,
                theme_mappings,
            )
        })
        .collect::<Vec<_>>();
    let evidence_keys = snapshots
        .iter()
        .flat_map(|snapshot| snapshot.transits_to_natal.iter())
        .map(|fact| fact.evidence_key.clone())
        .collect::<Vec<_>>();

    Ok(HoroscopePeriodCalculationResponse {
        contract_version: "horoscope_period_calculation_response".into(),
        service_code: request.service_code,
        period_resolution: request.period_resolution,
        scan_plan: request.scan_plan,
        snapshots,
        calculation_warnings: Vec::new(),
        evidence_keys,
    })
}

/// Fonction calculate_horoscope_period_natal.
pub fn calculate_horoscope_period_natal(
    request: HoroscopePeriodCalculationRequest,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period(request)
}

/// Fonction calculate_horoscope_period_natal_from_positions.
pub fn calculate_horoscope_period_natal_from_positions(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    period_max_major_aspect_orb_deg: f64,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_from_positions(
        request,
        natal_positions,
        period_max_major_aspect_orb_deg,
    )
}

/// Fonction calculate_horoscope_period_natal_from_transits.
pub fn calculate_horoscope_period_natal_from_transits(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_snapshots: &[(String, Vec<ObjectPositionFact>)],
    max_major_aspect_orb_deg: f64,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_from_transits(
        request,
        natal_positions,
        transit_snapshots,
        max_major_aspect_orb_deg,
    )
}

/// Fonction normalize_period_resolution_utc_field.
fn normalize_period_resolution_utc_field(
    period_resolution: &mut serde_json::Value,
    field: &str,
) -> Result<(), String> {
    let raw = period_resolution
        .get(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| format!("missing period_resolution.{field}"))?;
    period_resolution[field] = serde_json::json!(normalize_rfc3339_utc(raw)?);
    Ok(())
}

/// Fonction validate_horoscope_period_request_scan_plan.
fn validate_horoscope_period_request_scan_plan(
    request: &HoroscopePeriodCalculationRequest,
) -> Result<(), String> {
    if request.scan_plan.snapshot_count != request.scan_plan.snapshots.len() as i32 {
        return Err("scan_plan.snapshot_count mismatch".to_string());
    }
    let start = request
        .period_resolution
        .get("start_datetime_utc")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "missing period_resolution.start_datetime_utc".to_string())
        .and_then(parse_rfc3339)?;
    let end = request
        .period_resolution
        .get("end_datetime_utc")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "missing period_resolution.end_datetime_utc".to_string())
        .and_then(parse_rfc3339)?;
    if end <= start {
        return Err("period end must be after start".to_string());
    }
    let mut keys = HashSet::new();
    for snapshot in &request.scan_plan.snapshots {
        if !keys.insert(snapshot.snapshot_key.as_str()) {
            return Err("scan_plan snapshot_key must be unique".to_string());
        }
        let reference = parse_rfc3339(&snapshot.reference_datetime_utc)?;
        if reference < start || reference >= end {
            return Err("scan_plan snapshot outside period".to_string());
        }
    }
    Ok(())
}

/// Fonction real_period_snapshot.
fn real_period_snapshot(
    index: usize,
    snapshot: &HoroscopeSnapshotRequest,
    natal_positions: &[&ObjectPositionFact],
    transit_positions: Option<&[ObjectPositionFact]>,
    max_major_aspect_orb_deg: f64,
    aspect_definitions: &[AspectDefinition],
    theme_mappings: &[HoroscopeSignalThemeMapping],
) -> HoroscopePeriodSnapshot {
    let Some(transit_positions) = transit_positions else {
        return HoroscopePeriodSnapshot {
            snapshot_key: snapshot.snapshot_key.clone(),
            date: snapshot.date.clone(),
            reference_datetime_utc: snapshot.reference_datetime_utc.clone(),
            sky_snapshot: serde_json::json!({
                "reference_datetime_utc": snapshot.reference_datetime_utc,
                "visible_objects": [],
                "zodiacal_reference_system": "tropical",
                "source": "missing_transit_data"
            }),
            moon_context: serde_json::json!({
                "source": "missing_transit_data"
            }),
            transits_to_natal: Vec::new(),
            current_sky_aspects: Vec::new(),
            natal_house_activations: Vec::new(),
            calculation_warnings: vec![
                "horoscope period requires real transit positions".to_string()
            ],
        };
    };
    let object = transit_positions
        .last()
        .map(|position| position.object_code.as_str())
        .unwrap_or("transit");
    let transit = preferred_transit_position(Some(transit_positions), object);
    let object = transit
        .map(|position| position.object_code.as_str())
        .unwrap_or(object);
    let source = "swisseph_period_calculator_v1";
    let natal = natal_positions
        .get(index % natal_positions.len().max(1))
        .copied();
    let fallback_natal_target = natal
        .map(|position| format!("natal_{}", position.object_code))
        .unwrap_or_else(|| "natal_sun".to_string());
    let natal_longitude = natal.map(|position| position.longitude_deg).unwrap_or(0.0);
    let transit_longitude = transit
        .map(|position| position.longitude_deg)
        .unwrap_or_else(|| normalize_degrees(natal_longitude + 12.5 + (index as f64 * 27.0)));
    let transit_match = transit.and_then(|position| {
        nearest_major_transit_match(
            position,
            natal_positions,
            max_major_aspect_orb_deg,
            aspect_definitions,
        )
    });
    let nearest_aspect =
        nearest_major_aspect_name_and_orb(transit_longitude, natal_longitude, aspect_definitions);
    let (aspect_code, orb) = transit_match
        .as_ref()
        .map(|matched| (matched.aspect_code.clone(), matched.orb_deg))
        .unwrap_or((
            "context".to_string(),
            nearest_aspect.map(|(_, orb)| orb).unwrap_or(0.0),
        ));
    let aspect = aspect_code.as_str();
    let is_context_signal = object == "moon" || transit_match.is_none();
    let natal_target = transit_match
        .as_ref()
        .map(|matched| matched.natal_target.clone())
        .unwrap_or(fallback_natal_target);
    let natal_house = if object == "moon" {
        transit
            .and_then(|position| position.house_number)
            .or_else(|| {
                Some(
                    ((natal
                        .and_then(|position| position.house_number)
                        .unwrap_or(1)
                        + index as i32
                        - 1)
                        % 12)
                        + 1,
                )
            })
    } else {
        natal.and_then(|position| position.house_number)
    };
    let theme = theme_for(
        object,
        if is_context_signal {
            None
        } else {
            Some(aspect)
        },
        Some(natal_target.as_str()),
        natal_house,
        theme_mappings,
    );
    let tone = "focused";
    let evidence_key = if object == "moon" {
        format!(
            "period:{}:{}:moon:natal_house:{}",
            snapshot.date,
            snapshot.snapshot_key,
            natal_house.unwrap_or(1)
        )
    } else if transit_match.is_none() {
        format!(
            "period:{}:{}:{}:context:{}",
            snapshot.date, snapshot.snapshot_key, object, natal_target
        )
    } else {
        format!(
            "period:{}:{}:{}:{}:{}",
            snapshot.date, snapshot.snapshot_key, object, aspect, natal_target
        )
    };

    HoroscopePeriodSnapshot {
        snapshot_key: snapshot.snapshot_key.clone(),
        date: snapshot.date.clone(),
        reference_datetime_utc: snapshot.reference_datetime_utc.clone(),
        sky_snapshot: serde_json::json!({
            "reference_datetime_utc": snapshot.reference_datetime_utc,
            "visible_objects": visible_objects(Some(transit_positions)),
            "zodiacal_reference_system": "tropical",
            "source": source
        }),
        moon_context: serde_json::json!({
            "moon_sign": transit.map(|position| position.sign_code.as_str()).unwrap_or("unknown"),
            "natal_house": natal_house,
            "priority": "period_basic",
            "theme_code": theme,
            "tone": tone,
            "source": source
        }),
        transits_to_natal: vec![HoroscopeTransitFact {
            evidence_key,
            fact_type: if object == "moon" {
                "moon_house_by_day".into()
            } else if transit_match.is_none() {
                "transit_context".into()
            } else {
                "transit_to_natal".into()
            },
            source: source.into(),
            transiting_object: object.into(),
            natal_target: if object == "moon" {
                None
            } else {
                Some(natal_target)
            },
            aspect: if is_context_signal {
                None
            } else {
                Some(aspect.into())
            },
            orb_deg: if is_context_signal {
                None
            } else {
                Some(round1(orb))
            },
            natal_house,
        }],
        current_sky_aspects: vec![serde_json::json!({
            "transiting_object": object,
            "aspect": if is_context_signal { "context" } else { aspect },
            "target": "period_tone",
            "orb_deg": if is_context_signal { serde_json::Value::Null } else { serde_json::json!(round1(orb)) },
            "source": source
        })],
        natal_house_activations: vec![serde_json::json!({
            "house": natal_house.unwrap_or(1),
            "activation": theme,
            "source": source
        })],
        calculation_warnings: Vec::new(),
    }
}

/// Fonction visible_objects.
fn visible_objects(transit_positions: Option<&[ObjectPositionFact]>) -> Vec<String> {
    transit_positions
        .into_iter()
        .flatten()
        .filter(|position| is_standard_transit_object(position.object_code.as_str()))
        .map(|position| position.object_code.clone())
        .collect::<Vec<_>>()
}

/// Fonction period_theme_for.
fn theme_for<'a>(
    object: &str,
    aspect: Option<&str>,
    natal_target: Option<&str>,
    natal_house: Option<i32>,
    theme_mappings: &'a [HoroscopeSignalThemeMapping],
) -> &'a str {
    let house_target = natal_house.map(|house| format!("natal_house_{house}"));
    theme_mappings
        .iter()
        .find(|mapping| {
            mapping.match_object == object
                && optional_match(mapping.match_aspect.as_deref(), aspect)
                && (optional_match(mapping.match_natal_target.as_deref(), natal_target)
                    || optional_match(
                        mapping.match_natal_target.as_deref(),
                        house_target.as_deref(),
                    ))
        })
        .or_else(|| {
            theme_mappings
                .iter()
                .find(|mapping| mapping.match_object == object && mapping.match_aspect.is_none())
        })
        .map(|mapping| mapping.theme_code.as_str())
        .unwrap_or("transit_context")
}

fn optional_match(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected.is_none() || expected == actual
}

/// Fonction round1.
fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

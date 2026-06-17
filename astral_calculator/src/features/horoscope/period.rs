use std::collections::HashSet;

use crate::domain::ObjectPositionFact;
use crate::shared::astro_math::{normalize_degrees, shortest_angular_distance};
use crate::shared::time::{normalize_rfc3339_utc, parse_rfc3339};

use super::{
    HoroscopePeriodCalculationRequest, HoroscopePeriodCalculationResponse, HoroscopePeriodSnapshot,
    HoroscopeSnapshotRequest, HoroscopeTransitFact,
};

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

pub fn calculate_horoscope_period(
    request: HoroscopePeriodCalculationRequest,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_from_positions(request, &[], 8.0)
}

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

pub fn calculate_horoscope_period_from_transits(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_snapshots: &[(String, Vec<ObjectPositionFact>)],
    max_major_aspect_orb_deg: f64,
) -> HoroscopePeriodCalculationResponse {
    let request = normalize_horoscope_period_request_utc(request).unwrap_or_else(|request| {
        panic!("invalid horoscope period calculation request: {request}")
    });
    let usable_positions = natal_positions
        .iter()
        .filter(|position| {
            matches!(
                position.object_code.as_str(),
                "sun" | "moon" | "mercury" | "venus" | "mars" | "jupiter" | "saturn"
            )
        })
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
            )
        })
        .collect::<Vec<_>>();
    let evidence_keys = snapshots
        .iter()
        .flat_map(|snapshot| snapshot.transits_to_natal.iter())
        .map(|fact| fact.evidence_key.clone())
        .collect::<Vec<_>>();

    HoroscopePeriodCalculationResponse {
        contract_version: "horoscope_period_calculation_response".into(),
        service_code: request.service_code,
        period_resolution: request.period_resolution,
        scan_plan: request.scan_plan,
        snapshots,
        calculation_warnings: Vec::new(),
        evidence_keys,
    }
}

pub fn calculate_horoscope_period_natal(
    request: HoroscopePeriodCalculationRequest,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period(request)
}

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

fn real_period_snapshot(
    index: usize,
    snapshot: &HoroscopeSnapshotRequest,
    natal_positions: &[&ObjectPositionFact],
    transit_positions: Option<&[ObjectPositionFact]>,
    max_major_aspect_orb_deg: f64,
) -> HoroscopePeriodSnapshot {
    let transit_objects = ["moon", "venus", "mars", "sun", "mercury", "moon", "jupiter"];
    let object = transit_objects[index % transit_objects.len()];
    let transit = transit_positions
        .into_iter()
        .flatten()
        .find(|position| position.object_code == object);
    let source = if transit_positions.is_some() {
        "swisseph_period_calculator_v1"
    } else {
        "derived_period_calculator_v1"
    };
    let natal = natal_positions
        .get(index % natal_positions.len().max(1))
        .copied();
    let natal_target = natal
        .map(|position| format!("natal_{}", position.object_code))
        .unwrap_or_else(|| "natal_sun".to_string());
    let natal_longitude = natal.map(|position| position.longitude_deg).unwrap_or(0.0);
    let transit_longitude = transit
        .map(|position| position.longitude_deg)
        .unwrap_or_else(|| normalize_degrees(natal_longitude + 12.5 + (index as f64 * 27.0)));
    let nearest_aspect = nearest_major_aspect(transit_longitude, natal_longitude);
    let valid_aspect = nearest_aspect.filter(|(_, orb)| *orb <= max_major_aspect_orb_deg);
    let (aspect, orb) =
        valid_aspect.unwrap_or(("context", nearest_aspect.map(|(_, orb)| orb).unwrap_or(0.0)));
    let is_context_signal = object == "moon" || valid_aspect.is_none();
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
    let theme = period_theme_for(object, aspect, natal_house);
    let tone = period_tone_for(aspect);
    let evidence_key = if object == "moon" {
        format!(
            "period:{}:{}:moon:natal_house:{}",
            snapshot.date,
            snapshot.snapshot_key,
            natal_house.unwrap_or(1)
        )
    } else if valid_aspect.is_none() {
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
            "visible_objects": visible_objects(transit_positions),
            "zodiacal_reference_system": "tropical",
            "source": source
        }),
        moon_context: serde_json::json!({
            "moon_sign": sign_for_longitude(transit_longitude),
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
            } else if valid_aspect.is_none() {
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

fn visible_objects(transit_positions: Option<&[ObjectPositionFact]>) -> Vec<String> {
    let objects = transit_positions
        .into_iter()
        .flatten()
        .filter(|position| {
            matches!(
                position.object_code.as_str(),
                "sun" | "moon" | "mercury" | "venus" | "mars" | "jupiter" | "saturn"
            )
        })
        .map(|position| position.object_code.clone())
        .collect::<Vec<_>>();
    if objects.is_empty() {
        return [
            "sun", "moon", "mercury", "venus", "mars", "jupiter", "saturn",
        ]
        .iter()
        .map(|code| (*code).to_string())
        .collect();
    }
    objects
}

fn nearest_major_aspect(left: f64, right: f64) -> Option<(&'static str, f64)> {
    let separation = shortest_angular_distance(left, right);
    let mut best = ("conjunction", separation.abs());
    for (name, angle) in [
        ("sextile", 60.0),
        ("square", 90.0),
        ("trine", 120.0),
        ("opposition", 180.0),
    ] {
        let orb = (separation - angle).abs();
        if orb < best.1 {
            best = (name, orb);
        }
    }
    Some(best)
}

fn sign_for_longitude(longitude: f64) -> &'static str {
    match (normalize_degrees(longitude) / 30.0).floor() as i32 {
        0 => "aries",
        1 => "taurus",
        2 => "gemini",
        3 => "cancer",
        4 => "leo",
        5 => "virgo",
        6 => "libra",
        7 => "scorpio",
        8 => "sagittarius",
        9 => "capricorn",
        10 => "aquarius",
        _ => "pisces",
    }
}

fn period_theme_for(object: &str, aspect: &str, natal_house: Option<i32>) -> &'static str {
    if object == "moon" {
        return match natal_house.unwrap_or(1) {
            2 | 6 => "organization",
            3 | 7 => "relationship",
            _ => "routine",
        };
    }
    match (object, aspect) {
        ("venus", _) => "relationship",
        ("mars", "square") | ("mars", "opposition") => "energy",
        ("mercury", _) => "communication",
        ("jupiter", _) => "integration",
        ("sun", _) => "clarity",
        _ => "organization",
    }
}

fn period_tone_for(aspect: &str) -> &'static str {
    match aspect {
        "square" | "opposition" => "careful",
        "trine" | "sextile" => "supportive",
        "conjunction" => "active",
        _ => "focused",
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

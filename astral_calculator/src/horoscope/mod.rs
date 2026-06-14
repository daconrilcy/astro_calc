mod builders;

use std::collections::HashSet;

pub use astral_contracts::{
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
};
pub use builders::{
    build_horoscope_daily_calculation_request_from_public,
    build_horoscope_period_calculation_request_from_public,
};
use serde::{Deserialize, Serialize};

use crate::domain::ObjectPositionFact;
const HOROSCOPE_ORB_WEIGHT_BANDS_JSON: &str =
    include_str!("../../../json_db/horoscope_orb_weight_bands.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeCalculationRequest {
    pub contract_version: String,
    pub service_code: String,
    pub period: HoroscopePeriod,
    pub chart_calculation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<HoroscopeLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_profile_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_system_code: Option<String>,
    #[serde(default)]
    pub calculation_features: Vec<String>,
    pub slots: Vec<HoroscopeCalculationSlotRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeLocation {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopePeriod {
    pub date: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeCalculationSlotRequest {
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeCalculationResponse {
    pub contract_version: String,
    pub service_code: String,
    pub period: HoroscopePeriod,
    pub slots: Vec<HoroscopeCalculationSlot>,
    pub calculation_warnings: Vec<String>,
    pub evidence_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeCalculationSlot {
    pub slot_code: String,
    pub reference_local_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_datetime_utc: Option<String>,
    pub sky_snapshot: serde_json::Value,
    pub moon_context: serde_json::Value,
    pub transits_to_natal: Vec<HoroscopeTransitFact>,
    pub current_sky_aspects: Vec<serde_json::Value>,
    pub natal_house_activations: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_chart: Option<serde_json::Value>,
    #[serde(default)]
    pub local_house_placements: Vec<serde_json::Value>,
    #[serde(default)]
    pub angle_activations: Vec<serde_json::Value>,
    #[serde(default)]
    pub calculation_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeTransitFact {
    pub evidence_key: String,
    pub fact_type: String,
    pub source: String,
    pub transiting_object: String,
    pub natal_target: Option<String>,
    pub aspect: Option<String>,
    pub orb_deg: Option<f64>,
    pub natal_house: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopePeriodCalculationRequest {
    pub contract_version: String,
    pub service_code: String,
    pub chart_calculation_id: String,
    pub period_resolution: serde_json::Value,
    pub scan_plan: HoroscopeScanPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeScanPlan {
    pub scan_profile_code: String,
    pub granularity: String,
    pub snapshot_count: i32,
    pub snapshots: Vec<HoroscopeSnapshotRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeSnapshotRequest {
    pub snapshot_key: String,
    pub date: String,
    pub reference_time_local: String,
    pub reference_datetime_local: String,
    pub reference_datetime_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopePeriodCalculationResponse {
    pub contract_version: String,
    pub service_code: String,
    pub period_resolution: serde_json::Value,
    pub scan_plan: HoroscopeScanPlan,
    pub snapshots: Vec<HoroscopePeriodSnapshot>,
    pub calculation_warnings: Vec<String>,
    pub evidence_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopePeriodSnapshot {
    pub snapshot_key: String,
    pub date: String,
    pub reference_datetime_utc: String,
    pub sky_snapshot: serde_json::Value,
    pub moon_context: serde_json::Value,
    pub transits_to_natal: Vec<HoroscopeTransitFact>,
    pub current_sky_aspects: Vec<serde_json::Value>,
    pub natal_house_activations: Vec<serde_json::Value>,
    #[serde(default)]
    pub calculation_warnings: Vec<String>,
}

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

fn normalize_rfc3339_utc(raw: &str) -> Result<String, String> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&chrono::Utc).to_rfc3339())
        .map_err(|err| format!("invalid RFC3339 UTC field: {err}"))
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
        .and_then(parse_rfc3339_for_validation)?;
    let end = request
        .period_resolution
        .get("end_datetime_utc")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "missing period_resolution.end_datetime_utc".to_string())
        .and_then(parse_rfc3339_for_validation)?;
    if end <= start {
        return Err("period end must be after start".to_string());
    }
    let mut keys = HashSet::new();
    for snapshot in &request.scan_plan.snapshots {
        if !keys.insert(snapshot.snapshot_key.as_str()) {
            return Err("scan_plan snapshot_key must be unique".to_string());
        }
        let reference = parse_rfc3339_for_validation(&snapshot.reference_datetime_utc)?;
        if reference < start || reference >= end {
            return Err("scan_plan snapshot outside period".to_string());
        }
    }
    Ok(())
}

fn parse_rfc3339_for_validation(
    raw: &str,
) -> Result<chrono::DateTime<chrono::FixedOffset>, String> {
    chrono::DateTime::parse_from_rfc3339(raw).map_err(|err| format!("invalid RFC3339 field: {err}"))
}

pub fn calculate_horoscope_daily_natal(
    request: HoroscopeCalculationRequest,
) -> HoroscopeCalculationResponse {
    let service_code = request.service_code.clone();
    let slots = request
        .slots
        .iter()
        .enumerate()
        .map(|(idx, slot)| fake_slot(&request, slot, idx))
        .collect::<Vec<_>>();
    let evidence_keys = slots
        .iter()
        .flat_map(|slot| slot.transits_to_natal.iter())
        .map(|fact| fact.evidence_key.clone())
        .collect();

    HoroscopeCalculationResponse {
        contract_version: "horoscope_calculation_response".into(),
        service_code,
        period: request.period,
        slots,
        calculation_warnings: Vec::new(),
        evidence_keys,
    }
}

pub fn calculate_horoscope_period_natal(
    request: HoroscopePeriodCalculationRequest,
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_natal_from_positions(request, &[])
}

pub fn calculate_horoscope_period_natal_from_positions(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
) -> HoroscopePeriodCalculationResponse {
    calculate_horoscope_period_natal_from_transits(request, natal_positions, &[])
}

pub fn calculate_horoscope_period_natal_from_transits(
    request: HoroscopePeriodCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_snapshots: &[(String, Vec<ObjectPositionFact>)],
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
            real_period_snapshot(index, snapshot, &usable_positions, transit_positions)
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

fn real_period_snapshot(
    index: usize,
    snapshot: &HoroscopeSnapshotRequest,
    natal_positions: &[&ObjectPositionFact],
    transit_positions: Option<&[ObjectPositionFact]>,
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
        .unwrap_or_else(|| normalize_deg(natal_longitude + 12.5 + (index as f64 * 27.0)));
    let nearest_aspect = nearest_major_aspect(transit_longitude, natal_longitude);
    let valid_aspect = nearest_aspect.filter(|(_, orb)| *orb <= period_max_major_aspect_orb_deg());
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
    let separation = angular_separation(left, right);
    let mut best = ("conjunction", (separation - 0.0).abs());
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

fn period_max_major_aspect_orb_deg() -> f64 {
    serde_json::from_str::<serde_json::Value>(HOROSCOPE_ORB_WEIGHT_BANDS_JSON)
        .ok()
        .and_then(|value| {
            value
                .get("data")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|row| row.get("max_orb_deg").and_then(serde_json::Value::as_f64))
                .filter(|orb| orb.is_finite() && *orb > 0.0)
                .max_by(|left, right| left.total_cmp(right))
        })
        .expect("json_db/horoscope_orb_weight_bands.json must define positive max_orb_deg values")
}

fn angular_separation(left: f64, right: f64) -> f64 {
    let diff = (normalize_deg(left) - normalize_deg(right)).abs();
    if diff > 180.0 {
        360.0 - diff
    } else {
        diff
    }
}

fn normalize_deg(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

fn sign_for_longitude(longitude: f64) -> &'static str {
    match (normalize_deg(longitude) / 30.0).floor() as i32 {
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

fn fake_slot(
    request: &HoroscopeCalculationRequest,
    slot: &HoroscopeCalculationSlotRequest,
    index: usize,
) -> HoroscopeCalculationSlot {
    if request.service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return fake_premium_slot(request, slot, index);
    }
    let (moon_sign, facts) = match slot.slot_code.as_str() {
        "day" => (
            "virgo",
            vec![HoroscopeTransitFact {
                evidence_key: "slot:day:moon:natal_house:6".into(),
                fact_type: "moon_natal_house_activation".into(),
                source: "fake_calculator_v1".into(),
                transiting_object: "moon".into(),
                natal_target: Some("natal_house_6".into()),
                aspect: None,
                orb_deg: Some(0.8),
                natal_house: Some(6),
            }],
        ),
        "morning" => (
            "virgo",
            vec![HoroscopeTransitFact {
                evidence_key: "slot:morning:moon:natal_house:6".into(),
                fact_type: "moon_natal_house_activation".into(),
                source: "fake_calculator_v1".into(),
                transiting_object: "moon".into(),
                natal_target: Some("natal_house_6".into()),
                aspect: None,
                orb_deg: Some(0.8),
                natal_house: Some(6),
            }],
        ),
        "afternoon" => (
            "libra",
            vec![HoroscopeTransitFact {
                evidence_key: "slot:afternoon:mars:square:natal_moon".into(),
                fact_type: "transit_to_natal".into(),
                source: "fake_calculator_v1".into(),
                transiting_object: "mars".into(),
                natal_target: Some("natal_moon".into()),
                aspect: Some("square".into()),
                orb_deg: Some(2.2),
                natal_house: None,
            }],
        ),
        _ => (
            "libra",
            vec![HoroscopeTransitFact {
                evidence_key: "slot:evening:venus:trine:natal_mercury".into(),
                fact_type: "transit_to_natal".into(),
                source: "fake_calculator_v1".into(),
                transiting_object: "venus".into(),
                natal_target: Some("natal_mercury".into()),
                aspect: Some("trine".into()),
                orb_deg: Some(1.4),
                natal_house: None,
            }],
        ),
    };

    HoroscopeCalculationSlot {
        slot_code: slot.slot_code.clone(),
        reference_local_time: slot.reference_local_time.clone(),
        reference_datetime_utc: None,
        sky_snapshot: serde_json::json!({
            "reference_local_time": slot.reference_local_time,
            "visible_objects": ["moon", "venus", "mars"],
            "zodiacal_reference_system": "tropical"
        }),
        moon_context: serde_json::json!({
            "moon_sign": moon_sign,
            "priority": "primary"
        }),
        transits_to_natal: facts,
        current_sky_aspects: vec![serde_json::json!({
            "transiting_object": "moon",
            "aspect": "conjunction",
            "target": "day_tone",
            "orb_deg": 1.0
        })],
        natal_house_activations: vec![serde_json::json!({
            "house": 6,
            "activation": "routine"
        })],
        local_chart: None,
        local_house_placements: Vec::new(),
        angle_activations: Vec::new(),
        calculation_warnings: Vec::new(),
    }
}

fn fake_premium_slot(
    request: &HoroscopeCalculationRequest,
    slot: &HoroscopeCalculationSlotRequest,
    index: usize,
) -> HoroscopeCalculationSlot {
    let reference_datetime_utc = reference_datetime_utc(
        &request.period.date,
        &request.period.timezone,
        &slot.reference_local_time,
    )
    .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    let house_system_code = request
        .house_system_code
        .as_deref()
        .unwrap_or("missing_house_system")
        .to_string();
    let aspect = match index % 4 {
        0 => "trine",
        1 => "conjunction",
        2 => "square",
        _ => "trine",
    };
    let object = match index % 3 {
        0 => "moon",
        1 => "venus",
        _ => "mars",
    };
    let target = match index % 4 {
        0 => "natal_moon",
        1 => "natal_mercury",
        2 => "natal_sun",
        _ => "natal_venus",
    };
    let evidence_key = format!("slot:{}:{}:{}:{}", slot.slot_code, object, aspect, target);
    let sign = match index % 6 {
        0 => "virgo",
        1 => "libra",
        2 => "scorpio",
        3 => "sagittarius",
        4 => "capricorn",
        _ => "aquarius",
    };
    let warnings = if slot.slot_code == "slot_22_00" {
        vec!["FAKE_PREMIUM_LOCAL_DATA_STABLE_FOR_TESTS".to_string()]
    } else {
        Vec::new()
    };

    HoroscopeCalculationSlot {
        slot_code: slot.slot_code.clone(),
        reference_local_time: slot.reference_local_time.clone(),
        reference_datetime_utc: Some(reference_datetime_utc),
        sky_snapshot: serde_json::json!({
            "reference_local_time": slot.reference_local_time,
            "visible_objects": ["sun", "moon", "mercury", "venus", "mars"],
            "zodiacal_reference_system": "tropical"
        }),
        moon_context: serde_json::json!({
            "sign": sign,
            "moon_sign": sign,
            "natal_house": (index % 12) + 1,
            "local_house": ((index + 2) % 12) + 1,
            "phase": "waxing_gibbous",
            "aspects_to_natal": [evidence_key]
        }),
        transits_to_natal: vec![HoroscopeTransitFact {
            evidence_key,
            fact_type: "transit_to_natal".into(),
            source: "fake_calculator_premium_v1".into(),
            transiting_object: object.into(),
            natal_target: Some(target.into()),
            aspect: Some(aspect.into()),
            orb_deg: Some(0.7 + (index as f64 * 0.1)),
            natal_house: Some(((index % 12) + 1) as i32),
        }],
        current_sky_aspects: vec![serde_json::json!({
            "transiting_object": object,
            "aspect": aspect,
            "target": "daily_tone",
            "orb_deg": 1.0
        })],
        natal_house_activations: vec![serde_json::json!({
            "house": (index % 12) + 1,
            "activation": "premium_slot_focus"
        })],
        local_chart: Some(serde_json::json!({
            "house_system_code": house_system_code,
            "ascendant": {
                "sign": sign,
                "longitude_deg": round1(10.0 + (index as f64 * 14.0))
            },
            "midheaven": {
                "sign": match index % 4 {
                    0 => "gemini",
                    1 => "cancer",
                    2 => "leo",
                    _ => "virgo",
                },
                "longitude_deg": round1(40.0 + (index as f64 * 11.0))
            },
            "houses": (1..=12).map(|house| serde_json::json!({
                "house": house,
                "longitude_deg": round1(((house * 30) as f64 + index as f64) % 360.0)
            })).collect::<Vec<_>>()
        })),
        local_house_placements: vec![serde_json::json!({
            "object": object,
            "local_house": ((index + 2) % 12) + 1
        })],
        angle_activations: vec![serde_json::json!({
            "angle": "ascendant",
            "object": object,
            "orb_deg": 1.2
        })],
        calculation_warnings: warnings,
    }
}

fn reference_datetime_utc(date: &str, timezone: &str, time: &str) -> Option<String> {
    use chrono::{NaiveDate, NaiveTime, TimeZone};
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    let tz = timezone.parse::<chrono_tz::Tz>().ok()?;
    let local = date.and_time(time);
    let resolved = tz.from_local_datetime(&local).single()?;
    Some(resolved.with_timezone(&chrono::Utc).to_rfc3339())
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

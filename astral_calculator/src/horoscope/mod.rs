use serde::{Deserialize, Serialize};

pub const HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";
pub const HOROSCOPE_FREE_DAILY_SERVICE_CODE: &str = "horoscope_free_daily";
pub const HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE: &str =
    "horoscope_premium_daily_local_2h_slots";

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
        contract_version: "horoscope_calculation_response_v1".into(),
        service_code,
        period: request.period,
        slots,
        calculation_warnings: Vec::new(),
        evidence_keys,
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
    let reference_datetime_utc =
        reference_datetime_utc(&request.period.date, &request.period.timezone, &slot.reference_local_time)
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

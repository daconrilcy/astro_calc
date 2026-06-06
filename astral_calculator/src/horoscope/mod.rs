use serde::{Deserialize, Serialize};

pub const HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";
pub const HOROSCOPE_FREE_DAILY_SERVICE_CODE: &str = "horoscope_free_daily";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeCalculationRequest {
    pub contract_version: String,
    pub service_code: String,
    pub period: HoroscopePeriod,
    pub chart_calculation_id: String,
    pub slots: Vec<HoroscopeCalculationSlotRequest>,
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
    pub sky_snapshot: serde_json::Value,
    pub moon_context: serde_json::Value,
    pub transits_to_natal: Vec<HoroscopeTransitFact>,
    pub current_sky_aspects: Vec<serde_json::Value>,
    pub natal_house_activations: Vec<serde_json::Value>,
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
        .map(|slot| fake_slot(slot))
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

fn fake_slot(slot: &HoroscopeCalculationSlotRequest) -> HoroscopeCalculationSlot {
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
    }
}

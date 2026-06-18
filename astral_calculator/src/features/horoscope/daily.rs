//! Calcul horoscope quotidien.

use crate::astrology::transits::{
    is_standard_transit_object, nearest_major_aspect_name_and_orb, nearest_major_transit_match,
    preferred_transit_position,
};
use crate::domain::{AspectDefinition, ObjectPositionFact};
use crate::shared::astro_math::normalize_degrees;
use crate::shared::time::reference_datetime_utc;

use super::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopeCalculationSlot,
    HoroscopeCalculationSlotRequest, HoroscopeTransitFact,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
};

/// Fonction calculate_horoscope_daily.
pub fn calculate_horoscope_daily(
    request: HoroscopeCalculationRequest,
) -> HoroscopeCalculationResponse {
    calculate_horoscope_daily_from_transits(request, &[], &[], 8.0, &[])
}

/// Fonction calculate_horoscope_daily_natal.
pub fn calculate_horoscope_daily_natal(
    request: HoroscopeCalculationRequest,
) -> HoroscopeCalculationResponse {
    calculate_horoscope_daily(request)
}

/// Fonction calculate_horoscope_daily_from_transits.
pub fn calculate_horoscope_daily_from_transits(
    request: HoroscopeCalculationRequest,
    natal_positions: &[ObjectPositionFact],
    transit_slots: &[(String, Vec<ObjectPositionFact>)],
    max_major_aspect_orb_deg: f64,
    aspect_definitions: &[AspectDefinition],
) -> HoroscopeCalculationResponse {
    let service_code = request.service_code.clone();
    let slots = request
        .slots
        .iter()
        .enumerate()
        .map(|(idx, slot)| {
            let transit_positions = transit_slots
                .iter()
                .find(|(key, _)| key == &slot.slot_code)
                .map(|(_, positions)| positions.as_slice());
            derived_slot(
                &request,
                slot,
                idx,
                natal_positions,
                transit_positions,
                max_major_aspect_orb_deg,
                aspect_definitions,
            )
        })
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

#[allow(clippy::too_many_arguments)]
/// Fonction derived_slot.
fn derived_slot(
    request: &HoroscopeCalculationRequest,
    slot: &HoroscopeCalculationSlotRequest,
    index: usize,
    natal_positions: &[ObjectPositionFact],
    transit_positions: Option<&[ObjectPositionFact]>,
    max_major_aspect_orb_deg: f64,
    aspect_definitions: &[AspectDefinition],
) -> HoroscopeCalculationSlot {
    let source = if transit_positions.is_some() {
        "swisseph_daily_calculator_v1"
    } else {
        "derived_daily_calculator_v1"
    };
    let premium_local = request.service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE;
    let reference_datetime_utc = if premium_local || transit_positions.is_some() {
        reference_datetime_utc(
            &request.period.date,
            &request.period.timezone,
            &slot.reference_local_time,
        )
    } else {
        None
    };
    let usable_natal = natal_positions
        .iter()
        .filter(|position| is_standard_transit_object(position.object_code.as_str()))
        .collect::<Vec<_>>();
    let fallback_transit_object = transit_object_for_slot(index);
    let transit = preferred_transit_position(transit_positions, fallback_transit_object);
    let transit_object = transit
        .map(|position| position.object_code.as_str())
        .unwrap_or(fallback_transit_object);
    let fallback_natal = usable_natal.get(index % usable_natal.len().max(1)).copied();
    let fallback_natal_target = fallback_natal
        .map(|position| format!("natal_{}", position.object_code))
        .unwrap_or_else(|| "natal_sun".to_string());
    let natal_longitude = fallback_natal
        .map(|position| position.longitude_deg)
        .unwrap_or(0.0);
    let transit_longitude = transit
        .map(|position| position.longitude_deg)
        .unwrap_or_else(|| normalize_degrees(natal_longitude + 18.0 + (index as f64 * 41.0)));
    let transit_match = transit.and_then(|position| {
        nearest_major_transit_match(
            position,
            &usable_natal,
            max_major_aspect_orb_deg,
            aspect_definitions,
        )
    });
    let nearest_aspect =
        nearest_major_aspect_name_and_orb(transit_longitude, natal_longitude, aspect_definitions);
    let (aspect, orb) = transit_match
        .as_ref()
        .map(|matched| (matched.aspect_code.as_str(), matched.orb_deg))
        .unwrap_or_else(|| nearest_aspect.unwrap_or(("context", 0.0)));
    let is_context_signal = transit_object == "moon" || transit_match.is_none();
    let natal_house = if transit_object == "moon" {
        transit
            .and_then(|position| position.house_number)
            .or_else(|| Some(((index as i32 + 5) % 12) + 1))
    } else {
        transit_match
            .as_ref()
            .and_then(|matched| matched.natal_house)
            .or_else(|| fallback_natal.and_then(|position| position.house_number))
    };
    let natal_target = transit_match
        .as_ref()
        .map(|matched| matched.natal_target.clone())
        .unwrap_or(fallback_natal_target);
    let theme = daily_theme_for(transit_object, aspect, natal_house);
    let evidence_key = if transit_object == "moon" {
        format!(
            "slot:{}:moon:natal_house:{}",
            slot.slot_code,
            natal_house.unwrap_or(1)
        )
    } else if transit_match.is_none() {
        format!(
            "slot:{}:{}:context:{}",
            slot.slot_code, transit_object, natal_target
        )
    } else {
        format!(
            "slot:{}:{}:{}:{}",
            slot.slot_code, transit_object, aspect, natal_target
        )
    };
    let sign = transit
        .map(|position| position.sign_code.as_str())
        .unwrap_or_else(|| sign_for_longitude(transit_longitude));
    let house_system_code = request
        .house_system_code
        .as_deref()
        .unwrap_or("unspecified_house_system");

    HoroscopeCalculationSlot {
        slot_code: slot.slot_code.clone(),
        reference_local_time: slot.reference_local_time.clone(),
        reference_datetime_utc,
        sky_snapshot: serde_json::json!({
            "reference_local_time": slot.reference_local_time,
            "visible_objects": visible_objects(transit_positions),
            "zodiacal_reference_system": "tropical",
            "source": source
        }),
        moon_context: serde_json::json!({
            "sign": sign,
            "moon_sign": sign,
            "natal_house": natal_house,
            "priority": if premium_local { "premium_slot" } else { "daily_basic" },
            "theme_code": theme,
            "source": source
        }),
        transits_to_natal: vec![HoroscopeTransitFact {
            evidence_key,
            fact_type: if transit_object == "moon" {
                "moon_natal_house_activation".into()
            } else if transit_match.is_none() {
                "transit_context".into()
            } else {
                "transit_to_natal".into()
            },
            source: source.into(),
            transiting_object: transit_object.into(),
            natal_target: if transit_object == "moon" {
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
            "transiting_object": transit_object,
            "aspect": if is_context_signal { "context" } else { aspect },
            "target": "daily_tone",
            "orb_deg": if is_context_signal { serde_json::Value::Null } else { serde_json::json!(round1(orb)) },
            "source": source
        })],
        natal_house_activations: vec![serde_json::json!({
            "house": natal_house.unwrap_or(1),
            "activation": theme,
            "source": source
        })],
        local_chart: if premium_local {
            Some(serde_json::json!({
                "house_system_code": house_system_code,
                "ascendant": {
                    "sign": sign,
                    "longitude_deg": round1(normalize_degrees(transit_longitude + 90.0))
                },
                "midheaven": {
                    "sign": sign_for_longitude(transit_longitude + 180.0),
                    "longitude_deg": round1(normalize_degrees(transit_longitude + 180.0))
                },
                "houses": (1..=12).map(|house| serde_json::json!({
                    "house": house,
                    "longitude_deg": round1(((house * 30) as f64 + index as f64) % 360.0)
                })).collect::<Vec<_>>(),
                "source": source
            }))
        } else {
            None
        },
        local_house_placements: if premium_local {
            vec![serde_json::json!({
                "object": transit_object,
                "local_house": natal_house.unwrap_or(1),
                "source": source
            })]
        } else {
            Vec::new()
        },
        angle_activations: if premium_local {
            vec![serde_json::json!({
                "angle": "ascendant",
                "object": transit_object,
                "orb_deg": 1.2,
                "source": source
            })]
        } else {
            Vec::new()
        },
        calculation_warnings: Vec::new(),
    }
}

fn transit_object_for_slot(index: usize) -> &'static str {
    let preferred = [
        "moon", "venus", "mars", "sun", "mercury", "jupiter", "saturn",
    ];
    preferred[index % preferred.len()]
}

fn visible_objects(transit_positions: Option<&[ObjectPositionFact]>) -> Vec<String> {
    let objects = transit_positions
        .into_iter()
        .flatten()
        .filter(|position| is_standard_transit_object(position.object_code.as_str()))
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

fn daily_theme_for(object: &str, aspect: &str, natal_house: Option<i32>) -> &'static str {
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

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

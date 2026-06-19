//! Calcul horoscope quotidien.

use crate::astrology::angles::normalize_degrees;
use crate::astrology::transits::{
    is_standard_transit_object, nearest_major_aspect_name_and_orb, nearest_major_transit_match,
    preferred_transit_position,
};
use crate::domain::{AspectDefinition, ObjectPositionFact};
use crate::shared::time::reference_datetime_utc;

use super::{
    HoroscopeCalculationRequest, HoroscopeCalculationResponse, HoroscopeCalculationSlot,
    HoroscopeCalculationSlotRequest, HoroscopeSignalThemeMapping, HoroscopeTransitFact,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
};

/// Fonction calculate_horoscope_daily.
pub fn calculate_horoscope_daily(
    request: HoroscopeCalculationRequest,
) -> HoroscopeCalculationResponse {
    calculate_horoscope_daily_from_transits(request, &[], &[], 8.0, &[], &[])
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
    theme_mappings: &[HoroscopeSignalThemeMapping],
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
                theme_mappings,
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
    theme_mappings: &[HoroscopeSignalThemeMapping],
) -> HoroscopeCalculationSlot {
    let Some(transit_positions) = transit_positions else {
        return HoroscopeCalculationSlot {
            slot_code: slot.slot_code.clone(),
            reference_local_time: slot.reference_local_time.clone(),
            reference_datetime_utc: None,
            sky_snapshot: serde_json::json!({
                "reference_local_time": slot.reference_local_time,
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
            local_chart: None,
            local_house_placements: Vec::new(),
            angle_activations: Vec::new(),
            calculation_warnings: vec![
                "horoscope daily requires real transit positions".to_string()
            ],
        };
    };
    let source = "swisseph_daily_calculator_v1";
    let premium_local = request.service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE;
    let reference_datetime_utc = reference_datetime_utc(
        &request.period.date,
        &request.period.timezone,
        &slot.reference_local_time,
    );
    let usable_natal = natal_positions
        .iter()
        .filter(|position| is_standard_transit_object(position.object_code.as_str()))
        .collect::<Vec<_>>();
    let fallback_transit_object = transit_positions
        .first()
        .map(|position| position.object_code.as_str())
        .unwrap_or("transit");
    let transit = preferred_transit_position(Some(transit_positions), fallback_transit_object);
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
    let (aspect_code, orb) = transit_match
        .as_ref()
        .map(|matched| (matched.aspect_code.clone(), matched.orb_deg))
        .unwrap_or_else(|| nearest_aspect.unwrap_or(("context".to_string(), 0.0)));
    let aspect = aspect_code.as_str();
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
    let theme = theme_for(
        transit_object,
        if is_context_signal {
            None
        } else {
            Some(aspect)
        },
        Some(natal_target.as_str()),
        natal_house,
        theme_mappings,
    );
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
        .unwrap_or("unknown");
    HoroscopeCalculationSlot {
        slot_code: slot.slot_code.clone(),
        reference_local_time: slot.reference_local_time.clone(),
        reference_datetime_utc,
        sky_snapshot: serde_json::json!({
            "reference_local_time": slot.reference_local_time,
            "visible_objects": visible_objects(Some(transit_positions)),
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
        local_chart: None,
        local_house_placements: if premium_local {
            vec![serde_json::json!({
                "object": transit_object,
                "local_house": natal_house.unwrap_or(1),
                "source": source
            })]
        } else {
            Vec::new()
        },
        angle_activations: Vec::new(),
        calculation_warnings: Vec::new(),
    }
}

fn visible_objects(transit_positions: Option<&[ObjectPositionFact]>) -> Vec<String> {
    transit_positions
        .into_iter()
        .flatten()
        .filter(|position| is_standard_transit_object(position.object_code.as_str()))
        .map(|position| position.object_code.clone())
        .collect::<Vec<_>>()
}

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

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

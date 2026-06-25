//! Module astral_calculator\src\features\natal\signals\clusters.rs du moteur astral_calculator.

use std::collections::HashMap;

use serde_json::json;

use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft, ObjectPositionFact};
use crate::features::natal::catalog::BasicPayloadCatalog;

use super::angles::is_core_chart_object;
use super::constants::{SIGNAL_PREFIX_CLUSTER, SUPPRESSION_ACTIVE, SUPPRESSION_MERGED};
use super::positions::{house_theme_code, object_source_weight, position_priority};
use super::tags::cluster_semantic_tags;
use super::utils::round4;

pub(super) fn add_position_cluster_signals(
    facts: &CalculatedChartFacts,
    signals: &mut Vec<InterpretationSignalDraft>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
) {
    let mut sign_house_groups: HashMap<(String, i32), Vec<&ObjectPositionFact>> = HashMap::new();

    for position in &facts.positions {
        let Some(house_number) = position.house_number else {
            continue;
        };
        sign_house_groups
            .entry((position.sign_code.clone(), house_number))
            .or_default()
            .push(position);
    }

    let mut groups: Vec<_> = sign_house_groups
        .into_iter()
        .filter(|(_, positions)| positions.len() >= 3)
        .collect();
    groups.sort_by(|left, right| {
        right
            .1
            .len()
            .cmp(&left.1.len())
            .then_with(|| left.0.cmp(&right.0))
    });

    for ((sign_code, house_number), mut positions) in groups {
        positions.sort_by(|left, right| {
            position_priority(right, catalog)
                .partial_cmp(&position_priority(left, catalog))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.object_code.cmp(&right.object_code))
        });

        let sign_name = positions[0].sign_name.clone();
        let Some(house_name) = positions[0].house_name.clone() else {
            continue;
        };
        let source_signals: Vec<String> = positions
            .iter()
            .map(|position| format!("object_position:{}", position.object_code))
            .collect();
        let source_objects: Vec<String> = positions
            .iter()
            .map(|position| position.object_code.clone())
            .collect();
        let source_weight = round4(
            positions
                .iter()
                .map(|position| object_source_weight(position))
                .sum(),
        );
        let priority_score =
            round4((90.0 + positions.len() as f64 * 1.5 + source_weight * 2.0).min(99.0));
        let aggregation_group = format!("{sign_code}_house_{house_number}_cluster");
        let house_theme_code = house_theme_code(positions[0]);
        let semantic_tags = cluster_semantic_tags(&sign_code, house_number, &house_theme_code);

        signals.push(InterpretationSignalDraft {
            signal_key: format!("cluster:{sign_code}:house_{house_number}"),
            signal_type_id: None,
            theme_code: Some(house_theme_code.clone()),
            title: cluster_title(&sign_name, house_number, locale),
            summary: Some(cluster_summary(
                positions.len(),
                &sign_name,
                &house_name,
                locale,
            )),
            priority_score,
            confidence_score: Some(0.9),
            suppression_state: SUPPRESSION_ACTIVE.to_string(),
            payload_json: Some(json!({
                "interpretive_hint": cluster_hint(&sign_name, &house_name, locale),
                "semantic_tags": semantic_tags,
                "source_weight": source_weight,
                "aggregation_group": aggregation_group,
                "evidence": {
                    "fact_type": "position_cluster",
                    "cluster_type": "sign_house",
                    "sign_code": sign_code,
                    "sign_name": sign_name,
                    "house_number": house_number,
                    "house_name": house_name,
                    "house_theme_code": house_theme_code,
                    "source_signals": source_signals,
                    "source_objects": source_objects
                }
            })),
        });
    }
}

pub(super) fn apply_cluster_source_deduplication(
    signals: &mut [InterpretationSignalDraft],
) -> bool {
    let mut source_to_cluster: HashMap<String, String> = HashMap::new();

    for signal in signals.iter() {
        if signal.suppression_state != SUPPRESSION_ACTIVE
            || !signal.signal_key.starts_with(SIGNAL_PREFIX_CLUSTER)
        {
            continue;
        }

        let Some(source_signals) = signal
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("source_signals"))
            .and_then(|value| value.as_array())
        else {
            continue;
        };

        for source_signal in source_signals {
            if let Some(source_signal) = source_signal.as_str() {
                source_to_cluster.insert(source_signal.to_string(), signal.signal_key.clone());
            }
        }
    }

    if source_to_cluster.is_empty() {
        return false;
    }

    let mut changed = false;
    for signal in signals.iter_mut() {
        let Some(cluster_key) = source_to_cluster.get(&signal.signal_key).cloned() else {
            continue;
        };

        let object_code = signal
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("evidence"))
            .and_then(|evidence| evidence.get("object_code"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();

        if is_core_chart_object(object_code) {
            changed |= annotate_cluster_source(signal, &cluster_key, "kept");
        } else if signal.suppression_state != SUPPRESSION_MERGED {
            signal.suppression_state = SUPPRESSION_MERGED.to_string();
            changed = true;
            changed |= annotate_cluster_source(signal, &cluster_key, SUPPRESSION_MERGED);
        }
    }

    changed
}

fn cluster_title(sign_name: &str, house_number: i32, locale: &str) -> String {
    match locale {
        "fr" => format!("Forte concentration en {sign_name}, maison {house_number}"),
        "es" => format!("Fuerte concentración en {sign_name}, casa {house_number}"),
        "de" => format!("Starke Konzentration in {sign_name}, Haus {house_number}"),
        _ => format!("Strong concentration in {sign_name}, house {house_number}"),
    }
}

fn cluster_summary(count: usize, sign_name: &str, house_name: &str, locale: &str) -> String {
    match locale {
        "fr" => format!(
            "{count} facteurs du thème sont concentrés en {sign_name} et dans la maison {house_name}, ce qui donne un poids interprétatif supplémentaire à cette zone du thème."
        ),
        "es" => format!(
            "{count} factores de la carta se concentran en {sign_name} y en la casa {house_name}, lo que da un peso interpretativo extra a esta zona."
        ),
        "de" => format!(
            "{count} Faktoren konzentrieren sich in {sign_name} und im Haus {house_name}, was diesem Bereich des Horoskops zusätzliches interpretatives Gewicht verleiht."
        ),
        _ => format!(
            "{count} chart factors are concentrated in {sign_name} and the {house_name} house, giving extra interpretive weight to this area of the chart."
        ),
    }
}

fn cluster_hint(sign_name: &str, house_name: &str, locale: &str) -> String {
    match locale {
        "fr" => format!(
            "Lire cela comme un accent répété : les qualités de {sign_name} sont focalisées à travers les thèmes de la maison {house_name}."
        ),
        "es" => format!(
            "Léalo como un énfasis repetido: las cualidades de {sign_name} se concentran a través de los temas de la casa {house_name}."
        ),
        "de" => format!(
            "Lesen Sie dies als wiederholte Betonung: Die Qualitäten von {sign_name} bündeln sich durch die Themen des Hauses {house_name}."
        ),
        _ => format!(
            "Read this as a repeated emphasis: {sign_name} qualities are focused through the themes of the {house_name} house."
        ),
    }
}

/// Fonction annotate_cluster_source.
fn annotate_cluster_source(
    signal: &mut InterpretationSignalDraft,
    cluster_key: &str,
    editorial_state: &str,
) -> bool {
    let Some(payload) = signal
        .payload_json
        .as_mut()
        .and_then(|value| value.as_object_mut())
    else {
        return false;
    };

    let already_current = payload
        .get("editorial_state")
        .and_then(|state| state.get("state"))
        .and_then(|value| value.as_str())
        == Some(editorial_state)
        && payload
            .get("editorial_state")
            .and_then(|state| state.get("cluster_signal_key"))
            .and_then(|value| value.as_str())
            == Some(cluster_key);

    payload.insert(
        "editorial_state".to_string(),
        json!({
            "state": editorial_state,
            "reason": "source_signal_of_active_cluster",
            "cluster_signal_key": cluster_key
        }),
    );

    !already_current
}

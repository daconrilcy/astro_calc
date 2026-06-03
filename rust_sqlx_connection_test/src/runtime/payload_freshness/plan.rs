use std::collections::{HashMap, HashSet};

use crate::domain::{
    BasicEmphasisRefs, BasicPayload, BasicReadingPlanItem, BasicSecondarySlotCandidate, BasicSignal,
};

use super::text;

pub(super) fn has_current_reading_plan(payload: &BasicPayload) -> bool {
    if payload.reading_plan.is_empty() {
        return false;
    }

    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let primary_signal_slots: HashMap<&str, &str> = payload
        .reading_plan
        .iter()
        .flat_map(|item| {
            item.source_signal_keys
                .iter()
                .map(move |signal_key| (signal_key.as_str(), item.slot.as_str()))
        })
        .collect();
    let mut slots = HashSet::new();
    let mut primary_signal_keys = HashSet::new();
    let mut previous_slot_order = None;

    payload.reading_plan.iter().all(|item| {
        let slot = item.slot.trim();
        let Some(slot_order) = reading_slot_order(slot) else {
            return false;
        };
        let is_in_order = previous_slot_order.is_none_or(|previous| previous < slot_order);
        previous_slot_order = Some(slot_order);

        !slot.is_empty()
            && slots.insert(slot)
            && is_in_order
            && !item.title.trim().is_empty()
            && !item.source_signal_keys.is_empty()
            && item.primary_signal_keys == item.source_signal_keys
            && item
                .source_signal_keys
                .iter()
                .all(|signal_key| primary_signal_keys.insert(signal_key.as_str()))
            && secondary_candidates_are_valid(item, &signal_keys, &primary_signal_slots)
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
    })
}

pub(super) fn has_current_drafting_plan(payload: &BasicPayload) -> bool {
    if payload.drafting_plan.is_empty() || payload.drafting_plan.len() != payload.reading_plan.len()
    {
        return false;
    }

    let reading_sources_by_slot: HashMap<&str, &[String]> = payload
        .reading_plan
        .iter()
        .map(|item| (item.slot.as_str(), item.source_signal_keys.as_slice()))
        .collect();
    let reading_items_by_slot: HashMap<&str, &BasicReadingPlanItem> = payload
        .reading_plan
        .iter()
        .map(|item| (item.slot.as_str(), item))
        .collect();
    let signal_keys: HashSet<&str> = payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect();
    let mut slots = HashSet::new();
    let has_dominant_cluster = payload
        .reading_plan
        .iter()
        .any(|item| item.slot == "dominant_cluster");

    payload.drafting_plan.iter().all(|item| {
        let slot = item.slot.trim();
        !slot.is_empty()
            && slots.insert(slot)
            && reading_sources_by_slot
                .get(slot)
                .is_some_and(|reading_sources| {
                    *reading_sources == item.source_signal_keys.as_slice()
                })
            && reading_items_by_slot.get(slot).is_some_and(|reading_item| {
                reading_item.primary_signal_keys == item.primary_signal_keys
                    && reading_item.secondary_slot_candidates == item.secondary_slot_candidates
            })
            && reading_items_by_slot.get(slot).is_some_and(|reading_item| {
                item.emphasis_refs
                    == expected_emphasis_refs_for_slot(reading_item, payload, has_dominant_cluster)
            })
            && item.context_refs == expected_context_refs_for_slot(slot)
            && !item.section_title.trim().is_empty()
            && !item.writing_objective.trim().is_empty()
            && text::has_current_drafting_language(item)
            && item.max_words > 0
            && !item.avoid.is_empty()
            && item.avoid.iter().all(|rule| !rule.trim().is_empty())
            && item
                .avoid
                .contains(&"turn chart_emphasis into a standalone section".to_string())
            && item
                .avoid
                .contains(&"turn chart_context into a standalone section".to_string())
            && item
                .avoid
                .contains(&"turn rulership_context into a standalone section".to_string())
            && !item.source_signal_keys.is_empty()
            && item.source_signal_keys.iter().all(|signal_key| {
                let signal_key = signal_key.trim();
                !signal_key.is_empty() && signal_keys.contains(signal_key)
            })
    })
}

fn expected_context_refs_for_slot(slot: &str) -> crate::domain::BasicContextRefs {
    let chart_context = match slot {
        "core_identity" | "dominant_cluster" => {
            vec!["sect".to_string(), "hemisphere_emphasis".to_string()]
        }
        _ => Vec::new(),
    };
    let rulership_context = match slot {
        "core_identity" => vec!["ascendant_ruler".to_string()],
        "dominant_cluster" => {
            vec![
                "dominant_sign_rulers".to_string(),
                "dominant_house_rulers".to_string(),
            ]
        }
        _ => Vec::new(),
    };

    crate::domain::BasicContextRefs {
        chart_context,
        rulership_context,
    }
}

fn secondary_candidates_are_valid(
    item: &BasicReadingPlanItem,
    signal_keys: &HashSet<&str>,
    primary_signal_slots: &HashMap<&str, &str>,
) -> bool {
    item.secondary_slot_candidates.iter().all(|candidate| {
        secondary_candidate_is_valid(candidate, item, signal_keys, primary_signal_slots)
    })
}

fn secondary_candidate_is_valid(
    candidate: &BasicSecondarySlotCandidate,
    item: &BasicReadingPlanItem,
    signal_keys: &HashSet<&str>,
    primary_signal_slots: &HashMap<&str, &str>,
) -> bool {
    !candidate.signal_key.trim().is_empty()
        && signal_keys.contains(candidate.signal_key.as_str())
        && primary_signal_slots
            .get(candidate.signal_key.as_str())
            .is_some_and(|primary_slot| *primary_slot == candidate.primary_slot)
        && candidate.candidate_slot == item.slot
        && !item.source_signal_keys.contains(&candidate.signal_key)
}

fn reading_slot_order(slot: &str) -> Option<usize> {
    match slot {
        "core_identity" => Some(0),
        "dominant_cluster" => Some(1),
        "main_tension_or_support" => Some(2),
        "expression_style" => Some(3),
        "background_factors" => Some(4),
        _ => None,
    }
}

fn expected_emphasis_refs_for_slot(
    item: &BasicReadingPlanItem,
    payload: &BasicPayload,
    has_dominant_cluster: bool,
) -> BasicEmphasisRefs {
    let should_attach =
        item.slot == "dominant_cluster" || (item.slot == "core_identity" && !has_dominant_cluster);
    if !should_attach {
        return BasicEmphasisRefs::default();
    }

    let (dominant_signs, dominant_houses) = if item.slot == "dominant_cluster" {
        let cluster_signs = cluster_sign_refs(item, payload);
        let cluster_houses = cluster_house_refs(item, payload);
        (
            filtered_or_all_sign_refs(payload, &cluster_signs),
            filtered_or_all_house_refs(payload, &cluster_houses),
        )
    } else {
        (
            payload
                .chart_emphasis
                .dominant_signs
                .iter()
                .map(|entry| entry.sign_code.clone())
                .collect(),
            payload
                .chart_emphasis
                .dominant_houses
                .iter()
                .map(|entry| entry.house_number)
                .collect(),
        )
    };

    let slot_objects = emphasis_object_scope(item);
    let dominant_objects = if slot_objects.is_empty() {
        payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .map(|entry| entry.object_code.clone())
            .collect()
    } else {
        payload
            .chart_emphasis
            .dominant_objects
            .iter()
            .filter(|entry| slot_objects.contains(&entry.object_code))
            .map(|entry| entry.object_code.clone())
            .collect()
    };

    BasicEmphasisRefs {
        dominant_signs,
        dominant_houses,
        dominant_objects,
    }
}

fn cluster_sign_refs(item: &BasicReadingPlanItem, payload: &BasicPayload) -> Vec<String> {
    signals_for_plan_item(item, payload)
        .into_iter()
        .filter(|signal| signal.signal_key.starts_with("cluster:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("sign_code"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn cluster_house_refs(item: &BasicReadingPlanItem, payload: &BasicPayload) -> Vec<i32> {
    signals_for_plan_item(item, payload)
        .into_iter()
        .filter(|signal| signal.signal_key.starts_with("cluster:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("house_number"))
                .and_then(|value| value.as_i64())
                .and_then(|value| i32::try_from(value).ok())
        })
        .collect()
}

fn signals_for_plan_item<'a>(
    item: &BasicReadingPlanItem,
    payload: &'a BasicPayload,
) -> Vec<&'a BasicSignal> {
    item.source_signal_keys
        .iter()
        .filter_map(|key| {
            payload
                .signals
                .iter()
                .find(|signal| signal.signal_key == *key)
        })
        .collect()
}

fn filtered_or_all_sign_refs(payload: &BasicPayload, allowed_signs: &[String]) -> Vec<String> {
    let refs = payload
        .chart_emphasis
        .dominant_signs
        .iter()
        .filter(|entry| allowed_signs.contains(&entry.sign_code))
        .map(|entry| entry.sign_code.clone())
        .collect::<Vec<_>>();
    if refs.is_empty() {
        payload
            .chart_emphasis
            .dominant_signs
            .iter()
            .map(|entry| entry.sign_code.clone())
            .collect()
    } else {
        refs
    }
}

fn filtered_or_all_house_refs(payload: &BasicPayload, allowed_houses: &[i32]) -> Vec<i32> {
    let refs = payload
        .chart_emphasis
        .dominant_houses
        .iter()
        .filter(|entry| allowed_houses.contains(&entry.house_number))
        .map(|entry| entry.house_number)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        payload
            .chart_emphasis
            .dominant_houses
            .iter()
            .map(|entry| entry.house_number)
            .collect()
    } else {
        refs
    }
}

fn emphasis_object_scope(item: &BasicReadingPlanItem) -> Vec<String> {
    let mut object_codes = Vec::new();
    for signal_key in item.source_signal_keys.iter().chain(
        item.secondary_slot_candidates
            .iter()
            .map(|candidate| &candidate.signal_key),
    ) {
        if let Some(object_code) = object_code_from_signal_key(signal_key) {
            if !object_codes.contains(&object_code) {
                object_codes.push(object_code);
            }
        }
    }
    object_codes
}

fn object_code_from_signal_key(signal_key: &str) -> Option<String> {
    if let Some(object_code) = signal_key.strip_prefix("object_position:") {
        return Some(object_code.to_string());
    }
    signal_key
        .strip_prefix("dignity:")
        .and_then(|tail| tail.split(':').next())
        .filter(|object_code| !object_code.is_empty())
        .map(ToString::to_string)
}

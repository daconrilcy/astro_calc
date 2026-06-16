use std::collections::{HashMap, HashSet};

use crate::domain::{BasicPayload, BasicReadingPlanItem, BasicSecondarySlotCandidate, BasicSignal};

pub(crate) fn build_reading_plan(
    signals: &[BasicSignal],
    is_interpretive_aspect_signal: fn(&BasicSignal) -> bool,
    is_interpretive_support_aspect: fn(&BasicSignal) -> bool,
    is_interpretive_tension_aspect: fn(&BasicSignal) -> bool,
) -> Vec<BasicReadingPlanItem> {
    let mut plan = Vec::new();

    push_plan_item(
        &mut plan,
        "core_identity",
        "Core identity markers",
        signal_keys_for_objects(signals, &["sun", "moon", "ascendant"], 3),
    );

    if let Some(cluster) = signals.iter().find(|signal| signal.signal_key.starts_with("cluster:")) {
        let mut source_signal_keys = vec![cluster.signal_key.clone()];
        source_signal_keys.extend(
            cluster
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("source_signals"))
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_str())
                .filter(|source_key| signals.iter().any(|signal| signal.signal_key == *source_key))
                .map(ToString::to_string),
        );
        source_signal_keys.extend(cluster_source_dignity_keys(signals, cluster));
        dedupe_strings(&mut source_signal_keys);

        push_plan_item(
            &mut plan,
            "dominant_cluster",
            "Dominant repeated theme",
            source_signal_keys,
        );
    }

    push_plan_item(
        &mut plan,
        "main_tension_or_support",
        "Main dynamic aspect",
        main_dynamic_aspect_keys(
            signals,
            is_interpretive_aspect_signal,
            is_interpretive_support_aspect,
            is_interpretive_tension_aspect,
        ),
    );

    push_plan_item(
        &mut plan,
        "expression_style",
        "Expression style",
        signal_keys_for_objects(signals, &["mercury", "venus", "mars"], 3),
    );

    push_plan_item(
        &mut plan,
        "background_factors",
        "Background factors",
        signal_keys_for_objects(
            signals,
            &["mc", "jupiter", "saturn", "uranus", "neptune", "pluto"],
            3,
        ),
    );

    finalize_reading_plan(&mut plan);
    plan
}

pub(crate) fn has_current_reading_plan(payload: &BasicPayload) -> bool {
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

pub(crate) fn reading_slot_order(slot: &str) -> Option<usize> {
    match slot {
        "core_identity" => Some(0),
        "dominant_cluster" => Some(1),
        "main_tension_or_support" => Some(2),
        "expression_style" => Some(3),
        "background_factors" => Some(4),
        _ => None,
    }
}

fn main_dynamic_aspect_keys(
    signals: &[BasicSignal],
    is_interpretive_aspect_signal: fn(&BasicSignal) -> bool,
    is_interpretive_support_aspect: fn(&BasicSignal) -> bool,
    is_interpretive_tension_aspect: fn(&BasicSignal) -> bool,
) -> Vec<String> {
    let mut keys: Vec<String> = signals
        .iter()
        .filter(|signal| is_interpretive_aspect_signal(signal))
        .take(3)
        .map(|signal| signal.signal_key.clone())
        .collect();

    let has_tension = keys
        .iter()
        .filter_map(|key| signals.iter().find(|signal| signal.signal_key == *key))
        .any(is_interpretive_tension_aspect);
    if !has_tension {
        if let Some(tension_key) = signals
            .iter()
            .filter(|signal| is_interpretive_aspect_signal(signal))
            .find(|signal| is_interpretive_tension_aspect(signal))
            .map(|signal| signal.signal_key.clone())
        {
            push_balanced_aspect_key(
                &mut keys,
                tension_key,
                signals,
                is_interpretive_support_aspect,
            );
        }
    }

    let has_support = keys
        .iter()
        .filter_map(|key| signals.iter().find(|signal| signal.signal_key == *key))
        .any(is_interpretive_support_aspect);
    if !has_support {
        if let Some(support_key) = signals
            .iter()
            .filter(|signal| is_interpretive_aspect_signal(signal))
            .find(|signal| is_interpretive_support_aspect(signal))
            .map(|signal| signal.signal_key.clone())
        {
            push_balanced_aspect_key(
                &mut keys,
                support_key,
                signals,
                is_interpretive_tension_aspect,
            );
        }
    }

    dedupe_strings(&mut keys);
    keys.truncate(3);
    keys
}

fn push_balanced_aspect_key(
    keys: &mut Vec<String>,
    new_key: String,
    signals: &[BasicSignal],
    preserve: fn(&BasicSignal) -> bool,
) {
    if keys.contains(&new_key) {
        return;
    }

    if keys.len() < 3 {
        keys.push(new_key);
        return;
    }

    let preserved_count = keys
        .iter()
        .filter_map(|key| signals.iter().find(|signal| signal.signal_key == **key))
        .filter(|signal| preserve(signal))
        .count();

    let replacement_index = keys
        .iter()
        .enumerate()
        .rev()
        .find(|(_, key)| {
            signals
                .iter()
                .find(|signal| signal.signal_key == **key)
                .is_none_or(|signal| !preserve(signal) || preserved_count > 1)
        })
        .map(|(index, _)| index)
        .unwrap_or(keys.len() - 1);

    keys[replacement_index] = new_key;
}

fn push_plan_item(
    plan: &mut Vec<BasicReadingPlanItem>,
    slot: &str,
    title: &str,
    source_signal_keys: Vec<String>,
) {
    if source_signal_keys.is_empty() {
        return;
    }

    plan.push(BasicReadingPlanItem {
        slot: slot.to_string(),
        title: title.to_string(),
        primary_signal_keys: source_signal_keys.clone(),
        source_signal_keys,
        secondary_slot_candidates: Vec::new(),
    });
}

fn finalize_reading_plan(plan: &mut Vec<BasicReadingPlanItem>) {
    let mut primary_slots: Vec<(String, String)> = Vec::new();

    for item in plan.iter_mut() {
        let mut primary_signal_keys = Vec::new();
        let mut secondary_slot_candidates = Vec::new();

        for signal_key in item.source_signal_keys.drain(..) {
            if let Some((_, primary_slot)) =
                primary_slots.iter().find(|(assigned_key, _)| assigned_key == &signal_key)
            {
                secondary_slot_candidates.push(BasicSecondarySlotCandidate {
                    signal_key,
                    primary_slot: primary_slot.clone(),
                    candidate_slot: item.slot.clone(),
                });
            } else {
                primary_slots.push((signal_key.clone(), item.slot.clone()));
                primary_signal_keys.push(signal_key);
            }
        }

        item.source_signal_keys = primary_signal_keys.clone();
        item.primary_signal_keys = primary_signal_keys;
        item.secondary_slot_candidates = secondary_slot_candidates;
    }

    plan.retain(|item| !item.source_signal_keys.is_empty());
}

fn signal_keys_for_objects(
    signals: &[BasicSignal],
    object_codes: &[&str],
    limit: usize,
) -> Vec<String> {
    let mut keys = Vec::new();
    let mut selected_objects = 0;

    for object_code in object_codes {
        if let Some(signal_key) = position_signal_key_for_object(signals, object_code) {
            keys.push(signal_key);
            selected_objects += 1;
        }
        keys.extend(dignity_signal_keys_for_object(signals, object_code));
        if selected_objects >= limit {
            break;
        }
    }

    dedupe_strings(&mut keys);
    keys
}

fn position_signal_key_for_object(signals: &[BasicSignal], object_code: &str) -> Option<String> {
    let object_position_key = format!("object_position:{object_code}");
    if signals
        .iter()
        .any(|signal| signal.signal_key == object_position_key)
    {
        return Some(object_position_key);
    }

    let angle_prefix = format!("angle:{object_code}:sign:");
    signals
        .iter()
        .find(|signal| signal.signal_key.starts_with(&angle_prefix))
        .map(|signal| signal.signal_key.clone())
}

fn cluster_source_dignity_keys(signals: &[BasicSignal], cluster: &BasicSignal) -> Vec<String> {
    cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("source_objects"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .flat_map(|object_code| dignity_signal_keys_for_object(signals, object_code))
        .collect()
}

fn dignity_signal_keys_for_object(signals: &[BasicSignal], object_code: &str) -> Vec<String> {
    let prefix = format!("dignity:{object_code}:");
    signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with(&prefix))
        .map(|signal| signal.signal_key.clone())
        .collect()
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for value in values.drain(..) {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    *values = deduped;
}

fn secondary_candidates_are_valid(
    item: &BasicReadingPlanItem,
    signal_keys: &HashSet<&str>,
    primary_signal_slots: &HashMap<&str, &str>,
) -> bool {
    item.secondary_slot_candidates.iter().all(|candidate| {
        !candidate.signal_key.trim().is_empty()
            && signal_keys.contains(candidate.signal_key.as_str())
            && primary_signal_slots
                .get(candidate.signal_key.as_str())
                .is_some_and(|primary_slot| *primary_slot == candidate.primary_slot)
            && candidate.candidate_slot == item.slot
            && !item.source_signal_keys.contains(&candidate.signal_key)
    })
}

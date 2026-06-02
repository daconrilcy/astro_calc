use crate::domain::{
    BasicChartEmphasis, BasicDraftingPlanItem, BasicEmphasisRefs, BasicReadingPlanItem, BasicSignal,
};
pub(super) fn build_drafting_plan(
    reading_plan: &[BasicReadingPlanItem],
    signals: &[BasicSignal],
    chart_emphasis: &BasicChartEmphasis,
) -> Vec<BasicDraftingPlanItem> {
    let has_dominant_cluster = reading_plan
        .iter()
        .any(|item| item.slot == "dominant_cluster");

    reading_plan
        .iter()
        .map(|item| {
            let source_signals = signals_for_keys(signals, &item.source_signal_keys);
            BasicDraftingPlanItem {
                slot: item.slot.clone(),
                section_title: section_title(item, &source_signals),
                source_signal_keys: item.source_signal_keys.clone(),
                primary_signal_keys: item.primary_signal_keys.clone(),
                secondary_slot_candidates: item.secondary_slot_candidates.clone(),
                emphasis_refs: emphasis_refs_for_slot(
                    item,
                    &source_signals,
                    chart_emphasis,
                    has_dominant_cluster,
                ),
                writing_objective: writing_objective(item, &source_signals, has_dominant_cluster),
                max_words: max_words_for_slot(&item.slot),
                avoid: avoid_rules_for_slot(&item.slot),
            }
        })
        .collect()
}

fn emphasis_refs_for_slot(
    item: &BasicReadingPlanItem,
    signals: &[&BasicSignal],
    chart_emphasis: &BasicChartEmphasis,
    has_dominant_cluster: bool,
) -> BasicEmphasisRefs {
    let should_attach =
        item.slot == "dominant_cluster" || (item.slot == "core_identity" && !has_dominant_cluster);
    if !should_attach {
        return BasicEmphasisRefs::default();
    }

    let (dominant_signs, dominant_houses) = if item.slot == "dominant_cluster" {
        let cluster_signs = cluster_sign_refs(signals);
        let cluster_houses = cluster_house_refs(signals);
        (
            filtered_or_all_sign_refs(chart_emphasis, &cluster_signs),
            filtered_or_all_house_refs(chart_emphasis, &cluster_houses),
        )
    } else {
        (
            chart_emphasis
                .dominant_signs
                .iter()
                .map(|entry| entry.sign_code.clone())
                .collect(),
            chart_emphasis
                .dominant_houses
                .iter()
                .map(|entry| entry.house_number)
                .collect(),
        )
    };

    let slot_objects = emphasis_object_scope(item);
    let dominant_objects = if slot_objects.is_empty() {
        chart_emphasis
            .dominant_objects
            .iter()
            .map(|entry| entry.object_code.clone())
            .collect()
    } else {
        chart_emphasis
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

fn cluster_sign_refs(signals: &[&BasicSignal]) -> Vec<String> {
    signals
        .iter()
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

fn cluster_house_refs(signals: &[&BasicSignal]) -> Vec<i32> {
    signals
        .iter()
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

fn filtered_or_all_sign_refs(
    chart_emphasis: &BasicChartEmphasis,
    allowed_signs: &[String],
) -> Vec<String> {
    let refs = chart_emphasis
        .dominant_signs
        .iter()
        .filter(|entry| allowed_signs.contains(&entry.sign_code))
        .map(|entry| entry.sign_code.clone())
        .collect::<Vec<_>>();
    if refs.is_empty() {
        chart_emphasis
            .dominant_signs
            .iter()
            .map(|entry| entry.sign_code.clone())
            .collect()
    } else {
        refs
    }
}

fn filtered_or_all_house_refs(
    chart_emphasis: &BasicChartEmphasis,
    allowed_houses: &[i32],
) -> Vec<i32> {
    let refs = chart_emphasis
        .dominant_houses
        .iter()
        .filter(|entry| allowed_houses.contains(&entry.house_number))
        .map(|entry| entry.house_number)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        chart_emphasis
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
    if let Some(tail) = signal_key.strip_prefix("angle:") {
        return tail
            .split(':')
            .next()
            .filter(|object_code| !object_code.is_empty())
            .map(ToString::to_string);
    }
    signal_key
        .strip_prefix("dignity:")
        .and_then(|tail| tail.split(':').next())
        .filter(|object_code| !object_code.is_empty())
        .map(ToString::to_string)
}

fn signals_for_keys<'a>(signals: &'a [BasicSignal], keys: &[String]) -> Vec<&'a BasicSignal> {
    keys.iter()
        .filter_map(|key| signals.iter().find(|signal| signal.signal_key == *key))
        .collect()
}

fn section_title(item: &BasicReadingPlanItem, signals: &[&BasicSignal]) -> String {
    match item.slot.as_str() {
        "core_identity" => "Core chart markers".to_string(),
        "dominant_cluster" => cluster_section_title(signals),
        "main_tension_or_support" => "Main dynamics".to_string(),
        "expression_style" => "Expression and action style".to_string(),
        "background_factors" => "Background factors".to_string(),
        _ => item.title.clone(),
    }
}

fn cluster_section_title(signals: &[&BasicSignal]) -> String {
    let Some(cluster) = signals
        .iter()
        .copied()
        .find(|signal| signal.signal_key.starts_with("cluster:"))
    else {
        return "A structuring dominant theme".to_string();
    };

    let sign_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("sign_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the sign");
    let house_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("house_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the house");

    format!("A {sign_name} dominant theme around {house_name}")
}

fn writing_objective(
    item: &BasicReadingPlanItem,
    signals: &[&BasicSignal],
    has_dominant_cluster: bool,
) -> String {
    match item.slot.as_str() {
        "core_identity" if has_dominant_cluster => {
            "Explain the central identity markers, emotional needs, and overall chart orientation in plain language, letting chart_emphasis only adjust relative weight without creating another section.".to_string()
        }
        "core_identity" => {
            "Explain the central identity markers, emotional needs, and overall chart orientation in plain language, using emphasis_refs as weighting context rather than as a separate topic.".to_string()
        }
        "dominant_cluster" => dominant_cluster_objective(signals),
        "main_tension_or_support" => {
            "Explain the main relationships between chart factors, distinguishing supportive and challenging dynamics without turning aspects into verdicts.".to_string()
        }
        "expression_style" => {
            "Show how the person thinks, communicates, desires, chooses, and acts day to day without listing each placement separately.".to_string()
        }
        "background_factors" => {
            "Place the more collective or less central factors in the background with brief, proportionate wording.".to_string()
        }
        _ => format!(
            "Draft a short section from the {} slot while staying strictly grounded in the source signals.",
            item.slot
        ),
    }
}

fn dominant_cluster_objective(signals: &[&BasicSignal]) -> String {
    let Some(cluster) = signals
        .iter()
        .copied()
        .find(|signal| signal.signal_key.starts_with("cluster:"))
    else {
        return "Explain the chart's dominant theme in plain language without repeating each placement one by one.".to_string();
    };

    let sign_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("sign_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the sign");
    let house_name = cluster
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("house_name"))
        .and_then(|value| value.as_str())
        .unwrap_or("the house");
    let sign_code = sign_name.to_lowercase();
    let house_code = house_name.to_lowercase();
    let themes = cluster
        .semantic_tags
        .iter()
        .filter(|tag| {
            !matches!(
                tag.as_str(),
                "cluster" | "placement" | "aspect" | "high_strength" | "medium_strength"
            ) && !tag.starts_with("house_")
                && tag.as_str() != sign_code
                && tag.as_str() != house_code
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>();

    let theme_text = if themes.is_empty() {
        "the cluster's recurring themes".to_string()
    } else {
        themes.join(", ")
    };

    let grouping_context = cluster_grouping_context(signals);

    format!(
        "Explain in plain language that the chart emphasizes {sign_name}, {house_name}, and {theme_text}, grouping the {grouping_context} instead of enumerating placements. Use emphasis_refs only as weighting context, not as a separate section."
    )
}

fn cluster_grouping_context(signals: &[&BasicSignal]) -> String {
    let dignity_objects = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("dignity:"))
        .filter_map(|signal| {
            signal
                .evidence
                .as_ref()
                .and_then(|evidence| evidence.get("chart_object"))
                .and_then(|value| value.as_str())
                .map(capitalize_ascii)
        })
        .collect::<Vec<_>>();

    match dignity_objects.as_slice() {
        [] => "cluster evidence".to_string(),
        [object] => format!("cluster evidence and {object} dignity context"),
        _ => "cluster evidence and dignity contexts".to_string(),
    }
}

fn capitalize_ascii(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.as_str().to_ascii_lowercase()
    )
}

fn max_words_for_slot(slot: &str) -> u16 {
    match slot {
        "dominant_cluster" => 120,
        "core_identity" | "main_tension_or_support" | "expression_style" => 110,
        "background_factors" => 80,
        _ => 100,
    }
}

fn avoid_rules_for_slot(slot: &str) -> Vec<String> {
    let mut rules = vec![
        "use technical IDs".to_string(),
        "make fatalistic predictions".to_string(),
        "add information that is absent from the source signals".to_string(),
        "turn chart_emphasis into a standalone section".to_string(),
    ];

    match slot {
        "dominant_cluster" => {
            rules.insert(0, "repeat each placement one by one".to_string());
        }
        "main_tension_or_support" => {
            rules.insert(0, "present an aspect as an isolated verdict".to_string());
        }
        "background_factors" => {
            rules.insert(0, "give too much weight to background factors".to_string());
        }
        _ => {}
    }

    rules
}

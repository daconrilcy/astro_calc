use crate::domain::{
    BasicDraftingPlanItem, BasicGeneratedReadingPayload, BasicGeneratedSection,
    BasicObjectPosition, BasicPayload, BasicReadingPlanItem, BasicSignal, BasicWritingContract,
    NatalChartInput, ObjectPositionFact,
};
use crate::models::InterpretationSignalRow;

pub fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    let basic_signals: Vec<BasicSignal> = signals
        .iter()
        .take(12)
        .map(|signal| BasicSignal {
            signal_key: signal.signal_key.clone(),
            theme_code: signal.theme_code.clone(),
            title: signal.title.clone(),
            summary: signal.summary.clone(),
            priority_score: signal.priority_score,
            confidence_score: signal.confidence_score,
            interpretive_hint: payload_string(signal, "interpretive_hint"),
            semantic_tags: payload_string_array(signal, "semantic_tags"),
            source_weight: payload_f64(signal, "source_weight"),
            aggregation_group: payload_string(signal, "aggregation_group"),
            writing_guidance: payload_string(signal, "writing_guidance"),
            evidence: payload_value(signal, "evidence"),
        })
        .collect();

    let reading_plan = build_reading_plan(&basic_signals);
    let drafting_plan = build_drafting_plan(&reading_plan, &basic_signals);

    BasicPayload {
        product_code: input.product_code().to_string(),
        chart_calculation_id,
        reference_version_id: input.reference_version_id,
        subject_label: input.subject_label.clone(),
        birth_datetime_utc: input.birth_datetime_utc,
        writing_contract: Some(basic_writing_contract()),
        positions: positions
            .iter()
            .map(|position| BasicObjectPosition {
                object_code: position.object_code.clone(),
                object_name: position.object_name.clone(),
                longitude_deg: position.longitude_deg,
                sign_id: position.sign_id,
                sign_code: position.sign_code.clone(),
                sign_name: position.sign_name.clone(),
                house_id: position.house_id,
                house_number: position.house_number,
                house_name: position.house_name.clone(),
                motion_state_id: position.motion_state_id,
            })
            .collect(),
        signals: basic_signals,
        reading_plan,
        drafting_plan,
    }
}

fn build_reading_plan(signals: &[BasicSignal]) -> Vec<BasicReadingPlanItem> {
    let mut plan = Vec::new();

    push_plan_item(
        &mut plan,
        "core_identity",
        "Core identity markers",
        signal_keys_for_objects(signals, &["sun", "moon", "ascendant", "mc"], 4),
    );

    if let Some(cluster) = signals
        .iter()
        .find(|signal| signal.signal_key.starts_with("cluster:"))
    {
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
                .filter(|source_key| {
                    signals
                        .iter()
                        .any(|signal| signal.signal_key == *source_key)
                })
                .map(ToString::to_string),
        );
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
        main_dynamic_aspect_keys(signals),
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
            &["jupiter", "saturn", "uranus", "neptune", "pluto"],
            3,
        ),
    );

    plan
}

pub fn build_fake_generated_reading(payload: &BasicPayload) -> BasicGeneratedReadingPayload {
    let writing_contract = payload
        .writing_contract
        .clone()
        .unwrap_or_else(basic_writing_contract);
    let generated_sections = payload
        .drafting_plan
        .iter()
        .map(|item| {
            let text = fake_section_text(
                item,
                &signals_for_keys(&payload.signals, &item.source_signal_keys),
            );
            let text = sanitize_generated_text(&truncate_words(&text, item.max_words));
            BasicGeneratedSection {
                slot: item.slot.clone(),
                section_title: item.section_title.clone(),
                source_signal_keys: item.source_signal_keys.clone(),
                word_count: word_count(&text),
                text,
            }
        })
        .collect();

    BasicGeneratedReadingPayload {
        product_code: generated_product_code(&payload.product_code),
        source_product_code: payload.product_code.clone(),
        chart_calculation_id: payload.chart_calculation_id,
        reference_version_id: payload.reference_version_id,
        subject_label: payload.subject_label.clone(),
        birth_datetime_utc: payload.birth_datetime_utc,
        generation_provider: "fake_deterministic_v1".to_string(),
        writing_contract,
        generated_sections,
    }
}

pub fn is_valid_fake_generated_reading(
    source_payload: &BasicPayload,
    generated_payload: &BasicGeneratedReadingPayload,
) -> bool {
    let Some(writing_contract) = source_payload.writing_contract.as_ref() else {
        return false;
    };
    if generated_payload.source_product_code != source_payload.product_code
        || generated_payload.chart_calculation_id != source_payload.chart_calculation_id
        || generated_payload.reference_version_id != source_payload.reference_version_id
        || generated_payload.generation_provider != "fake_deterministic_v1"
        || generated_payload.writing_contract.audience_level != writing_contract.audience_level
        || generated_payload.writing_contract.tone != writing_contract.tone
        || generated_payload.writing_contract.language != writing_contract.language
        || generated_payload.writing_contract.max_total_words != writing_contract.max_total_words
        || generated_payload.writing_contract.must_not != writing_contract.must_not
        || generated_payload.generated_sections.len() != source_payload.drafting_plan.len()
    {
        return false;
    }

    let signal_keys = source_payload
        .signals
        .iter()
        .map(|signal| signal.signal_key.as_str())
        .collect::<Vec<_>>();
    let total_words: u32 = generated_payload
        .generated_sections
        .iter()
        .map(|section| u32::from(section.word_count))
        .sum();

    total_words <= u32::from(writing_contract.max_total_words)
        && generated_payload
            .generated_sections
            .iter()
            .zip(&source_payload.drafting_plan)
            .all(|(section, plan_item)| {
                section.slot == plan_item.slot
                    && section.section_title == plan_item.section_title
                    && section.source_signal_keys == plan_item.source_signal_keys
                    && section.word_count == word_count(&section.text)
                    && section.word_count <= plan_item.max_words
                    && !section.text.trim().is_empty()
                    && !contains_technical_id_token(&section.text)
                    && section
                        .source_signal_keys
                        .iter()
                        .all(|key| signal_keys.contains(&key.as_str()))
            })
}

pub fn generated_product_code(source_product_code: &str) -> String {
    format!("{source_product_code}_generated_fake")
}

pub fn basic_writing_contract() -> BasicWritingContract {
    BasicWritingContract {
        audience_level: "beginner".to_string(),
        tone: "clear, warm, non fatalistic".to_string(),
        language: "fr".to_string(),
        max_total_words: 650,
        must_not: vec![
            "list placements mechanically".to_string(),
            "mention internal IDs".to_string(),
            "invent facts not present in source signals".to_string(),
            "use deterministic or fatalistic wording".to_string(),
        ],
    }
}

fn main_dynamic_aspect_keys(signals: &[BasicSignal]) -> Vec<String> {
    let mut keys: Vec<String> = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("aspect:"))
        .take(3)
        .map(|signal| signal.signal_key.clone())
        .collect();

    if keys.iter().any(|key| {
        signals
            .iter()
            .find(|signal| signal.signal_key == *key)
            .is_some_and(is_strong_tension_aspect)
    }) {
        return keys;
    }

    if let Some(tension_key) = signals
        .iter()
        .filter(|signal| signal.signal_key.starts_with("aspect:"))
        .find(|signal| is_strong_tension_aspect(signal))
        .map(|signal| signal.signal_key.clone())
    {
        if keys.len() >= 3 {
            keys.pop();
        }
        keys.push(tension_key);
        dedupe_strings(&mut keys);
    }

    keys
}

fn is_strong_tension_aspect(signal: &BasicSignal) -> bool {
    let aspect_code = signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("aspect_code"))
        .and_then(|value| value.as_str());
    let strength_score = signal
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.get("strength_score"))
        .and_then(|value| value.as_f64())
        .unwrap_or(signal.priority_score / 80.0);

    matches!(aspect_code, Some("square" | "opposition")) && strength_score >= 0.75
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
        source_signal_keys,
    });
}

fn signal_keys_for_objects(
    signals: &[BasicSignal],
    object_codes: &[&str],
    limit: usize,
) -> Vec<String> {
    let mut keys = Vec::new();

    for object_code in object_codes {
        let signal_key = format!("object_position:{object_code}");
        if signals.iter().any(|signal| signal.signal_key == signal_key) {
            keys.push(signal_key);
        }
        if keys.len() >= limit {
            break;
        }
    }

    keys
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

fn build_drafting_plan(
    reading_plan: &[BasicReadingPlanItem],
    signals: &[BasicSignal],
) -> Vec<BasicDraftingPlanItem> {
    reading_plan
        .iter()
        .map(|item| {
            let source_signals = signals_for_keys(signals, &item.source_signal_keys);
            BasicDraftingPlanItem {
                slot: item.slot.clone(),
                section_title: section_title(item, &source_signals),
                source_signal_keys: item.source_signal_keys.clone(),
                writing_objective: writing_objective(item, &source_signals),
                max_words: max_words_for_slot(&item.slot),
                avoid: avoid_rules_for_slot(&item.slot),
            }
        })
        .collect()
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

fn writing_objective(item: &BasicReadingPlanItem, signals: &[&BasicSignal]) -> String {
    match item.slot.as_str() {
        "core_identity" => {
            "Explain the central identity markers, emotional needs, and overall chart orientation in plain language.".to_string()
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

    format!(
        "Explain in plain language that the chart emphasizes {sign_name}, {house_name}, and {theme_text}, grouping the related placements instead of enumerating them."
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

fn fake_section_text(item: &BasicDraftingPlanItem, signals: &[&BasicSignal]) -> String {
    let titles = signals
        .iter()
        .map(|signal| signal.title.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let summaries = signals
        .iter()
        .filter_map(|signal| signal.summary.as_deref())
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    let source_text = if titles.is_empty() {
        "les signaux retenus".to_string()
    } else {
        titles
    };
    let summary_text = if summaries.is_empty() {
        "Le texte reste volontairement limite aux signaux disponibles.".to_string()
    } else {
        summaries
    };

    match item.slot.as_str() {
        "core_identity" => format!(
            "Cette section presente les reperes centraux du theme a partir de {source_text}. {summary_text} La lecture reste simple et relie ces facteurs sans les transformer en certitudes."
        ),
        "dominant_cluster" => format!(
            "Cette section regroupe la dominante principale au lieu de lister chaque placement. Les sources retenues sont {source_text}. {summary_text} Le propos souligne une tendance de fond avec une formulation prudente."
        ),
        "main_tension_or_support" => format!(
            "Cette section explique les relations les plus visibles entre facteurs du theme. Elle s'appuie sur {source_text}. {summary_text} Les appuis et les tensions sont presentes comme des dynamiques, pas comme des verdicts."
        ),
        "expression_style" => format!(
            "Cette section decrit la maniere de penser, choisir, aimer et agir au quotidien. Elle s'appuie sur {source_text}. {summary_text} Le texte cherche une synthese lisible plutot qu'une liste mecanique."
        ),
        "background_factors" => format!(
            "Cette section place les facteurs de fond a leur juste niveau. Les sources retenues sont {source_text}. {summary_text} Elle donne du contexte sans alourdir la lecture principale."
        ),
        _ => format!(
            "Cette section suit l'objectif editorial prevu pour {}. Elle s'appuie sur {source_text}. {summary_text}",
            item.section_title
        ),
    }
}

fn sanitize_generated_text(text: &str) -> String {
    technical_id_tokens()
        .iter()
        .fold(text.to_string(), |cleaned, token| {
            cleaned.replace(token, "technical reference")
        })
}

fn truncate_words(text: &str, max_words: u16) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() <= usize::from(max_words) {
        return text.to_string();
    }
    words
        .into_iter()
        .take(usize::from(max_words))
        .collect::<Vec<_>>()
        .join(" ")
}

fn word_count(text: &str) -> u16 {
    text.split_whitespace().count().min(u16::MAX as usize) as u16
}

fn contains_technical_id_token(text: &str) -> bool {
    technical_id_tokens()
        .iter()
        .any(|token| text.contains(token))
}

fn technical_id_tokens() -> &'static [&'static str] {
    &["chart_object_id", "aspect_id", "house_id", "sign_id"]
}

fn payload_value(signal: &InterpretationSignalRow, key: &str) -> Option<serde_json::Value> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key).cloned())
}

fn payload_string(signal: &InterpretationSignalRow, key: &str) -> Option<String> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn payload_f64(signal: &InterpretationSignalRow, key: &str) -> Option<f64> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_f64())
}

fn payload_string_array(signal: &InterpretationSignalRow, key: &str) -> Vec<String> {
    signal
        .payload_json
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use super::*;

    fn input() -> NatalChartInput {
        NatalChartInput {
            subject_label: None,
            birth_datetime_utc: Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap(),
            latitude_deg: 48.8566,
            longitude_deg: 2.3522,
            altitude_m: None,
            reference_version_id: 1,
            calculation_profile_id: None,
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            house_system_id: 1,
            product_code: Some("basic".to_string()),
            language_id: None,
        }
    }

    fn position() -> ObjectPositionFact {
        ObjectPositionFact {
            chart_object_id: 1,
            object_code: "sun".to_string(),
            object_name: "Sun".to_string(),
            zodiacal_reference_system_id: 1,
            coordinate_reference_system_id: 1,
            sign_id: 3,
            sign_code: "gemini".to_string(),
            sign_name: "Gemini".to_string(),
            house_id: Some(9),
            house_number: Some(9),
            house_name: Some("Beliefs".to_string()),
            motion_state_id: Some(1),
            horizon_position_id: None,
            longitude_deg: 84.0,
            latitude_deg: None,
            apparent_speed_deg_per_day: Some(1.0),
            altitude_deg: None,
            is_visible: None,
            facts_json: None,
        }
    }

    #[test]
    fn basic_payload_exposes_semantic_signal_fields() {
        let signal = InterpretationSignalRow {
            id: 1,
            signal_key: "object_position:sun".to_string(),
            theme_code: Some("beliefs".to_string()),
            title: "Sun in Gemini, house 9".to_string(),
            summary: Some("summary".to_string()),
            priority_score: 100.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", "gemini", "beliefs"],
                "source_weight": 1.0,
                "aggregation_group": "gemini:house_9",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "object_position"}
            })),
        };

        let payload = build_basic_payload(42, &input(), &[position()], &[signal]);
        let basic_signal = &payload.signals[0];

        assert_eq!(basic_signal.theme_code.as_deref(), Some("beliefs"));
        assert_eq!(basic_signal.interpretive_hint.as_deref(), Some("hint"));
        assert_eq!(
            basic_signal.semantic_tags,
            vec!["placement", "gemini", "beliefs"]
        );
        assert_eq!(basic_signal.source_weight, Some(1.0));
        assert_eq!(
            basic_signal.aggregation_group.as_deref(),
            Some("gemini:house_9")
        );
        assert_eq!(basic_signal.writing_guidance.as_deref(), Some("guidance"));
        assert_eq!(
            basic_signal
                .evidence
                .as_ref()
                .and_then(|value| value.get("fact_type"))
                .and_then(|value| value.as_str()),
            Some("object_position")
        );
        assert_eq!(payload.reading_plan.len(), 1);
        assert_eq!(payload.reading_plan[0].slot, "core_identity");
        assert_eq!(
            payload.reading_plan[0].source_signal_keys,
            vec!["object_position:sun"]
        );
        assert_eq!(payload.drafting_plan.len(), 1);
        assert_eq!(payload.drafting_plan[0].slot, "core_identity");
        assert_eq!(
            payload.drafting_plan[0].source_signal_keys,
            payload.reading_plan[0].source_signal_keys
        );
        assert_eq!(payload.drafting_plan[0].max_words, 110);
        let contract = payload.writing_contract.as_ref().expect("writing contract");
        assert_eq!(contract.audience_level, "beginner");
        assert_eq!(contract.language, "fr");
        assert_eq!(contract.max_total_words, 650);
    }

    #[test]
    fn basic_payload_builds_reading_plan_with_cluster_sources() {
        let signals = vec![
            InterpretationSignalRow {
                id: 1,
                signal_key: "cluster:capricorn:house_2".to_string(),
                theme_code: Some("resources".to_string()),
                title: "Strong concentration in Capricorn, house 2".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 99.0,
                confidence_score: Some(0.9),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["cluster", "capricorn", "house_2", "resources", "structure", "responsibility"],
                    "source_weight": 2.0,
                    "aggregation_group": "capricorn_house_2_cluster",
                    "writing_guidance": "guidance",
                    "evidence": {
                        "fact_type": "position_cluster",
                        "sign_name": "Capricorn",
                        "house_name": "Resources",
                        "source_signals": [
                            "object_position:sun",
                            "object_position:saturn"
                        ]
                    }
                })),
            },
            InterpretationSignalRow {
                id: 2,
                signal_key: "object_position:sun".to_string(),
                theme_code: Some("resources".to_string()),
                title: "Sun in Capricorn, house 2".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 100.0,
                confidence_score: Some(0.95),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["placement", "sun"],
                    "source_weight": 1.0,
                    "aggregation_group": "capricorn:house_2",
                    "writing_guidance": "guidance",
                    "evidence": {"fact_type": "object_position", "object_code": "sun"}
                })),
            },
            InterpretationSignalRow {
                id: 3,
                signal_key: "aspect:sun:neptune:conjunction".to_string(),
                theme_code: Some("aspect".to_string()),
                title: "Sun conjunction Neptune".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 78.0,
                confidence_score: Some(0.85),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["aspect", "conjunction"],
                    "source_weight": 1.35,
                    "aggregation_group": "aspect:conjunction",
                    "writing_guidance": "guidance",
                    "evidence": {"fact_type": "aspect"}
                })),
            },
        ];

        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let cluster_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected dominant cluster plan item");

        assert_eq!(
            cluster_plan.source_signal_keys,
            vec!["cluster:capricorn:house_2", "object_position:sun"]
        );
        assert!(payload
            .reading_plan
            .iter()
            .any(|item| item.slot == "main_tension_or_support"));

        let cluster_drafting = payload
            .drafting_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .expect("expected dominant cluster drafting item");
        assert_eq!(
            cluster_drafting.source_signal_keys,
            vec!["cluster:capricorn:house_2", "object_position:sun"]
        );
        assert_eq!(
            cluster_drafting.section_title,
            "A Capricorn dominant theme around Resources"
        );
        assert!(cluster_drafting
            .avoid
            .contains(&"repeat each placement one by one".to_string()));
    }

    #[test]
    fn main_dynamic_aspects_include_strong_tension_when_available() {
        let signals = vec![
            aspect_signal(1, "aspect:moon:neptune:sextile", "sextile", 0.95),
            aspect_signal(2, "aspect:sun:moon:sextile", "sextile", 0.93),
            aspect_signal(3, "aspect:sun:neptune:conjunction", "conjunction", 0.9),
            aspect_signal(4, "aspect:moon:mars:square", "square", 0.88),
        ];

        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let aspect_plan = payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "main_tension_or_support")
            .expect("expected aspect plan");

        assert_eq!(aspect_plan.source_signal_keys.len(), 3);
        assert!(aspect_plan
            .source_signal_keys
            .contains(&"aspect:moon:mars:square".to_string()));
    }

    #[test]
    fn fake_generation_builds_bounded_sections_from_drafting_plan() {
        let signals = vec![
            InterpretationSignalRow {
                id: 1,
                signal_key: "object_position:sun".to_string(),
                theme_code: Some("beliefs".to_string()),
                title: "Sun in Gemini, house 9".to_string(),
                summary: Some("summary".to_string()),
                priority_score: 100.0,
                confidence_score: Some(0.95),
                payload_json: Some(json!({
                    "interpretive_hint": "hint",
                    "semantic_tags": ["placement", "sun"],
                    "source_weight": 1.0,
                    "aggregation_group": "gemini:house_9",
                    "writing_guidance": "guidance",
                    "evidence": {"fact_type": "object_position"}
                })),
            },
            aspect_signal(2, "aspect:moon:mars:square", "square", 0.88),
        ];
        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let generated = build_fake_generated_reading(&payload);
        let signal_keys = payload
            .signals
            .iter()
            .map(|signal| signal.signal_key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(generated.product_code, "basic_generated_fake");
        assert_eq!(generated.source_product_code, "basic");
        assert_eq!(generated.generation_provider, "fake_deterministic_v1");
        assert_eq!(
            generated.generated_sections.len(),
            payload.drafting_plan.len()
        );
        assert!(is_valid_fake_generated_reading(&payload, &generated));

        for (section, plan_item) in generated
            .generated_sections
            .iter()
            .zip(&payload.drafting_plan)
        {
            assert_eq!(section.slot, plan_item.slot);
            assert_eq!(section.section_title, plan_item.section_title);
            assert_eq!(section.source_signal_keys, plan_item.source_signal_keys);
            assert!(section.word_count <= plan_item.max_words);
            assert!(section
                .source_signal_keys
                .iter()
                .all(|key| signal_keys.contains(&key.as_str())));
            assert!(!contains_technical_id_token(&section.text));
        }
    }

    #[test]
    fn fake_generation_sanitizes_technical_tokens_from_source_text() {
        let signals = vec![InterpretationSignalRow {
            id: 1,
            signal_key: "object_position:sun".to_string(),
            theme_code: Some("beliefs".to_string()),
            title: "Sun references chart_object_id".to_string(),
            summary: Some("summary with aspect_id and house_id".to_string()),
            priority_score: 100.0,
            confidence_score: Some(0.95),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["placement", "sun"],
                "source_weight": 1.0,
                "aggregation_group": "gemini:house_9",
                "writing_guidance": "guidance",
                "evidence": {"fact_type": "object_position"}
            })),
        }];
        let payload = build_basic_payload(42, &input(), &[position()], &signals);
        let generated = build_fake_generated_reading(&payload);

        assert!(is_valid_fake_generated_reading(&payload, &generated));
        assert!(generated
            .generated_sections
            .iter()
            .all(|section| !contains_technical_id_token(&section.text)));
    }

    fn aspect_signal(
        id: i32,
        signal_key: &str,
        aspect_code: &str,
        strength_score: f64,
    ) -> InterpretationSignalRow {
        InterpretationSignalRow {
            id,
            signal_key: signal_key.to_string(),
            theme_code: Some("aspect".to_string()),
            title: format!("Aspect {aspect_code}"),
            summary: Some(format!(
                "Two chart factors form a {aspect_code} with a controlled summary."
            )),
            priority_score: strength_score * 80.0,
            confidence_score: Some(0.85),
            payload_json: Some(json!({
                "interpretive_hint": "hint",
                "semantic_tags": ["aspect", aspect_code],
                "source_weight": 1.0,
                "aggregation_group": format!("aspect:{aspect_code}"),
                "writing_guidance": "guidance",
                "evidence": {
                    "fact_type": "aspect",
                    "aspect_code": aspect_code,
                    "aspect_name": aspect_code,
                    "strength_score": strength_score
                }
            })),
        }
    }
}

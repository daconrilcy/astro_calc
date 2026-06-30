use astral_llm_domain::{
    generation_response::{CalculationReferenceMetadata, NatalReadingResponse, ReadingChapter},
    GenerateReadingRequest,
};
use serde_json::{json, Value};

pub fn enrich_reading_response(
    reading: &mut NatalReadingResponse,
    request: &GenerateReadingRequest,
) {
    fill_chapter_summaries(reading);
    reading.calculation_reference = calculation_reference_from_payload(&request.astro_result.data);
}

pub fn fill_chapter_summaries(reading: &mut NatalReadingResponse) {
    for chapter in &mut reading.chapters {
        if chapter.summary_sentence.trim().is_empty() {
            chapter.summary_sentence = first_sentence(&chapter.body)
                .filter(|sentence| !sentence.trim().is_empty())
                .unwrap_or_else(|| fallback_chapter_summary(chapter));
        } else {
            chapter.summary_sentence = first_sentence(&chapter.summary_sentence)
                .unwrap_or_else(|| chapter.summary_sentence.trim().to_string());
        }
    }
}

pub fn attach_significant_houses(data_payload: &mut Value, astro_data: &Value) {
    let houses = significant_houses_from_payload(astro_data);
    if houses.is_empty() {
        return;
    }
    if let Some(obj) = data_payload.as_object_mut() {
        obj.insert("significant_houses".into(), json!(houses));
    }
}

fn calculation_reference_from_payload(data: &Value) -> Option<CalculationReferenceMetadata> {
    let calculation = data
        .pointer("/chart/calculation")
        .or_else(|| data.pointer("/llm_payload/chart/calculation"));

    let metadata = CalculationReferenceMetadata {
        zodiacal_reference_system: string_at_any(
            calculation,
            &["zodiac", "zodiacal_reference_system"],
        ),
        coordinate_reference_system: string_at_any(
            calculation,
            &["coordinates", "coordinate_reference_system"],
        ),
        house_system: string_at_any(calculation, &["house_system"]),
        ephemeris_reference: string_at_paths(
            data,
            &[
                "/calculation_result/ephemeris_version",
                "/ephemeris_version",
                "/metadata/ephemeris_version",
            ],
        ),
        precision: string_at_paths(
            data,
            &[
                "/calculation_result/precision",
                "/calculation_result/precision_arc",
                "/precision",
                "/metadata/precision",
                "/chart/calculation/precision",
                "/llm_payload/chart/calculation/precision",
            ],
        ),
    };

    if metadata.zodiacal_reference_system.is_some()
        || metadata.coordinate_reference_system.is_some()
        || metadata.house_system.is_some()
        || metadata.ephemeris_reference.is_some()
        || metadata.precision.is_some()
    {
        Some(metadata)
    } else {
        None
    }
}

fn significant_houses_from_payload(data: &Value) -> Vec<Value> {
    data.pointer("/dominant_houses")
        .or_else(|| data.pointer("/chart_emphasis/dominant_houses"))
        .or_else(|| data.pointer("/llm_payload/dominant_houses"))
        .or_else(|| data.pointer("/llm_payload/chart_emphasis/dominant_houses"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(2)
                .filter_map(|item| {
                    let house_number = item.get("house_number").and_then(Value::as_i64)?;
                    let mut house = json!({ "house_number": house_number });
                    if let Some(obj) = house.as_object_mut() {
                        if let Some(theme_code) = item.get("theme_code").and_then(Value::as_str) {
                            obj.insert("theme_code".into(), json!(theme_code));
                        }
                        if let Some(score) = item.get("score").and_then(Value::as_f64) {
                            obj.insert("score".into(), json!(score));
                        }
                    }
                    Some(house)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn string_at_any(base: Option<&Value>, keys: &[&str]) -> Option<String> {
    let value = base?;
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn string_at_paths(data: &Value, paths: &[&str]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| data.pointer(path).and_then(Value::as_str))
        .map(str::to_string)
}

fn first_sentence(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, '.' | '!' | '?').then_some(idx + ch.len_utf8()))
        .unwrap_or(trimmed.len());
    Some(trimmed[..end].trim().to_string())
}

fn fallback_chapter_summary(chapter: &ReadingChapter) -> String {
    if chapter.title.trim().is_empty() {
        "Chapitre interpretatif synthetise.".to_string()
    } else {
        format!("{} est synthetise dans ce chapitre.", chapter.title.trim())
    }
}

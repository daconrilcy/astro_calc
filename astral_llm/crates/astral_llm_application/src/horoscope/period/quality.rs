use super::*;
pub(crate) fn period_v2_quality_issue(
    path: &str,
    code: &str,
    severity: &str,
    message: &str,
) -> Value {
    serde_json::to_value(PeriodV2QualityIssue {
        path: path.to_string(),
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
    })
    .unwrap_or_else(
        |_| json!({ "path": path, "code": code, "severity": severity, "message": message }),
    )
}
pub(crate) fn validate_period_response_quality_gates_v2(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    if !is_period_writer_request_v2(request) {
        return Ok(());
    }
    validate_period_response_contract_gates_v2(request, response)
}
pub(crate) async fn period_style_editor_response_v2(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    response: &Value,
    error: &GenerationError,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response_v2(request);
    }
    let schema = period_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: period_style_editor_messages_v2(request, response, error)?,
        structured_schema: Some(schema),
        reasoning_effort: period_writer_reasoning_effort(request),
        temperature: Some(0.2),
        max_output_tokens: Some(period_writer_max_output_tokens(request)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: run_id
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE)
                .to_string(),
            chapter_code: Some("period_v2_quality_retry".to_string()),
        },
    };
    let routed = use_case
        .router
        .generate(
            provider_request,
            defaults.provider.clone(),
            &defaults.model,
            false,
            true,
            ModelRouteContext::PrimaryReading,
        )
        .await?;
    let mut edited = routed        .response        .parsed_json        .or_else(|| parse_period_provider_json(&routed.response.raw_text))        .ok_or_else(|| {            let incomplete_reason =                period_provider_incomplete_reason(&routed.response.provider_metadata);            GenerationError::with_details(                GenerationErrorCode::PostSafetyValidationFailed,                format!(                    "HOROSCOPE_PERIOD_RESPONSE_INVALID: editor_response_not_json raw_text_len={}",                    routed.response.raw_text.len()                ),                json!({                    "reason": "editor_response_not_json",                    "raw_text_len": routed.response.raw_text.len(),                    "provider_incomplete_reason": incomplete_reason                }),            )        })?;
    if !edited
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        edited["quality"] = json!({});
    }
    edited["quality"]["provider"] = json!(routed.used_provider.as_str());
    edited["quality"]["model"] = json!(routed.response.model_used);
    edited["quality"]["fallback_used"] = json!(routed.fallback_used);
    repair_period_response_shape_v2(request, &mut edited);
    edited = postprocess_period_provider_response_v2(request, edited);
    validate_period_response_quality_gates_v2(request, &edited)?;
    Ok(edited)
}
pub fn period_v2_quality_audit(response: &Value) -> Value {
    let mut public_text = String::new();
    collect_period_v2_public_text_only(response, &mut public_text);
    json!({        "mode": "non_blocking",        "public_word_count": simple_public_word_count(&public_text),        "section_word_counts": period_v2_section_word_counts(response),        "top_repeated_terms": period_v2_top_repeated_terms(&public_text, 8),        "duplicate_titles": period_v2_duplicate_titles(response),        "window_title_time_mismatches": period_v2_window_title_time_mismatches(response)    })
}
pub fn period_v2_editorial_audit(_request: &Value, response: &Value) -> Value {
    period_v2_quality_audit(response)
}
pub(crate) fn period_v2_section_word_counts(response: &Value) -> Value {
    let mut counts = serde_json::Map::new();
    for (label, pointer) in [
        ("week_overview", "/week_overview/text"),
        ("strategy", "/strategy/text"),
        ("advice", "/advice/main"),
        ("watch_summary", "/watch_summary/text"),
    ] {
        let count = response
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(simple_public_word_count)
            .unwrap_or(0);
        counts.insert(label.to_string(), json!(count));
    }
    for field in [
        "daily_timeline",
        "domain_sections",
        "best_windows",
        "watch_windows",
    ] {
        let total = response[field]
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| {
                let mut text = String::new();
                for key in ["title", "text", "reason", "watch_point", "advice"] {
                    if let Some(value) = item.get(key).and_then(Value::as_str) {
                        text.push_str(value);
                        text.push(' ');
                    }
                }
                simple_public_word_count(&text)
            })
            .sum::<usize>();
        counts.insert(field.to_string(), json!(total));
    }
    Value::Object(counts)
}
pub(crate) fn period_v2_top_repeated_terms(public_text: &str, limit: usize) -> Value {
    let mut counts = HashMap::<String, usize>::new();
    for raw in public_text
        .split(|ch: char| !ch.is_alphanumeric() && ch != '\'' && ch != '’')
        .map(|word| word.trim_matches(['\'', '’']).to_lowercase())
        .filter(|word| word.chars().count() > 4)
        .filter(|word| !period_v2_audit_stopword(word))
    {
        *counts.entry(raw).or_default() += 1;
    }
    let mut items = counts.into_iter().collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    Value::Array(
        items
            .into_iter()
            .take(limit)
            .map(|(term, count)| json!({ "term": term, "count": count }))
            .collect(),
    )
}
pub(crate) fn period_v2_audit_stopword(word: &str) -> bool {
    matches!(
        word,
        "cette"
            | "votre"
            | "leurs"
            | "faire"
            | "entre"
            | "comme"
            | "quand"
            | "pourra"
            | "avant"
            | "après"
            | "jours"
            | "semaine"
    )
}
pub(crate) fn period_v2_duplicate_titles(response: &Value) -> Value {
    let mut counts = HashMap::<String, usize>::new();
    for field in [
        "key_days",
        "best_days",
        "watch_days",
        "daily_timeline",
        "domain_sections",
        "best_windows",
        "watch_windows",
    ] {
        for item in response[field].as_array().into_iter().flatten() {
            if let Some(title) = item
                .get("title")
                .or_else(|| item.get("day_label"))
                .and_then(Value::as_str)
            {
                *counts.entry(normalized_text(title)).or_default() += 1;
            }
        }
    }
    Value::Array({
        let mut items = counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .collect::<Vec<_>>();
        items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        items
            .into_iter()
            .map(|(title, count)| json!({ "title": title, "count": count }))
            .collect()
    })
}
pub(crate) fn period_v2_window_title_time_mismatches(response: &Value) -> Value {
    let mut mismatches = Vec::new();
    for field in ["best_windows", "watch_windows"] {
        for window in response[field].as_array().into_iter().flatten() {
            let range = window
                .get("time_range_label")
                .and_then(Value::as_str)
                .unwrap_or("");
            let title = window.get("title").and_then(Value::as_str).unwrap_or("");
            if period_window_title_conflicts_with_time(range, title) {
                mismatches.push(json!({                    "field": field,                    "date": window["date"],                    "time_range_label": range,                    "title": title                }));
            }
        }
    }
    Value::Array(mismatches)
}

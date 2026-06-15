use super::*;
pub(crate) const PERIOD_V2_SEVEN_DAILY_SHAPE_WARNING: &str = "PERIOD_V2_SEVEN_DAILY_SHAPE_WARNING";
pub(crate) const PERIOD_V2_WORD_COUNT_WARNING: &str = "PERIOD_V2_WORD_COUNT_WARNING";
pub(crate) fn period_v2_failure_issue(
    path: &str,
    code: &str,
    severity: &str,
    message: &str,
) -> Value {
    json!({
        "path": path,
        "code": code,
        "severity": severity,
        "message": message
    })
}
pub fn validate_period_response_quality_gates(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    if !is_period_writer_request(request) {
        return Ok(());
    }
    validate_period_response_contract_gates(request, response)
}
pub(crate) async fn period_style_editor_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    response: &Value,
    error: &GenerationError,
    run_id: Option<&str>,
) -> Result<(Value, GenerationStepRecord), GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response_from_writer_request(request).map(|response| {
            (
                response,
                horoscope_generation_step(
                    "horoscope_period_quality_retry",
                    Some("period_quality_retry".to_string()),
                    ProviderKind::Fake.as_str(),
                    defaults.model.clone(),
                    ChapterGenerationStatus::Repaired,
                    None,
                    0,
                    None,
                ),
            )
        });
    }
    let schema = period_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: period_style_editor_messages(request, response, error)?,
        structured_schema: Some(schema),
        reasoning_effort: period_writer_reasoning_effort(request),
        temperature: Some(0.2),
        max_output_tokens: Some(period_style_editor_max_output_tokens(request)),
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
            chapter_code: Some("period_quality_retry".to_string()),
            prompt_trace_step: Some("horoscope_period_quality_retry".into()),
            prompt_trace_attempt: Some("repair".into()),
            prompt_family: Some("horoscope_period_writer".into()),
            prompt_version: Some("v1".into()),
        },
    };
    tracing::warn!(
        service_code = %request["service_code"].as_str().unwrap_or("unknown"),
        max_output_tokens = provider_request.max_output_tokens.unwrap_or_default(),
        issue = %error.detail().message,
        "horoscope period quality retry requested"
    );
    let provider_started_at = std::time::Instant::now();
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
    let provider_latency_ms = provider_started_at.elapsed().as_millis();
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
    edited = postprocess_period_provider_response(request, edited);
    validate_period_response_quality_gates(request, &edited)?;
    let step = horoscope_generation_step(
        "horoscope_period_quality_retry",
        Some("period_quality_retry".to_string()),
        routed.used_provider.as_str(),
        routed.response.model_used.clone(),
        ChapterGenerationStatus::Repaired,
        routed.response.usage.clone(),
        provider_latency_ms,
        None,
    );
    Ok((edited, step))
}
pub fn period_quality_audit(response: &Value) -> Value {
    period_quality_audit_with_request(None, response)
}
pub fn period_editorial_audit(request: &Value, response: &Value) -> Value {
    period_quality_audit_with_request(Some(request), response)
}
pub(crate) fn period_quality_audit_with_request(
    request: Option<&Value>,
    response: &Value,
) -> Value {
    let public_text = collect_period_v2_public_text(response);
    json!({
        "mode": "non_blocking",
        "public_word_count": simple_public_word_count(&public_text),
        "section_word_counts": period_v2_section_word_counts(response),
        "warnings": period_quality_warnings_json(request, response, &public_text)
    })
}
pub(crate) fn collect_period_v2_public_text(response: &Value) -> String {
    let mut public_text = String::new();
    collect_period_public_text_only(response, &mut public_text);
    public_text
}
pub(crate) fn period_quality_warnings_json(
    request: Option<&Value>,
    response: &Value,
    public_text: &str,
) -> Value {
    Value::Array(
        period_collect_quality_warnings(request, response, public_text)
            .into_iter()
            .map(|warning| serde_json::to_value(warning).unwrap_or_else(|_| json!({})))
            .collect(),
    )
}
pub(crate) fn period_collect_quality_warnings(
    request: Option<&Value>,
    response: &Value,
    public_text: &str,
) -> Vec<PeriodV2QualityWarning> {
    let mut warnings = Vec::new();
    warnings.extend(period_v2_seven_daily_shape_warnings(response));
    warnings.extend(period_v2_word_count_warnings(
        request,
        response,
        public_text,
    ));
    warnings
}
pub(crate) fn period_v2_warning(
    path: &str,
    code: &str,
    severity: PeriodV2QualitySeverity,
    message: &str,
) -> PeriodV2QualityWarning {
    PeriodV2QualityWarning {
        path: path.to_string(),
        code: code.to_string(),
        severity,
        message: message.to_string(),
    }
}
pub(crate) fn period_v2_seven_daily_shape_warnings(
    response: &Value,
) -> Vec<PeriodV2QualityWarning> {
    if validate_period_not_seven_daily(response).is_err() {
        return vec![period_v2_warning(
            "/reading",
            PERIOD_V2_SEVEN_DAILY_SHAPE_WARNING,
            PeriodV2QualitySeverity::Warning,
            "La lecture ressemble davantage à une suite de daily qu'à une synthèse de période.",
        )];
    }
    Vec::new()
}
pub(crate) fn validate_period_not_seven_daily(response: &Value) -> Result<(), GenerationError> {
    let day_count = response["daily_timeline"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    if day_count != 7 {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_TIMELINE_MISSING",
            json!({ "timeline_count": day_count }),
        ));
    }
    Ok(())
}
pub(crate) fn period_v2_word_count_warnings(
    request: Option<&Value>,
    response: &Value,
    public_text: &str,
) -> Vec<PeriodV2QualityWarning> {
    let Some(request) = request else {
        return Vec::new();
    };
    if response["quality"]["provider"].as_str() == Some("fake") {
        return Vec::new();
    }
    let limits = period_word_limits_for_request(request);
    let word_count = public_text.split_whitespace().count();
    if word_count >= period_effective_min_word_count(request, &limits)
        && word_count < limits.target_min
    {
        return vec![period_v2_warning(
            "/reading",
            PERIOD_V2_WORD_COUNT_WARNING,
            PeriodV2QualitySeverity::Metric,
            "La lecture est légèrement sous la cible éditoriale mais reste dans le contrat produit.",
        )];
    }
    if word_count > limits.target_max && word_count <= limits.hard_limit {
        return vec![period_v2_warning(
            "/reading",
            PERIOD_V2_WORD_COUNT_WARNING,
            PeriodV2QualitySeverity::Metric,
            "La lecture dépasse la cible éditoriale sans violer la borne dure du produit.",
        )];
    }
    Vec::new()
}
pub(crate) fn validate_period_v2_public_text_forbidden_technical_leaks(
    public_text: &str,
) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in [
        "slot:",
        "slot_",
        "[morning]",
        "[afternoon]",
        "[evening]",
        "raw_transits",
        "period:",
        "natal_",
        "fake_",
        "theme_code",
        "evidence_key",
        "snapshot_key",
        "source_snapshot_keys",
        "scan_plan",
        "period_resolution",
        "contract_version",
        "daily_timeline",
        "transit_exact",
        "transit_active",
        "moon_house_by_day",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    Ok(())
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

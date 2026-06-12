use super::*;
pub fn validate_period_public_request(
    payload: &Value,
) -> Result<HoroscopePeriodPublicRequest, GenerationError> {
    let mut request: HoroscopePeriodPublicRequest = serde_json::from_value(payload.clone())
        .map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("HOROSCOPE_PERIOD_PAYLOAD_INVALID: {err}"),
                Value::Null,
            )
        })?;
    if request.chart_calculation_id.trim().is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_NATAL_CHART_REQUIRED"));
    }
    NaiveDate::parse_from_str(&request.anchor_date, "%Y-%m-%d").map_err(|_| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED",
            Value::Null,
        )
    })?;
    if request.timezone.parse::<Tz>().is_err() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED",
            Value::Null,
        ));
    }
    let language = request.normalized_target_language_code()?;
    if payload.get("target_language").is_some() {
        if let Some(explicit_language) = &request.target_language_code {
            let legacy = request.target_language.trim().to_ascii_lowercase();
            if !legacy.is_empty() && legacy != explicit_language.as_str() {
                request.language_compat_warning = Some(
                    json!({                    "legacy_target_language_ignored": true,                    "target_language": legacy,                    "target_language_code": explicit_language.as_str(),                    "reason": "target_language_code_takes_precedence"                }),
                );
            }
        }
    }
    request.target_language = language.as_str().to_string();
    request.target_language_code = Some(language);
    if let Some(persona) = &request.astrologer_persona {
        validate_astrologer_persona(persona)?;
    }
    Ok(request)
}
pub(crate) fn validate_astrologer_persona(
    persona: &AstrologerPersona,
) -> Result<(), GenerationError> {
    validate_persona_vec("tone", &persona.tone, 8)?;
    validate_persona_vec("lexical_field", &persona.lexical_field, 20)?;
    validate_persona_vec("priority_domains", &persona.priority_domains, 12)?;
    validate_persona_vec("avoid_style", &persona.avoid_style, 20)?;
    for value in persona
        .tone
        .iter()
        .chain(persona.lexical_field.iter())
        .chain(persona.priority_domains.iter())
        .chain(persona.avoid_style.iter())
    {
        validate_persona_fragment(value)?;
    }
    for value in [
        persona.persona_id.as_deref(),
        persona.interpretation_style.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_persona_fragment(value)?;
    }
    Ok(())
}
pub(crate) fn validate_persona_vec(
    field: &str,
    values: &[String],
    max: usize,
) -> Result<(), GenerationError> {
    if values.len() > max {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_PERSONA_INVALID",
            json!({ "field": field, "max_items": max }),
        ));
    }
    Ok(())
}
pub(crate) fn validate_persona_fragment(value: &str) -> Result<(), GenerationError> {
    let trimmed = value.trim();
    if trimmed.len() > 120 {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_PERSONA_INVALID",
            json!({ "reason": "fragment_too_long" }),
        ));
    }
    let lower = trimmed.to_ascii_lowercase();
    let forbidden = [
        "ignore previous",
        "ignore toutes",
        "system prompt",
        "developer message",
        "hors schema",
        "diagnostic médical",
        "diagnostic medical",
        "certitude",
        "tu vas mourir",
        "gain garanti",
    ];
    if forbidden.iter().any(|pattern| lower.contains(pattern)) {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_PERSONA_INVALID",
            json!({ "reason": "forbidden_fragment" }),
        ));
    }
    Ok(())
}

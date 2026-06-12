use super::*;
pub fn validate_response_evidence(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    validate_horoscope_response_schema(response)?;
    let service_code = request
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if response.get("contract_version").and_then(|v| v.as_str()) != Some("horoscope_response_v1")
        || response.get("service_code").and_then(|v| v.as_str()) != Some(service_code)
    {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let allowed = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("evidence_key").and_then(|v| v.as_str()))
        .collect::<HashSet<_>>();
    let request_slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if allowed.is_empty() {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        return validate_free_response_evidence(request, response, &allowed);
    }
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return validate_premium_response_evidence(request, response, &allowed);
    }
    if service_code != HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let slots = response
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if slots.len() != 3 {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    validate_day_overview_not_copied(request, slots)?;
    let mut texts = Vec::new();
    let mut advices = Vec::new();
    let mut best_for_sets = Vec::new();
    for slot in slots {
        let slot_code = slot
            .get("slot_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        let keys = slot
            .get("evidence_keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        if keys.is_empty() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "HOROSCOPE_EVIDENCE_MISMATCH",
                json!({ "reason": "slot_without_evidence" }),
            ));
        }
        if keys.iter().any(|key| key.as_str().is_none()) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                "HOROSCOPE_EVIDENCE_MISMATCH",
                json!({ "reason": "non_string_evidence_key" }),
            ));
        }
        let request_slot = request_slots
            .iter()
            .find(|item| item.get("slot_code").and_then(|v| v.as_str()) == Some(slot_code))
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        validate_slot_specificity(request_slot)?;
        validate_slot_evidence_alignment(request_slot, keys)?;
        validate_public_slot_text(slot)?;
        let text = slot.get("text").and_then(|v| v.as_str()).unwrap_or("");
        validate_astrological_reference(slot_code, text, request_slot)?;
        texts.push(text.to_string());
        advices.push(
            slot.get("advice")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        );
        best_for_sets.push(
            slot.get("best_for")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .map(str::to_string)
                .collect::<Vec<_>>(),
        );
    }
    validate_slot_diversity(&texts)?;
    validate_distinct_strings(&advices, "HOROSCOPE_SLOT_ADVICE_DUPLICATED")?;
    validate_distinct_best_for(&best_for_sets)?;
    let mut cited = Vec::new();
    collect_evidence_keys(response, &mut cited);
    let invented = cited
        .into_iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    if invented.is_empty() {
        Ok(())
    } else {
        Err(GenerationError::with_details(
            GenerationErrorCode::PostSafetyValidationFailed,
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "invented_evidence_keys": invented }),
        ))
    }
}
pub(crate) fn validate_premium_calculation_local_chart(
    service_code: &str,
    calculation: &Value,
) -> Result<(), GenerationError> {
    if service_code != HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return Ok(());
    }
    let slots = calculation
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    if slots.len() != 12 {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_TIMELINE_MISSING",
            json!({ "reason": "premium_calculation_must_have_12_slots" }),
        ));
    }
    for slot in slots {
        let local_chart = slot
            .get("local_chart")
            .and_then(|v| v.as_object())
            .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING", Value::Null))?;
        if !local_chart.contains_key("ascendant")
            || !local_chart.contains_key("midheaven")
            || !local_chart.contains_key("houses")
        {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING",
                json!({ "reason": "ascendant_midheaven_or_houses_missing" }),
            ));
        }
        if !local_chart
            .get("ascendant")
            .and_then(|v| v.as_object())
            .is_some_and(|angle| angle.contains_key("sign") && angle.contains_key("longitude_deg"))
            || !local_chart
                .get("midheaven")
                .and_then(|v| v.as_object())
                .is_some_and(|angle| {
                    angle.contains_key("sign") && angle.contains_key("longitude_deg")
                })
            || local_chart
                .get("houses")
                .and_then(|v| v.as_array())
                .map(|houses| houses.len() != 12)
                .unwrap_or(true)
        {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_LOCAL_CHART_MISSING",
                json!({ "reason": "local_chart_shape_invalid" }),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_free_response_evidence(
    request: &Value,
    response: &Value,
    allowed: &HashSet<&str>,
) -> Result<(), GenerationError> {
    if response.get("slots").is_some() {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let request_slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if request_slots.len() != 1
        || request_slots[0].get("slot_code").and_then(|v| v.as_str()) != Some("day")
    {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let evidence_keys = response
        .get("evidence_keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if evidence_keys.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "reason": "free_without_evidence" }),
        ));
    }
    validate_slot_evidence_alignment(&request_slots[0], evidence_keys)?;

    let mut cited = Vec::new();
    collect_evidence_keys(response, &mut cited);
    let invented = cited
        .into_iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    if !invented.is_empty() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::PostSafetyValidationFailed,
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "invented_evidence_keys": invented }),
        ));
    }

    let public_text = free_public_text(response);
    validate_public_text_no_technical_codes(&public_text)?;
    validate_free_text_quality(&public_text, response)?;
    validate_astrological_reference("day", &public_text, &request_slots[0])?;
    Ok(())
}

pub(crate) fn validate_premium_response_evidence(
    request: &Value,
    response: &Value,
    allowed: &HashSet<&str>,
) -> Result<(), GenerationError> {
    let timeline = response
        .get("timeline")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_TIMELINE_MISSING", Value::Null))?;
    let request_slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if timeline.len() != 12 || request_slots.len() != 12 {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_TIMELINE_MISSING",
            json!({ "reason": "timeline_must_have_exactly_12_entries" }),
        ));
    }
    for (idx, (response_slot, request_slot)) in timeline.iter().zip(request_slots).enumerate() {
        let expected_label = request_slot
            .get("slot_label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        let received_label = response_slot
            .get("slot_label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        if received_label != expected_label {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_TIMELINE_MISSING",
                json!({
                    "reason": "timeline_label_order_mismatch",
                    "index": idx,
                    "expected": expected_label,
                    "received": received_label
                }),
            ));
        }
        validate_public_slot_text(response_slot)?;
        let keys = response_slot
            .get("evidence_keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING", Value::Null))?;
        if keys.is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING",
                json!({ "slot_label": expected_label }),
            ));
        }
        validate_slot_evidence_alignment(request_slot, keys)?;
    }

    let request_by_label = request_slots
        .iter()
        .filter_map(|slot| Some((slot.get("slot_label")?.as_str()?, slot)))
        .collect::<HashMap<_, _>>();
    let best = response
        .get("best_slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_BEST_SLOTS_MISSING", Value::Null))?;
    let watch = response
        .get("watch_slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_WATCH_SLOTS_MISSING", Value::Null))?;
    if best.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_BEST_SLOTS_MISSING",
            Value::Null,
        ));
    }
    if watch.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_WATCH_SLOTS_MISSING",
            Value::Null,
        ));
    }
    validate_premium_slot_summaries(best, &request_by_label, "best_slots")?;
    validate_premium_slot_summaries(watch, &request_by_label, "watch_slots")?;
    validate_premium_slot_summary_reason_diversity(best, "best_slots")?;
    validate_premium_slot_summary_reason_diversity(watch, "watch_slots")?;
    let best_labels = best
        .iter()
        .filter_map(|slot| slot.get("slot_label").and_then(|v| v.as_str()))
        .collect::<HashSet<_>>();
    let watch_labels = watch
        .iter()
        .filter_map(|slot| slot.get("slot_label").and_then(|v| v.as_str()))
        .collect::<HashSet<_>>();
    if best_labels.iter().any(|label| watch_labels.contains(label)) {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_CONTRADICTORY_SLOT_CLASSIFICATION",
            json!({ "reason": "slot_in_best_and_watch" }),
        ));
    }

    let domain_sections = response
        .get("domain_sections")
        .and_then(|v| v.as_array())
        .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_DOMAIN_SECTION_MISSING", Value::Null))?;
    if domain_sections.is_empty() {
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_DOMAIN_SECTION_MISSING",
            Value::Null,
        ));
    }

    let mut cited = Vec::new();
    collect_evidence_keys(response, &mut cited);
    let invented = cited
        .into_iter()
        .filter(|key| !allowed.contains(key.as_str()))
        .collect::<Vec<_>>();
    if invented.is_empty() {
        Ok(())
    } else {
        Err(GenerationError::with_details(
            GenerationErrorCode::PostSafetyValidationFailed,
            "HOROSCOPE_EVIDENCE_MISMATCH",
            json!({ "invented_evidence_keys": invented }),
        ))
    }
}

pub(crate) fn validate_premium_slot_summaries(
    slots: &[Value],
    request_by_label: &HashMap<&str, &Value>,
    field: &str,
) -> Result<(), GenerationError> {
    let mut seen = HashSet::new();
    for slot in slots {
        let label = slot
            .get("slot_label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
        if !seen.insert(label) {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_DUPLICATED_SLOT_CLASSIFICATION",
                json!({ "field": field, "slot_label": label }),
            ));
        }
        let request_slot = request_by_label.get(label).ok_or_else(|| {
            quality_error(
                "HOROSCOPE_PREMIUM_UNKNOWN_SLOT_CLASSIFICATION",
                json!({ "field": field, "slot_label": label }),
            )
        })?;
        let keys = slot
            .get("evidence_keys")
            .and_then(|v| v.as_array())
            .ok_or_else(|| quality_error("HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING", Value::Null))?;
        if keys.is_empty() {
            return Err(quality_error(
                "HOROSCOPE_PREMIUM_SLOT_EVIDENCE_MISSING",
                json!({ "field": field, "slot_label": label }),
            ));
        }
        validate_slot_evidence_alignment(request_slot, keys)?;
        validate_premium_summary_public_text(slot)?;
    }
    Ok(())
}

pub(crate) fn validate_premium_slot_summary_reason_diversity(
    slots: &[Value],
    field: &str,
) -> Result<(), GenerationError> {
    let mut seen = HashSet::new();
    for slot in slots {
        let reason = slot.get("reason").and_then(Value::as_str).unwrap_or("");
        let normalized = normalize_editorial_sentence(reason);
        if normalized.is_empty() || seen.insert(normalized) {
            continue;
        }
        return Err(quality_error(
            "HOROSCOPE_PREMIUM_REPETITIVE_SLOT_REASON",
            json!({ "field": field, "reason": reason }),
        ));
    }
    Ok(())
}

pub(crate) fn validate_premium_summary_public_text(slot: &Value) -> Result<(), GenerationError> {
    let mut public_text = String::new();
    for key in ["slot_label", "title", "reason"] {
        if let Some(value) = slot.get(key).and_then(|v| v.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for key in ["best_for", "avoid"] {
        for value in slot
            .get(key)
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
        {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    validate_public_text_no_technical_codes(&public_text)
}

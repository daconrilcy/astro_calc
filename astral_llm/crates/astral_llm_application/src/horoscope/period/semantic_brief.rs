use super::*;
pub fn build_period_writer_request(
    public: &HoroscopePeriodPublicRequest,
    calculation: &Value,
) -> Result<Value, GenerationError> {
    let service_code = calculation
        .get("service_code")
        .and_then(Value::as_str)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    validate_period_service_code(service_code)?;
    let service_profile = period_service_profile(service_code)?;
    let detail_profile_code = service_profile
        .detail_profile_code
        .as_deref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"))?;
    let detail = period_detail_profile(detail_profile_code)?;
    let period_resolution = calculation
        .get("period_resolution")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    validate_period_writer_v2_next_7_days_contract(&period_resolution)?;
    let scan_plan = calculation
        .get("scan_plan")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    validate_scan_plan(&period_resolution, &scan_plan)?;
    let snapshots = calculation
        .get("snapshots")
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    let raw_evidence = period_evidence_from_snapshots(snapshots)?
        .into_iter()
        .take(detail.max_evidence)
        .collect::<Vec<_>>();
    if raw_evidence.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    let events = build_period_events(&raw_evidence, &period_resolution)?;
    let evidence = sanitize_writer_v2_evidence(&raw_evidence);
    let semantic_brief =
        build_period_semantic_brief(&period_resolution, &scan_plan, &evidence, &events, &detail)?;
    let language = public.normalized_target_language_code()?;
    let astrologer_persona = public
        .astrologer_persona
        .clone()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("HOROSCOPE_PERIOD_PERSONA_INVALID: {err}"),
                Value::Null,
            )
        })?
        .unwrap_or(Value::Null);
    let request = json!({
        "contract_version": "horoscope_period_writer_request",
        "service_code": service_code,
        "target_language_code": language.as_str(),
        "astrologer_persona": astrologer_persona,
        "period_resolution": period_resolution,
        "scan_plan": scan_plan,
        "detail_profile_code": detail_profile_code,
        "semantic_brief": semantic_brief,
        "evidence": evidence,
        "safety_profile": astrology_public_safety_profile(),
        "output_contract_version": "horoscope_period_response"
    });
    validate_period_writer_request_schema(&request)?;
    Ok(request)
}

pub(crate) fn validate_period_writer_v2_next_7_days_contract(
    period_resolution: &Value,
) -> Result<(), GenerationError> {
    if period_resolution["period_profile_code"].as_str() != Some("next_7_days") {
        return Err(horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"));
    }
    let included_dates = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"))?;
    if included_dates.len() != 7 || period_resolution["duration_days"].as_i64() != Some(7) {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    Ok(())
}

pub(crate) fn sanitize_writer_v2_evidence(items: &[Value]) -> Vec<Value> {
    items        .iter()        .map(|item| {            json!({                "evidence_key": item["evidence_key"],                "snapshot_key": item["snapshot_key"],                "date": item["date"],                "fact_type": item["fact_type"],                "transiting_object": item["transiting_object"],                "aspect": item["aspect"],                "natal_target": item["natal_target"],                "natal_house": item["natal_house"],                "theme_code": item["theme_code"],                "tone_code": item["tone"],                "score": item["score"]            })        })        .collect()
}
pub(crate) fn build_period_semantic_brief(
    period_resolution: &Value,
    scan_plan: &Value,
    evidence: &[Value],
    events: &[Value],
    detail: &PeriodDetailProfile,
) -> Result<Value, GenerationError> {
    let included_dates = period_resolution["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    validate_period_writer_v2_next_7_days_contract(period_resolution)?;
    let daily_signal_summary = included_dates
        .iter()
        .enumerate()
        .map(|(index, date)| build_daily_signal_summary_v2(date, index, events))
        .collect::<Vec<_>>();
    let dominant_keywords = dominant_keywords_v2(events, 8);
    let period_arc_keywords = period_arc_keywords_v2(events, 8);
    let week_tone_codes = dominant_tone_codes_v2(events, 4);
    let week_intensity = week_intensity_v2(events);
    let key_day_candidates = build_day_candidates_v2(events, "key", detail.max_key_days);
    let watch_day_candidates = if detail.include_watch_days {
        build_day_candidates_v2(
            &events
                .iter()
                .filter(|event| is_period_watch_event(event))
                .cloned()
                .collect::<Vec<_>>(),
            "watch",
            detail.max_watch_days,
        )
    } else {
        Vec::new()
    };
    let key_dates = key_day_candidates
        .iter()
        .filter_map(|day| day["date"].as_str().map(str::to_string))
        .collect::<HashSet<_>>();
    let watch_dates = watch_day_candidates
        .iter()
        .filter_map(|day| day["date"].as_str().map(str::to_string))
        .collect::<HashSet<_>>();
    let best_day_candidates = if detail.include_best_days {
        build_day_candidates_v2(
            &events
                .iter()
                .filter(|event| {
                    event["date"].as_str().is_some_and(|date| {
                        !watch_dates.contains(date) && !key_dates.contains(date)
                    }) && is_period_best_candidate(event)
                })
                .cloned()
                .collect::<Vec<_>>(),
            "best",
            detail.max_best_days,
        )
    } else {
        Vec::new()
    };
    let mut window_candidates = Vec::new();
    if detail.include_best_windows {
        window_candidates.extend(build_window_candidates_v2(
            events,
            scan_plan,
            "best",
            detail.max_best_windows,
            is_period_best_candidate,
        ));
    }
    if detail.include_watch_windows {
        let best_snapshot_keys = window_candidates
            .iter()
            .flat_map(|window| {
                window["source_snapshot_keys"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<HashSet<_>>();
        window_candidates.extend(build_window_candidates_v2(
            events,
            scan_plan,
            "watch",
            detail.max_watch_windows,
            |event| {
                is_period_watch_event(event)
                    && event["snapshot_key"]
                        .as_str()
                        .map_or(true, |key| !best_snapshot_keys.contains(key))
            },
        ));
    }
    Ok(
        json!({        "editorial_arc": build_period_editorial_arc_v2(&included_dates),        "editorial_angles": build_period_editorial_angles_v2(&daily_signal_summary),        "section_roles": build_period_premium_section_roles_v2(),        "period_arc_keywords": period_arc_keywords,        "dominant_keywords": dominant_keywords,        "week_tone_codes": week_tone_codes,        "week_intensity": week_intensity,        "daily_signal_summary": daily_signal_summary,        "best_day_candidates": best_day_candidates,        "watch_day_candidates": watch_day_candidates,        "key_day_candidates": key_day_candidates,        "window_candidates": window_candidates,        "domain_candidates": build_domain_candidates_v2(evidence, detail.max_domain_sections),        "repeating_arcs": build_repeating_arcs_v2(events)    }),
    )
}
pub(crate) fn build_period_editorial_arc_v2(included_dates: &[&str]) -> Value {
    let days = included_dates        .iter()        .enumerate()        .map(|(index, date)| {            let phase = match index {                0 => "ouverture",                1 | 2 => "mise_en_mouvement",                3 => "pivot",                4 | 5 => "consolidation",                _ => "cloture",            };            let function = match index {                0 => "installer le cadre de la semaine sans tout décider",                1 | 2 => "mettre les premiers choix en pratique",                3 => "changer de rythme et trier ce qui mérite d'être gardé",                4 | 5 => "intégrer les apprentissages et alléger les promesses",                _ => "rendre visible ce qui est prêt et préparer la suite",            };            json!({                "date": date,                "phase": phase,                "editorial_function": function            })        })        .collect::<Vec<_>>();
    json!({        "purpose": "Donner une trajectoire lisible: ouverture, pivot, consolidation, clôture.",        "days": days    })
}
pub(crate) fn build_period_editorial_angles_v2(daily_signal_summary: &[Value]) -> Vec<Value> {
    let mut used_angles = HashSet::<String>::new();
    daily_signal_summary        .iter()        .enumerate()        .map(|(index, day)| {            let theme = day["theme_codes"]                .as_array()                .and_then(|items| items.first())                .and_then(Value::as_str)                .unwrap_or("organization");            let angle = period_editorial_angle_v2(theme, index, &mut used_angles);            json!({                "date": day["date"],                "angle_code": angle.0,                "angle_hint": angle.1,                "avoid_repetition_hint": period_editorial_repetition_hint_v2(theme)            })        })        .collect()
}
pub(crate) fn period_editorial_angle_v2(
    theme: &str,
    index: usize,
    used_angles: &mut HashSet<String>,
) -> (&'static str, &'static str) {
    let preferred = match period_editorial_theme_key(theme) {
        "relationship" => (
            "relation",
            "pacifier un lien ou formuler une attente sans mise en scène",
        ),
        "communication" => (
            "clarification",
            "dire moins mais mieux, avec une demande ou une preuve précise",
        ),
        "energy" => ("action", "transformer l'élan en geste court et réversible"),
        "integration" => (
            "consolidation",
            "laisser mûrir avant d'élargir le mouvement",
        ),
        "clarity" => ("nomination", "nommer ce qui compte avant de choisir"),
        _ => (
            "organisation",
            "mettre de l'ordre dans une priorité observable",
        ),
    };
    if used_angles.insert(preferred.0.to_string()) {
        return preferred;
    }
    let fallback = match index {
        0 => (
            "ouverture",
            "installer le cadre sans fermer trop vite les options",
        ),
        1 | 2 => (
            "mise_en_pratique",
            "tester une décision dans un geste limité",
        ),
        3 => ("pivot", "changer de rythme et sélectionner l'essentiel"),
        4 | 5 => (
            "integration",
            "relier ce qui a été compris à une action réaliste",
        ),
        _ => (
            "finalisation",
            "conclure ce qui est prêt et préparer la suite",
        ),
    };
    used_angles.insert(fallback.0.to_string());
    fallback
}
pub(crate) fn period_editorial_repetition_hint_v2(theme: &str) -> &'static str {
    match period_editorial_theme_key(theme) {
        "relationship" => "Varier lien: écoute, réparation, limite douce, geste concret.",
        "communication" => {
            "Varier parole: message ciblé, négociation, reformulation, vérification."
        }
        "energy" => "Varier action: lancement, canalisation, rythme, récupération.",
        "integration" => {
            "Varier intégration: tri, consolidation, patience, décision proportionnée."
        }
        "clarity" => "Varier clarté: désir nommé, choix assumé, visibilité, simplification.",
        _ => "Varier organisation: cadre, ressource, routine, service, visibilité.",
    }
}
pub(crate) fn build_period_premium_section_roles_v2() -> Value {
    json!({        "overview_role": "trajectoire de période, pas résumé des sept jours",        "timeline_role": "vécu quotidien différencié, un usage concret par date",        "domains_role": "synthèse transversale, sans recopier la timeline",        "best_window_role": "créneaux d'usage concret liés à l'heure",        "watch_window_role": "ralentir ou vérifier seulement si fourni",        "strategy_role": "arbitrage final sans refaire le calendrier"    })
}
pub(crate) fn build_daily_signal_summary_v2(date: &str, index: usize, events: &[Value]) -> Value {
    let day_events = events
        .iter()
        .filter(|event| event["date"].as_str() == Some(date))
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    let evidence_keys = day_events
        .iter()
        .flat_map(|event| event["evidence_keys"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let evidence_keys = unique_string_values_from_iter(evidence_keys, 4);
    let theme_codes = unique_event_strings(&day_events, "theme_code", 3);
    let tone_codes = unique_event_strings(&day_events, "tone", 3);
    let keywords = unique_keywords(
        day_events
            .iter()
            .flat_map(period_keywords_for_fact)
            .collect::<Vec<_>>(),
        8,
    );
    let opportunity_keywords = non_empty_keywords(
        day_events
            .iter()
            .filter(|event| is_period_best_candidate(event))
            .flat_map(period_keywords_for_fact)
            .take(5)
            .collect::<Vec<_>>(),
        &keywords,
    );
    let risk_keywords = non_empty_keywords(
        day_events
            .iter()
            .filter(|event| is_period_watch_event(event))
            .flat_map(period_keywords_for_fact)
            .take(5)
            .collect::<Vec<_>>(),
        &keywords,
    );
    json!({        "date": date,        "day_index": index,        "main_event_keys": day_events.iter().filter_map(|event| event["event_key"].as_str()).take(3).collect::<Vec<_>>(),        "evidence_keys": evidence_keys,        "theme_codes": theme_codes,        "tone_codes": tone_codes,        "intensity": day_events.first().and_then(|event| event["intensity"].as_str()).unwrap_or("medium"),        "role_hint": period_role_hint(index),        "keywords": non_empty_keywords(keywords, &["daily_signal".to_string()]),        "opportunity_keywords": opportunity_keywords,        "risk_keywords": risk_keywords,        "avoid_keywords": ["overpromise", "overinterpretation"]    })
}
pub(crate) fn non_empty_keywords(mut values: Vec<String>, fallback: &[String]) -> Vec<String> {
    values.retain(|value| !value.trim().is_empty());
    values.sort();
    values.dedup();
    if values.is_empty() {
        fallback.iter().take(1).cloned().collect()
    } else {
        values
    }
}
pub(crate) fn build_day_candidates_v2(
    events: &[Value],
    candidate_type: &str,
    limit: usize,
) -> Vec<Value> {
    let mut used_dates = HashSet::new();
    events        .iter()        .filter(|event| {            event["date"]                .as_str()                .is_some_and(|date| used_dates.insert(date.to_string()))        })        .take(limit)        .map(|event| {            json!({                "date": event["date"],                "candidate_type": candidate_type,                "score": event["score"],                "keywords": unique_keywords(period_keywords_for_fact(event), 8),                "evidence_keys": event["evidence_keys"]            })        })        .collect()
}
pub(crate) fn build_window_candidates_v2<F>(
    events: &[Value],
    scan_plan: &Value,
    candidate_type: &str,
    limit: usize,
    predicate: F,
) -> Vec<Value>
where
    F: Fn(&Value) -> bool,
{
    let snapshot_keys = scan_plan_snapshot_keys_by_date(scan_plan);
    let mut out = Vec::new();
    let mut used_dates = HashSet::new();
    for event in events.iter().filter(|event| predicate(event)) {
        let Some(candidate) = build_window_candidate_v2(event, &snapshot_keys, candidate_type)
        else {
            continue;
        };
        let date = candidate["date"].as_str().unwrap_or("");
        if !used_dates.insert(date.to_string()) {
            continue;
        }
        out.push(candidate);
        if out.len() == limit {
            break;
        }
    }
    out
}
pub(crate) fn build_window_candidate_v2(
    event: &Value,
    snapshot_keys: &HashMap<String, Vec<(String, String)>>,
    candidate_type: &str,
) -> Option<Value> {
    let date = event["date"].as_str()?;
    let window = build_period_window(event, snapshot_keys, candidate_type == "watch", 1)?;
    Some(
        json!({        "date": date,        "time_range_label": window["time_range_label"],        "score": event["score"],        "usage_keywords": unique_keywords(period_keywords_for_fact(event), 8),        "tone_code": event["tone"],        "theme_code": event["theme_code"],        "evidence_keys": event["evidence_keys"],        "source_snapshot_keys": window["source_snapshot_keys"]    }),
    )
}
pub(crate) fn build_domain_candidates_v2(evidence: &[Value], max_sections: usize) -> Vec<Value> {
    let mut by_theme: HashMap<String, Vec<&Value>> = HashMap::new();
    for item in evidence {
        by_theme
            .entry(
                item["theme_code"]
                    .as_str()
                    .unwrap_or("organization")
                    .to_string(),
            )
            .or_default()
            .push(item);
    }
    let mut out = by_theme        .into_iter()        .map(|(theme, items)| {            json!({                "domain_code": theme,                "weight": ((items.len() as f64) / (max_sections.max(1) as f64)).min(1.0),                "keywords": unique_keywords(items.iter().flat_map(|item| period_keywords_for_fact(item)).collect::<Vec<_>>(), 8),                "evidence_keys": unique_string_values_from_iter(items.iter().filter_map(|item| item["evidence_key"].as_str()), 4)            })        })        .collect::<Vec<_>>();
    out.sort_by(|left, right| {
        right["weight"]
            .as_f64()
            .partial_cmp(&left["weight"].as_f64())
            .unwrap_or(Ordering::Equal)
    });
    out.truncate(max_sections);
    out
}
pub(crate) fn build_repeating_arcs_v2(events: &[Value]) -> Vec<Value> {
    let mut grouped: HashMap<String, Vec<&Value>> = HashMap::new();
    for event in events {
        let signature = format!(
            "{}|{}|{}|{}",
            event["transiting_object"].as_str().unwrap_or(""),
            event["aspect"].as_str().unwrap_or(""),
            event["natal_target"].as_str().unwrap_or(""),
            event["theme_code"].as_str().unwrap_or("")
        );
        grouped.entry(signature).or_default().push(event);
    }
    grouped        .into_iter()        .filter(|(_, items)| items.len() > 1)        .take(5)        .map(|(signature, items)| {            let dates = unique_string_values_from_iter(                items.iter().filter_map(|event| event["date"].as_str()),                8,            );            let evidence_keys = unique_string_values_from_iter(                items.iter().flat_map(|event| {                    event["evidence_keys"]                        .as_array()                        .into_iter()                        .flatten()                        .filter_map(Value::as_str)                }),                8,            );            json!({                "signature_code": signature,                "dates": dates,                "dominant_keywords": unique_keywords(                    items                        .iter()                        .flat_map(|event| period_keywords_for_fact(event))                        .collect::<Vec<_>>(),                    8,                ),                "evidence_keys": evidence_keys            })        })        .collect()
}
pub(crate) fn unique_string_values_from_iter<'a, I>(values: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .filter_map(|value| {
            if seen.insert(value.to_string()) {
                Some(value.to_string())
            } else {
                None
            }
        })
        .take(limit)
        .collect()
}
pub(crate) fn dominant_keywords_v2(events: &[Value], limit: usize) -> Vec<String> {
    unique_keywords(
        events
            .iter()
            .flat_map(period_keywords_for_fact)
            .collect::<Vec<_>>(),
        limit,
    )
}
pub(crate) fn period_arc_keywords_v2(events: &[Value], limit: usize) -> Vec<String> {
    let mut values = Vec::new();
    values.extend(unique_event_strings(events, "theme_code", limit));
    values.extend(unique_event_strings(events, "fact_type", limit));
    unique_keywords(values, limit)
}
pub(crate) fn dominant_tone_codes_v2(events: &[Value], limit: usize) -> Vec<String> {
    let values = unique_event_strings(events, "tone", limit);
    if values.is_empty() {
        vec!["focused".to_string()]
    } else {
        values
    }
}
pub(crate) fn week_intensity_v2(events: &[Value]) -> &'static str {
    let max_score = events
        .iter()
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0_f64, f64::max);
    if max_score >= 0.72 {
        "high"
    } else if max_score >= 0.42 {
        "medium"
    } else {
        "low"
    }
}
pub(crate) fn unique_keywords(values: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .filter(|value| seen.insert(value.clone()))
        .take(limit)
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push("weekly_signal".to_string());
    }
    out
}
pub(crate) fn period_keywords_for_fact(value: &Value) -> Vec<String> {
    let mut keywords = Vec::new();
    for field in [
        "theme_code",
        "tone",
        "tone_code",
        "fact_type",
        "transiting_object",
        "aspect",
        "natal_target",
        "candidate_type",
    ] {
        if let Some(raw) = value.get(field).and_then(Value::as_str) {
            if !raw.trim().is_empty() {
                keywords.push(raw.trim().to_string());
            }
        }
    }
    keywords.sort();
    keywords.dedup();
    keywords
}
pub(crate) fn unique_event_strings(events: &[Value], field: &str, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    events
        .iter()
        .filter_map(|event| event[field].as_str())
        .filter(|value| seen.insert((*value).to_string()))
        .take(limit)
        .map(str::to_string)
        .collect()
}
pub(crate) fn period_role_hint(index: usize) -> &'static str {
    match index {
        0 => "entry",
        1 | 2 => "development",
        3 => "pivot",
        4 | 5 => "integration",
        _ => "closure",
    }
}
pub(crate) fn astrology_public_safety_profile() -> Value {
    json!({        "domain": "astrology_public_guidance",        "forbid_medical_guidance": true,        "forbid_fatalism": true,        "forbid_financial_promises": true,        "forbid_certain_predictions": true,        "persona_cannot_override_safety": true,        "evidence_keys_must_come_from_request": true    })
}
pub(crate) fn period_editorial_theme_key(theme: &str) -> &str {
    if theme == "routine" {
        "organization"
    } else {
        theme
    }
}

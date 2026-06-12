use super::*;

pub fn build_period_interpretation_request(
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
    let scan_plan = calculation
        .get("scan_plan")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    validate_scan_plan(&period_resolution, &scan_plan)?;
    let snapshots = calculation
        .get("snapshots")
        .and_then(|value| value.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
    let included_dates = period_resolution
        .get("included_dates")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    if included_dates.len() != 7 {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    let scan_profile_code = scan_plan["scan_profile_code"]
        .as_str()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    let scan_profile = scan_profile(scan_profile_code)?;
    if snapshots.len() != included_dates.len() * scan_profile.expected_snapshots_per_day {
        return Err(horoscope_error("HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH"));
    }
    let evidence = period_evidence_from_snapshots(snapshots)?
        .into_iter()
        .take(detail.max_evidence)
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    let events = build_period_events(&evidence, &period_resolution)?;
    let daily_plans = if detail.include_daily_timeline {
        build_daily_plans(&included_dates, &events)?
    } else {
        Vec::new()
    };
    let mut key_days = build_period_key_day_markers(&events, detail.max_key_days);
    if is_free_period_service(service_code) && key_days.is_empty() {
        if let Some(first) = evidence.first() {
            key_days.push(json!({
                "date": first["date"],
                "title": "Jour à retenir",
                "reason": "Un repère utile ressort pour comprendre la tendance sans détailler chaque journée.",
                "evidence_keys": [first["evidence_key"].clone()],
                "fallback_reason": null
            }));
        }
    }
    if is_free_period_service(service_code) {
        for day in &mut key_days {
            day["title"] = json!("Jour à retenir");
        }
    }
    let key_dates = key_days
        .iter()
        .filter_map(|day| day.get("date").and_then(Value::as_str).map(str::to_string))
        .collect::<HashSet<_>>();
    let watch_days = if detail.include_watch_days {
        build_period_watch_day_markers(&events, detail.max_watch_days)
    } else {
        Vec::new()
    };
    let watch_dates = watch_days
        .iter()
        .filter_map(|day| day.get("date").and_then(Value::as_str).map(str::to_string))
        .collect::<HashSet<_>>();
    let best_days = if detail.include_best_days {
        build_period_best_day_markers(&events, &watch_dates, &key_dates, detail.max_best_days)
    } else {
        Vec::new()
    };
    let best_windows = if detail.include_best_windows {
        build_period_best_windows(&events, &scan_plan, detail.max_best_windows)
    } else {
        Vec::new()
    };
    let watch_windows = if detail.include_watch_windows {
        build_period_watch_windows(&events, &scan_plan, &best_windows, detail.max_watch_windows)
    } else {
        Vec::new()
    };
    let watch_summary_plan = build_period_watch_summary_plan(
        &watch_days,
        is_premium_period_service(service_code),
        &watch_windows,
    );
    let strategy = if detail.include_strategy_section {
        json!({
            "title": "Stratégie de semaine",
            "focus": "Lire d'abord le mouvement général, puis le détail de chaque journée, puis utiliser les fenêtres horaires comme repères pratiques sans ajouter de nouvelles dates dans les conseils.",
            "best_use": "Réserver les fenêtres favorables déjà listées aux échanges, décisions et actions concrètes.",
            "recovery": "Après les fenêtres de vigilance déjà listées, revenir au fil général avant de relancer un sujet.",
            "evidence_keys": evidence.iter().take(4).filter_map(|item| item["evidence_key"].as_str()).collect::<Vec<_>>()
        })
    } else {
        Value::Null
    };
    let editorial_brief = if is_premium_period_service(service_code) {
        build_period_editorial_brief(&daily_plans, &key_days, &best_days, &watch_days)
    } else {
        Value::Null
    };
    let mut request = json!({
        "contract_version": "horoscope_period_interpretation_request_v1",
        "service_code": service_code,
        "period_resolution": period_resolution,
        "scan_plan": scan_plan,
        "target_language": public.target_language,
        "detail_profile_code": detail_profile_code,
        "week_overview_plan": {
            "dominant_theme": events.first().and_then(|event| event["theme_code"].as_str()).unwrap_or("weekly_focus"),
            "tone": events.first().and_then(|event| event["tone"].as_str()).unwrap_or("constructive"),
            "trajectory_hint": "Construire une lecture coherente sur la periode, pas sept lectures quotidiennes independantes.",
            "evidence_keys": evidence.iter().take(4).filter_map(|item| item["evidence_key"].as_str()).collect::<Vec<_>>()
        },
        "period_events": events.clone(),
        "main_events": events.iter().take(detail.max_main_events).cloned().collect::<Vec<_>>(),
        "key_days": key_days,
        "best_days": best_days,
        "watch_days": watch_days,
        "watch_summary_plan": watch_summary_plan,
        "daily_plans": daily_plans,
        "domain_sections": if detail.include_domain_sections { build_period_domain_sections(&evidence, detail.max_domain_sections) } else { Vec::new() },
        "evidence": evidence
    });
    if !editorial_brief.is_null() {
        request["editorial_brief"] = editorial_brief;
    }
    if detail.include_best_windows
        || detail.include_watch_windows
        || detail.include_strategy_section
    {
        request["best_windows"] = json!(best_windows);
        request["watch_windows"] = json!(watch_windows);
        request["strategy"] = strategy;
        request["premium_scores"] = json!(build_period_premium_scores(&request));
    }
    validate_period_interpretation_request_schema(&request)?;
    Ok(request)
}

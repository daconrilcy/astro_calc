use super::*;

pub(crate) fn repair_free_period_response_shape(request: &Value, response: &mut Value) {
    response["summary"] = sanitize_free_period_summary(response.get("summary"));
    response["dominant_theme"] =
        sanitize_free_period_dominant_theme(response.get("dominant_theme"), request);
    response["key_days"] = sanitize_period_markers(
        response.get("key_days"),
        &request["key_days"],
        PeriodMarkerRole::Key,
    );
    if response["key_days"].as_array().map(Vec::len).unwrap_or(0) == 0 {
        let first = request["evidence"]
            .as_array()
            .and_then(|items| items.first());
        response["key_days"] = json!([{            "date": first.and_then(|item| item["date"].as_str()).unwrap_or("2026-06-07"),            "title": "Jour à retenir",            "reason": "Un repère utile ressort pour organiser la semaine sans détailler chaque journée.",            "evidence_keys": first.and_then(|item| item["evidence_key"].as_str()).map(|key| vec![key]).unwrap_or_default(),            "fallback_reason": null        }]);
    }
    if response["key_days"].as_array().map(Vec::len).unwrap_or(0) > 2 {
        response["key_days"] = json!(response["key_days"]
            .as_array()
            .unwrap()
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>());
    }
    if let Some(days) = response["key_days"].as_array_mut() {
        for day in days {
            day["title"] = json!("Jour à retenir");
        }
    }
    response["advice"] = json!(sanitize_period_public_string(
        response
            .get("advice")
            .and_then(Value::as_str)
            .unwrap_or("Choisissez une priorité simple et gardez une marge d'ajustement.")
    ));
    response["watch_summary"] =
        sanitize_free_period_watch_summary(response.get("watch_summary"), request);
    response["evidence_summary"] =
        sanitize_period_evidence_summary(response.get("evidence_summary"), request);
    if response["evidence_summary"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        > 3
    {
        response["evidence_summary"] = json!(response["evidence_summary"]
            .as_array()
            .unwrap()
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>());
    }
    response.as_object_mut().map(|map| {
        for field in [
            "week_overview",
            "best_days",
            "watch_days",
            "daily_timeline",
            "domain_sections",
            "best_windows",
            "watch_windows",
            "strategy",
        ] {
            map.remove(field);
        }
    });
    let provider = response["quality"]["provider"]
        .as_str()
        .unwrap_or("openai")
        .to_string();
    let model = response["quality"]["model"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let fallback_used = response["quality"]["fallback_used"]
        .as_bool()
        .unwrap_or(false);
    response["quality"] = json!({        "daily_timeline_count": 0,        "evidence_guard_passed": true,        "best_watch_overlap_passed": true,        "provider": provider,        "model": model,        "fallback_used": fallback_used,        "period_contract": "free_next_7_days"    });
}
pub(crate) fn sanitize_free_period_summary(value: Option<&Value>) -> Value {
    json!({        "title": sanitize_period_public_string(value.and_then(|v| v.get("title")).and_then(Value::as_str).unwrap_or("Vos 7 prochains jours")),        "text": sanitize_period_public_string(value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("Les prochains jours donnent une tendance à comprendre plutôt qu'un planning détaillé. Repérez le thème qui revient, choisissez une priorité simple et laissez de la place pour ajuster votre rythme sans chercher à tout décider maintenant."))    })
}
pub(crate) fn sanitize_free_period_dominant_theme(value: Option<&Value>, request: &Value) -> Value {
    let fallback_theme = request["week_overview_plan"]["dominant_theme"]
        .as_str()
        .map(period_theme_public_label)
        .unwrap_or("organisation");
    json!({        "theme": sanitize_period_public_string(value.and_then(|v| v.get("theme")).and_then(Value::as_str).unwrap_or(fallback_theme)),        "text": sanitize_period_public_string(value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("Ce thème donne le relief principal de la semaine et aide à choisir une action concrète sans ouvrir trop de sujets."))    })
}
pub(crate) fn sanitize_free_period_watch_summary(value: Option<&Value>, request: &Value) -> Value {
    let allowed_keys = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["evidence_key"].as_str())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let allowed = allowed_keys
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let generated_status = value
        .and_then(|item| item.get("status"))
        .and_then(Value::as_str)
        .filter(|status| matches!(*status, "none" | "low" | "present"));
    let status = generated_status.unwrap_or("none");
    let fallback_text = if status == "none" {
        FREE_PERIOD_NONE_WATCH_SUMMARY
    } else {
        "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation."
    };
    let text = sanitize_period_public_string(
        value
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(fallback_text),
    );
    let mut evidence_keys = value
        .and_then(|item| item.get("evidence_keys"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|key| allowed.contains(*key))
        .map(|key| json!(key))
        .collect::<Vec<_>>();
    if status != "none" && evidence_keys.is_empty() {
        if let Some(first) = allowed_keys.first() {
            evidence_keys.push(json!(first));
        }
    }
    if status == "none" {
        evidence_keys.clear();
    }
    json!({        "status": status,        "text": text,        "evidence_keys": evidence_keys    })
}
pub(crate) fn is_premium_period_service(service_code: &str) -> bool {
    service_code == HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
}
pub(crate) fn is_free_period_service(service_code: &str) -> bool {
    service_code == HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
}
pub(crate) fn sanitize_period_week_overview(value: Option<&Value>) -> Value {
    let text = value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("La période se lit comme une progression continue, avec des jours d'appui, des ajustements concrets et une consolidation finale.");
    let trajectory = value
        .and_then(|v| v.get("trajectory"))
        .and_then(Value::as_str)
        .unwrap_or("Clarifier, ajuster, puis consolider.");
    let trajectory = normalize_period_trajectory_text(trajectory);
    json!({        "title": sanitize_period_public_string(value.and_then(|v| v.get("title")).and_then(Value::as_str).unwrap_or("Vue d'ensemble")),        "text": sanitize_period_public_string(&ensure_period_explicit_personalization_text(text, "La semaine se pilote avec vos priorités concrètes : qui fait quoi, pour quand, avec quelle preuve.")),        "trajectory": sanitize_period_public_string(&ensure_period_personalization_text(trajectory, "Le mouvement part d'un appui pratique, passe par une vérification des engagements, puis se termine par une validation plus claire des rôles."))    })
}
pub(crate) fn normalize_period_trajectory_text(text: &str) -> &str {
    let lower = text.to_lowercase();
    if lower.contains("le mouvement relie vos repères personnels")
        || lower.contains("les appuis émotionnels et les choix à consolider")
        || lower.contains("zones personnelles")
    {
        "Le mouvement va d'une sécurisation pratique vers une vérification des engagements, puis vers une validation plus collective si les rôles sont clairs."
    } else {
        text
    }
}
pub(crate) fn sanitize_period_advice(value: Option<&Value>) -> Value {
    json!({        "main": sanitize_period_public_string(value.and_then(|v| v.get("main")).and_then(Value::as_str).unwrap_or("Gardez une progression simple et reliez les décisions d'un jour à l'autre.")),        "best_use": sanitize_period_public_string(value.and_then(|v| v.get("best_use")).and_then(Value::as_str).unwrap_or("Utiliser les appuis de la semaine pour organiser, dialoguer et consolider.")),        "avoid": sanitize_period_public_string(value.and_then(|v| v.get("avoid")).and_then(Value::as_str).unwrap_or("Éviter de transformer un signal quotidien en certitude définitive."))    })
}
pub(crate) fn sanitize_period_watch_summary(value: Option<&Value>, fallback: &Value) -> Value {
    let status = fallback
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let fallback_text = fallback
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or(FREE_PERIOD_NONE_WATCH_SUMMARY);
    json!({        "status": status,        "text": sanitize_period_public_string(value            .and_then(|item| item.get("text"))            .and_then(Value::as_str)            .filter(|text| !text.trim().is_empty())            .unwrap_or(fallback_text)),        "evidence_keys": string_array_value(fallback.get("evidence_keys")).unwrap_or_else(|| json!([]))    })
}
pub(crate) fn sanitize_period_markers(
    value: Option<&Value>,
    fallback: &Value,
    role: PeriodMarkerRole,
) -> Value {
    let generated_items = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let generated_by_date = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item.clone())))
        .collect::<HashMap<_, _>>();
    let source = fallback.as_array().cloned().unwrap_or_else(Vec::new);
    Value::Array(        source            .into_iter()            .enumerate()            .map(|(index, fallback_item)| {                let date = fallback_item                    .get("date")                    .and_then(Value::as_str)                    .unwrap_or("");                let generated_item = generated_by_date                    .get(date)                    .or_else(|| generated_items.get(index));                let fallback_reason = generated_item                    .and_then(|item| item.get("fallback_reason"))                    .filter(|value| !value.is_null())                    .and_then(Value::as_str)                    .filter(|reason| !reason.trim().is_empty())                    .or_else(|| {                        fallback_item                            .get("fallback_reason")                            .filter(|value| !value.is_null())                            .and_then(Value::as_str)                            .filter(|reason| !reason.trim().is_empty())                    })                    .map_or(Value::Null, |reason| json!(reason));                let fallback_reason_text = fallback_item                    .get("reason")                    .and_then(Value::as_str)                    .filter(|reason| !reason.trim().is_empty())                    .filter(|reason| !period_marker_reason_is_suspect_for_role(reason, role));                let reason = generated_item                    .and_then(|item| item.get("reason"))                    .and_then(Value::as_str)                    .filter(|reason| !reason.trim().is_empty())                    .filter(|reason| !period_marker_reason_is_suspect_for_role(reason, role))                    .or(fallback_reason_text)                    .map(ToOwned::to_owned)                    .unwrap_or_else(|| naturalized_period_marker_fallback_reason(&fallback_item));                json!({                    "date": fallback_item["date"],                    "title": sanitize_period_public_string(                        generated_item                            .and_then(|item| item.get("title"))                            .and_then(Value::as_str)                            .or_else(|| fallback_item.get("title").and_then(Value::as_str))                            .unwrap_or("Jour")                    ),                    "reason": sanitize_period_public_string(&reason),                    "evidence_keys": non_empty_string_array_value(fallback_item.get("evidence_keys")).unwrap_or_else(|| json!([])),                    "fallback_reason": fallback_reason                })            })            .collect(),    )
}
pub(crate) fn naturalized_period_marker_fallback_reason(marker: &Value) -> String {
    let title = marker
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    let date_label = marker
        .get("date")
        .and_then(Value::as_str)
        .map(public_day_label)
        .unwrap_or_else(|| "Ce jour".to_string());
    if title.contains("vigilance") {
        format!(            "{date_label} demande de ralentir avant d'engager une réponse, une promesse ou une décision."        )
    } else if title.contains("favorable") || title.contains("appui") || title.contains("meilleur") {
        format!(            "{date_label} aide à sécuriser une ressource, une preuve ou une tâche pratique sans rouvrir tout le dossier."        )
    } else {
        format!(            "{date_label} donne un repère concret pour ajuster une priorité sans isoler la journée du reste de la période."        )
    }
}
pub(crate) fn period_marker_reason_is_suspect(reason: &str) -> bool {
    let lower = reason.to_lowercase();
    lower.contains("autour de vérifier")
        || lower.contains("autour de attendre")
        || lower.contains(": appuis concrets aide")
        || lower.contains("appui concret :")
        || lower.contains("est un point d'appui pour")
        || lower.contains("demande de ralentir sur")
        || lower.contains("la journée dynamique un premier frottement")
        || lower.contains("la même priorité revint")
        || lower.contains("stabiliser tester limites agir par gestes courts")
        || lower.contains("dans échanges à cadrer, le plus utile")
        || lower.contains("dans cap à mettre au net, le plus utile")
        || lower.contains("dans énergie mentale, le plus utile")
        || lower.contains(". .")
}
pub(crate) fn period_marker_reason_is_suspect_for_role(
    reason: &str,
    role: PeriodMarkerRole,
) -> bool {
    if period_marker_reason_is_suspect(reason) {
        return true;
    }
    matches!(role, PeriodMarkerRole::Best)
        && reason
            .to_lowercase()
            .contains("avant de promettre davantage")
}
pub(crate) fn sanitize_period_daily_timeline(value: Option<&Value>, request: &Value) -> Value {
    let by_date = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|day| Some((day.get("date")?.as_str()?.to_string(), day.clone())))
        .collect::<HashMap<_, _>>();
    let days = request["daily_plans"]        .as_array()        .into_iter()        .flatten()        .map(|plan| {            let date = plan.get("date").and_then(Value::as_str).unwrap_or("");            let generated = by_date.get(date);            let theme = plan                .get("theme_label")                .and_then(Value::as_str)                .unwrap_or("priorité");            let fallback_text = period_public_day_text(plan, 0);            let fallback_advice = period_public_day_advice(plan);            json!({                "date": date,                "day_label": sanitize_period_public_string(generated.and_then(|day| day.get("day_label")).and_then(Value::as_str).or_else(|| plan.get("day_label").and_then(Value::as_str)).unwrap_or("Jour")),                "theme": sanitize_period_public_string(theme),                "tone": generated.and_then(|day| day.get("tone")).and_then(Value::as_str).unwrap_or("concentré"),                "text": sanitize_period_public_string(&generated.and_then(|day| day.get("text")).and_then(Value::as_str).map(|text| ensure_period_personalization_text(text, &period_public_interpretive_sentence(plan))).unwrap_or(fallback_text)),                "advice": sanitize_period_public_string(generated.and_then(|day| day.get("advice")).and_then(Value::as_str).unwrap_or(&fallback_advice)),                "evidence_keys": string_array_value(plan.get("evidence_keys")).unwrap_or_else(|| json!([]))            })        })        .collect::<Vec<_>>();
    Value::Array(days)
}
pub(crate) fn sanitize_period_domain_sections(value: Option<&Value>, request: &Value) -> Value {
    let generated = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let fallback_sections = request["domain_sections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fallback_by_domain = fallback_sections
        .iter()
        .filter_map(|section| {
            let domain = section.get("domain")?.as_str()?.to_string();
            Some((domain, section.clone()))
        })
        .collect::<HashMap<_, _>>();
    let fallback_by_title = fallback_sections
        .iter()
        .filter_map(|section| {
            let title = normalized_text(section.get("title")?.as_str()?);
            Some((title, section.clone()))
        })
        .collect::<HashMap<_, _>>();
    let fallback = fallback_sections        .iter()        .map(|section| {            json!({                "domain": section["domain"],                "title": section["title"],                "text": period_public_domain_text(section),                "evidence_keys": section["evidence_keys"]            })        })        .collect::<Vec<_>>();
    let source = if generated.is_empty() {
        fallback
    } else {
        generated
    };
    Value::Array(        source            .into_iter()            .enumerate()            .map(|(index, section)| {                let fallback = section                    .get("domain")                    .and_then(Value::as_str)                    .and_then(|domain| fallback_by_domain.get(domain))                    .or_else(|| {                        section                            .get("title")                            .and_then(Value::as_str)                            .and_then(|title| fallback_by_title.get(&normalized_text(title)))                    })                    .or_else(|| fallback_sections.get(index));                json!({                    "domain": sanitize_period_public_string(                        fallback                            .and_then(|item| item.get("domain"))                            .and_then(Value::as_str)                            .or_else(|| section.get("domain").and_then(Value::as_str))                            .unwrap_or("organisation")                    ),                    "title": sanitize_period_public_string(                        section                            .get("title")                            .and_then(Value::as_str)                            .or_else(|| fallback.and_then(|item| item.get("title")).and_then(Value::as_str))                            .unwrap_or("Organisation")                    ),                    "text": sanitize_period_public_string(                        &rewrite_period_domain_template_text(                            &section                            .get("text")                            .and_then(Value::as_str)                            .map(|text| ensure_period_explicit_personalization_text(text, &period_public_domain_interpretive_sentence(fallback.unwrap_or(&section))))                            .unwrap_or_else(|| fallback.map(period_public_domain_text).unwrap_or_else(|| period_public_domain_text(&section))),                            fallback.unwrap_or(&section),                        )                    ),                    "evidence_keys": fallback                        .and_then(|fallback| non_empty_string_array_value(fallback.get("evidence_keys")))                        .unwrap_or_else(|| json!([]))                })            })            .collect(),    )
}
pub(crate) fn sanitize_period_windows(
    value: Option<&Value>,
    request: &Value,
    field: &str,
) -> Value {
    let allowed = request[field].as_array().cloned().unwrap_or_default();
    let allowed_by_key = allowed
        .iter()
        .filter_map(|window| {
            let key = period_window_identity(window)?;
            Some((key, window.clone()))
        })
        .collect::<HashMap<_, _>>();
    let generated = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for window in generated {
        let Some(identity) = period_window_identity(&window) else {
            continue;
        };
        let Some(fallback) = allowed_by_key.get(&identity) else {
            continue;
        };
        out.push(sanitize_period_window_from_fallback(
            &window, fallback, field,
        ));
    }
    Value::Array(out)
}
pub(crate) fn period_window_identity(window: &Value) -> Option<String> {
    let date = window.get("date")?.as_str()?;
    let keys = window
        .get("source_snapshot_keys")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("|");
    if keys.is_empty() {
        return None;
    }
    Some(format!("{date}:{keys}"))
}
pub(crate) fn sanitize_period_window_from_fallback(
    window: &Value,
    fallback: &Value,
    field: &str,
) -> Value {
    let mut out = json!({        "date": fallback["date"],        "time_range_label": sanitize_period_public_string(window.get("time_range_label").and_then(Value::as_str).or_else(|| fallback.get("time_range_label").and_then(Value::as_str)).unwrap_or("")),        "source_snapshot_keys": fallback["source_snapshot_keys"],        "title": sanitize_period_public_string(window.get("title").and_then(Value::as_str).or_else(|| fallback.get("title").and_then(Value::as_str)).unwrap_or("Fenêtre")),        "theme": sanitize_period_public_string(window.get("theme").and_then(Value::as_str).or_else(|| fallback.get("theme").and_then(Value::as_str)).unwrap_or("priorité")),        "tone": sanitize_period_public_string(window.get("tone").and_then(Value::as_str).or_else(|| fallback.get("tone").and_then(Value::as_str)).unwrap_or("nuancé")),        "evidence_keys": fallback["evidence_keys"]    });
    if field == "best_windows" {
        let generated_reason = window
            .get("reason")
            .and_then(Value::as_str)
            .filter(|reason| !period_best_window_reason_is_generic(reason));
        out["reason"] = json!(sanitize_period_public_string(
            generated_reason
                .or_else(|| fallback.get("reason").and_then(Value::as_str))
                .unwrap_or("Ce créneau aide à poser une action simple et vérifiable.")
        ));
        out["best_for"] = fallback["best_for"].clone();
    } else {
        out["watch_point"] = json!(sanitize_period_public_string(
            window
                .get("watch_point")
                .and_then(Value::as_str)
                .or_else(|| fallback.get("watch_point").and_then(Value::as_str))
                .unwrap_or("Garder une marge avant de répondre.")
        ));
    }
    out
}
pub(crate) fn period_best_window_reason_is_generic(reason: &str) -> bool {
    let lower = reason.to_lowercase();
    [
        "ce créneau peut servir à poser une action simple et vérifiable",
        "ce créneau se prête à un échange plus simple et mieux ajusté",
        "ce créneau aide à transformer l'élan en action courte",
        "ce créneau favorise une formulation plus nette",
        "ce créneau aide à trier et décider sans disperser l'attention",
        "ce créneau aide à consolider ce qui a déjà été compris",
    ]
    .iter()
    .any(|fragment| lower.contains(fragment))
}
pub(crate) fn sanitize_period_strategy(value: Option<&Value>, request: &Value) -> Value {
    let fallback = &request["strategy"];
    json!({        "title": sanitize_period_public_string(value.and_then(|v| v.get("title")).and_then(Value::as_str).or_else(|| fallback.get("title").and_then(Value::as_str)).unwrap_or("Stratégie de semaine")),        "text": sanitize_period_public_string(value.and_then(|v| v.get("text")).and_then(Value::as_str).unwrap_or("Alterner les fenêtres favorables pour agir, les moments de vigilance pour ralentir et les temps d'intégration pour consolider les choix.")),        "best_use": sanitize_period_public_string(value.and_then(|v| v.get("best_use")).and_then(Value::as_str).or_else(|| fallback.get("best_use").and_then(Value::as_str)).unwrap_or("Utiliser les appuis pour décider et communiquer simplement.")),        "recovery": sanitize_period_public_string(value.and_then(|v| v.get("recovery")).and_then(Value::as_str).or_else(|| fallback.get("recovery").and_then(Value::as_str)).unwrap_or("Préserver un temps de recul après les moments plus réactifs.")),        "evidence_keys": string_array_value(fallback.get("evidence_keys")).unwrap_or_else(|| json!([]))    })
}
pub(crate) fn ensure_period_personalization_text(text: &str, personalization: &str) -> String {
    let base = sanitize_period_public_string(text);
    if period_text_has_personalization(&base) {
        base
    } else {
        format!("{base} {personalization}")
    }
}
pub(crate) fn ensure_period_explicit_personalization_text(
    text: &str,
    personalization: &str,
) -> String {
    let base = sanitize_period_public_string(text);
    if period_text_has_explicit_personal_anchor(&base) {
        base
    } else {
        format!("{base} {personalization}")
    }
}
pub(crate) fn period_text_has_explicit_personal_anchor(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("vos priorités")
        || lower.contains("vos priorites")
        || lower.contains("votre agenda")
        || lower.contains("qui fait quoi")
        || lower.contains("quelle preuve")
}
pub(crate) fn period_public_day_text(day: &Value, index: usize) -> String {
    let day_label = day
        .get("day_label")
        .and_then(Value::as_str)
        .unwrap_or("Ce jour");
    let theme = day
        .get("theme_label")
        .and_then(Value::as_str)
        .or_else(|| day.get("theme").and_then(Value::as_str))
        .unwrap_or("priorité");
    let focus = period_public_focus_text(day);
    let focus_sentence = period_daily_focus_sentence(&focus);
    match period_style_code(day) {        "relation" => format!(            "{day_label} adoucit le thème {theme}. {focus_sentence} Une attente ou une parole simple peut détendre l'échange sans chercher un accord de façade."        ),        "action" => format!(            "{day_label} donne du relief au thème {theme}. {focus_sentence} Une action courte vaut mieux qu'une série de réponses dispersées."        ),        "clarity" => format!(            "{day_label} aide à nommer ce qui compte dans le thème {theme}. {focus_sentence} Le tri devient plus simple et les choix gagnent en lisibilité."        ),        "communication" => format!(            "{day_label} remet le thème {theme} dans les mots justes. {focus_sentence} Une formulation directe peut éviter plusieurs malentendus."        ),        "integration" => format!(            "{day_label} invite à relier le thème {theme} à ce qui a déjà été compris. {focus_sentence} Il devient plus simple de consolider sans ouvrir trop de nouveaux fronts."        ),        _ => match index {            0 => format!(                "{day_label} ouvre la période sur le thème {theme}. {focus_sentence} Il s'agit surtout de remettre de l'ordre dans ce qui circule déjà, sans tout contrôler."            ),            5 => format!(                "{day_label} ramène le thème {theme} vers une priorité réaliste. {focus_sentence} Il devient plus facile de choisir ce qui mérite d'être tenu jusqu'au bout."            ),            _ => format!(                "{day_label} recentre le thème {theme}. {focus_sentence} Le plus utile consiste à poser un repère clair avant d'élargir le mouvement."            ),        },    }
}
pub(crate) fn period_public_day_advice(day: &Value) -> String {
    let focus = period_public_focus_text(day);
    let parts = period_focus_parts(&focus, 2);
    match period_style_code(day) {        "relation" => format!(            "Privilégiez un geste relationnel simple. {} N'essayez pas de traiter tous les sujets.",            naturalize_period_focus(&focus)        ),        "action" => format!("Transformez cette priorité en une action vérifiable, puis laissez le reste en attente."),        "clarity" => "Nommez ce qui compte vraiment, puis gardez une décision progressive et vérifiable.".to_string(),        "communication" => format!("Formulez une demande courte et vérifiable, puis écoutez la réponse sans surinterpréter."),        "integration" => format!("Reliez ce travail d'intégration à une habitude déjà solide et consolidez-la avant d'ajouter autre chose."),        _ => match parts.as_slice() {            [one] => format!("Commencez par {one}, puis avancez par un geste mesuré."),            [one, two, ..] => {                format!("Commencez par {one}, puis limitez la suite à {two}.")            }            [] => "Posez une priorité claire, puis avancez par un geste mesuré.".to_string(),        },    }
}
pub(crate) fn period_daily_advice_expansion(index: usize) -> &'static str {
    match index % 7 {
        0 => "Gardez un geste simple et retenez une suite concrète.",
        1 => "Avancez par une décision courte, puis laissez le rythme se stabiliser.",
        2 => "Choisissez un repère utile et vérifiez-le avant d'élargir l'action.",
        3 => "Préservez une marge de recul avant de répondre trop vite.",
        4 => "Transformez l'élan du jour en action mesurable et limitée.",
        5 => "Revenez à ce qui peut vraiment être tenu jusqu'au lendemain.",
        _ => "Laissez la journée fermer une étape avant d'en ouvrir une autre.",
    }
}
pub(crate) fn period_public_domain_text(section: &Value) -> String {
    let domain = section
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| section.get("domain").and_then(Value::as_str))
        .unwrap_or("Ce domaine");
    let focus = period_clean_focus_fragment(&period_public_focus_text(section));
    let focus_sentence = period_domain_focus_sentence_for_domain(&focus, domain);
    format!("{domain} ouvre un fil pratique de la semaine. {focus_sentence} Gardez ce point de tri concret, pas comme une obligation de plus.")
}
pub(crate) fn period_public_personalization_sentence(item: &Value) -> String {
    period_public_interpretive_sentence(item)
}
pub(crate) fn period_public_day_personalization_sentence(item: &Value, index: usize) -> String {
    let focus = period_focus_parts(&period_public_focus_text(item), 2);
    match (index % 4, focus.as_slice()) {
        (0, [one, ..]) => format!(
            "Gardez un critère simple pour vous : {one}, puis une preuve concrète avant d'élargir."
        ),
        (1, [one, two, ..]) => {
            format!("Votre agenda gagne à séparer {one} de {two}, sans tout traiter le même jour.")
        }
        (2, [one, ..]) => {
            format!("La bonne mesure consiste à relier {one} à qui fait quoi, pour quand.")
        }
        (3, [one, ..]) => {
            format!("Pour vous, l'appui utile reste {one}, puis une suite courte à confirmer.")
        }
        _ => period_public_interpretive_sentence(item),
    }
}
pub(crate) fn period_public_interpretive_sentence(_item: &Value) -> String {
    "Gardez le critère le plus simple : qui fait quoi, pour quand, avec quelle preuve.".to_string()
}
pub(crate) fn period_public_domain_personalization_sentence(item: &Value) -> String {
    period_public_domain_interpretive_sentence(item)
}
pub(crate) fn period_public_domain_interpretive_sentence(item: &Value) -> String {
    let focus = period_clean_focus_fragment(&period_public_focus_text(item));
    let domain = item
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| item.get("domain").and_then(Value::as_str))
        .unwrap_or("Ce domaine");
    let focus_sentence = period_domain_focus_sentence_for_domain(&focus, domain);
    format!(        "{} {focus_sentence} Gardez un geste simple à poser, une limite à nommer et une suite concrète à tenir sans charger le reste.",        period_domain_personalization_intro(domain)    )
}
pub(crate) fn period_public_domain_personalization_tail(item: &Value) -> String {
    let domain = item
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| item.get("domain").and_then(Value::as_str))
        .unwrap_or("Ce domaine");
    format!(
        "{} Gardez une limite à nommer et une suite concrète à tenir sans charger le reste.",
        period_domain_personalization_intro(domain)
    )
}
pub(crate) fn period_domain_personalization_intro(domain: &str) -> String {
    match period_text_variant_index(domain, 4) {
        0 => "L'enjeu est de choisir ce qui mérite vraiment votre attention cette semaine."
            .to_string(),
        1 => "Votre avantage vient d'un échange court, cadré, puis refermé au bon moment."
            .to_string(),
        2 => {
            "L'enjeu est de distinguer ce qui peut avancer maintenant de ce qui doit rester léger."
                .to_string()
        }
        _ => "Le plus solide reste un geste bref, vérifiable et orienté vers une décision simple."
            .to_string(),
    }
}
pub(crate) fn period_text_variant_index(text: &str, modulo: usize) -> usize {
    if modulo == 0 {
        return 0;
    }
    text.bytes()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize))
        % modulo
}
pub(crate) fn period_daily_focus_sentence(focus: &str) -> String {
    let parts = period_focus_parts(focus, 2);
    match parts.as_slice() {
        [] => "Le geste utile reste simple et vérifiable.".to_string(),
        [one] => format!("Le geste utile consiste à {one}."),
        [one, two, ..] => format!("Le geste utile consiste à {one}, puis à {two}."),
    }
}
pub(crate) fn period_clean_focus_fragment(focus: &str) -> String {
    focus
        .trim()
        .trim_end_matches(|ch: char| ch == '.' || ch == ';' || ch == ',' || ch.is_whitespace())
        .trim()
        .to_string()
}
pub(crate) fn period_domain_focus_sentence_for_domain(focus: &str, domain: &str) -> String {
    period_domain_focus_sentence_variant(focus, period_text_variant_index(domain, 4))
}
pub(crate) fn period_domain_focus_sentence_variant(focus: &str, variant: usize) -> String {
    let parts = period_focus_parts(focus, 2);
    match parts.as_slice() {
        [] => "Un repère simple et vérifiable suffit à orienter les choix.".to_string(),
        [one] => match variant % 4 {
            0 => format!("Le plus concret est {}.", period_de_action(one)),
            1 => format!("Le bon appui est {}.", period_de_action(one)),
            2 => format!("Le geste à garder est {}.", period_de_action(one)),
            _ => format!("La bonne mesure reste {}.", period_de_action(one)),
        },
        [one, two, ..] => match variant % 4 {
            0 => format!(
                "Le plus concret est {}, puis {}.",
                period_de_action(one),
                period_de_action(two)
            ),
            1 => format!(
                "Le bon appui est {}, puis {}.",
                period_de_action(one),
                period_de_action(two)
            ),
            2 => format!(
                "Le geste à garder est {}, puis {}.",
                period_de_action(one),
                period_de_action(two)
            ),
            _ => format!(
                "La bonne mesure reste {}, puis {}.",
                period_de_action(one),
                period_de_action(two)
            ),
        },
    }
}
pub(crate) fn period_de_action(action: &str) -> String {
    let trimmed = action.trim();
    if trimmed.is_empty() {
        return "de choisir un repère simple".to_string();
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("de ") || lower.starts_with("d'") || lower.starts_with("d’") {
        return trimmed.to_string();
    }
    if let Some(rest) = trimmed
        .strip_prefix("à ")
        .or_else(|| trimmed.strip_prefix("À "))
        .filter(|rest| !rest.trim().is_empty())
    {
        return period_de_action(rest);
    }
    let first = trimmed
        .chars()
        .next()
        .map(|ch| ch.to_lowercase().to_string())
        .unwrap_or_default();
    if matches!(
        first.as_str(),
        "a" | "à"
            | "â"
            | "e"
            | "é"
            | "è"
            | "ê"
            | "ë"
            | "i"
            | "î"
            | "ï"
            | "o"
            | "ô"
            | "u"
            | "ù"
            | "û"
            | "ü"
            | "y"
            | "h"
    ) {
        format!("d'{trimmed}")
    } else {
        format!("de {trimmed}")
    }
}
pub(crate) fn period_focus_parts(focus: &str, limit: usize) -> Vec<String> {
    period_clean_focus_fragment(focus)
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .take(limit)
        .map(ToOwned::to_owned)
        .collect()
}
pub(crate) fn rewrite_period_domain_template_text(text: &str, fallback: &Value) -> String {
    if period_domain_template_fragment(text).is_some() {
        period_public_domain_text(fallback)
    } else {
        text.to_string()
    }
}
pub(crate) fn period_domain_template_fragment(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    for fragment in [
        "dans ce domaine, les repères les plus utiles consistent",
        "et à choisir le bon niveau d'engagement",
        "dans ce domaine, vos repères personnels liés à",
    ] {
        if lower.contains(fragment) {
            return Some(fragment);
        }
    }
    if period_domain_public_template_re().is_match(text) {
        return Some("dans x, le plus utile est");
    }
    None
}
pub(crate) fn period_domain_public_template_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\bDans\s+[^.,;:!?]{2,60},\s*Le\s+plus\s+utile\s+est\b")
            .expect("domain public template regex")
    })
}
pub(crate) fn period_style_code(item: &Value) -> &str {
    item.get("style_variant_code")
        .and_then(Value::as_str)
        .unwrap_or_else(|| match item.get("theme_code").and_then(Value::as_str) {
            Some("relationship") => "relation",
            Some("energy") => "action",
            Some("clarity") => "clarity",
            Some("communication") => "communication",
            Some("integration") => "integration",
            _ => "anchor",
        })
}
pub(crate) fn period_public_focus_text(item: &Value) -> String {
    for key in [
        "personalization_hint",
        "natal_focus_label",
        "natal_focus_hint",
        "la journée gagne un repère",
        "sans devenir une explication abstraite",
        "ce signal",
    ] {
        if let Some(raw) = item.get(key).and_then(Value::as_str) {
            let cleaned = period_public_focus_from_hint(raw);
            if !cleaned.trim().is_empty() {
                return cleaned;
            }
        }
    }
    "une priorité concrète".to_string()
}
pub(crate) fn period_public_focus_from_hint(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    for prefix in [
        "Personnaliser ce signal par ",
        "Personnaliser ce signal avec ",
        "Relier ce signal à ",
        "Relier ce signal aux ",
        "Relier ce signal au ",
        "Relier ce domaine à ",
        "Situations associées : ",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.to_string();
            break;
        }
    }
    for suffix in [
        " plutôt que rester sur un conseil générique.",
        " plutôt que rester sur un conseil générique",
        ", sans jargon technique.",
        " sans jargon technique.",
    ] {
        if let Some(rest) = text.strip_suffix(suffix) {
            text = rest.to_string();
        }
    }
    text
}
pub(crate) fn sanitize_period_public_string(text: &str) -> String {
    let reprocessed = reprocess_horoscope_period("fr", json!(text), None)
        .payload
        .as_str()
        .unwrap_or(text)
        .to_string();
    let (reprocessed, _) = restore_french_glued_compounds(&reprocessed);
    let repaired = repair_period_truncated_public_tail(&reprocessed);
    repair_period_mechanical_public_fragments(&repaired)
}
pub(crate) fn repair_period_mechanical_public_fragments(text: &str) -> String {
    let mut repaired = text.to_string();
    for (pattern, replacement) in period_mechanical_public_fragment_replacements() {
        repaired = pattern.replace_all(&repaired, *replacement).into_owned();
    }
    repaired
}
pub(crate) fn period_mechanical_public_fragment_replacements() -> &'static [(Regex, &'static str)] {
    static REPLACEMENTS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REPLACEMENTS        .get_or_init(|| {            [                (r"(?i)\bautour\s+de\s+vérifier\b", "pour vérifier"),                (r"(?i)\bautour\s+d['’]attendre\b", "avant d'attendre"),                (r"(?i)\bautour\s+de\s+attendre\b", "avant d'attendre"),                (                    r"(?i):\s*appuis\s+concrets\s+aide\b",                    " : cet appui aide",                ),                (r"(?i)\bappui\s+concret\s*:", "Point d'appui :"),                (                    r"(?i)\best\s+un\s+point\s+d['’]appui\s+pour\s+appuis\s+concrets\b",                    "aide à sécuriser un appui concret",                ),                (                    r"(?i)\best\s+un\s+point\s+d['’]appui\s+pour\s+liens,\s*valeur\s+et\s+attachement\b",                    "aide à clarifier un lien personnel",                ),                (                    r"(?i)\best\s+un\s+point\s+d['’]appui\s+pour\s+énergie\s+mentale\b",                    "aide à cadrer l'énergie mentale",                ),                (                    r"(?i)\best\s+un\s+point\s+d['’]appui\s+pour\s+engagements\s+et\s+limites\b",                    "aide à vérifier les engagements et les limites",                ),                (                    r"(?i)\best\s+un\s+point\s+d['’]appui\s+pour\s+échanges\s+à\s+cadrer\b",                    "aide à cadrer les échanges",                ),                (                    r"(?i)\best\s+un\s+point\s+d['’]appui\s+pour\b",                    "aide à clarifier",                ),                (                    r"(?i)\bdemande\s+de\s+ralentir\s+sur\b",                    "demande de ralentir avant",                ),                (                    r"(?i)\bcette\s+énergie\s+devient\s+utile\s+quand\s+elle\s+sert\s+à\b",                    "Ce domaine aide surtout à",                ),                (                    r"(?i)\bla\s+journée\s+dynamique\s+un\s+premier\s+frottement\b",                    "La journée crée un premier frottement",                ),                (                    r"(?i)\b(Soleil|Mars|Mercure|Vénus|Venus|Jupiter|Saturne|Lune)\s+dynamique\b",                    "$1 dynamise",                ),                (                    r"(?i)\bet\s+suspendre\s+la\s+discussion\b",                    "et suspendez la discussion",                ),                (r"(?i)\brevint\b", "revient"),                (                    r"(?i)\bStabiliser\s+Tester\s+limites\s+Agir\s+par\s+gestes\s+courts\b",                    "Le mouvement suit trois étapes : stabiliser, tester les limites, puis agir par gestes courts.",                ),                (                    r"(?i)^Ouvrir par un repère visible trancher et prouver vérifier et réduire les engagements\b",                    "Ouvrir par un repère visible ; trancher et prouver ; vérifier et réduire les engagements.",                ),                (                    r"(?i)Dans\s+Échanges\s+à\s+cadrer,\s*Le\s+plus\s+utile\s+est\s+de\s+choisir\s+une\s+action\s+courte,\s+puis\s+de\s+poser\s+une\s+limite\s+claire\.?",                    "Dans les échanges, le plus efficace consiste à choisir une action courte, puis à poser une limite claire.",                ),                (                    r"(?i)Dans\s+Cap\s+à\s+mettre\s+au\s+net,\s*Le\s+plus\s+utile\s+est\s+de\s+nommer\s+ce\s+qui\s+compte,\s+puis\s+d['’]accorder\s+une\s+attente\s+affective\.?",                    "Pour mettre le cap au net, le plus utile est de nommer ce qui compte, puis d'accorder une attente affective.",                ),                (                    r"(?i)Dans\s+Énergie\s+mentale,\s*Le\s+plus\s+utile\s+est\s+de\s+préparer\s+un\s+message\s+court,\s+puis\s+de\s+différer\s+une\s+réponse\s+rapide\.?",                    "Votre avantage mental vient d'un geste bref : préparer un message court, puis différer une réponse rapide.",                ),                (                    r"(?i)\bLe\s+mouvement\s+part\s+de\s+vos\s+repères\s+personnels\s+pour\s+sécuriser\s+le\s+concret,\s+vérifier\s+les\s+engagements,\s+puis\s+valider\s+les\s+rôles\s+avec\s+plus\s+de\s+clarté\.?",                    "Le mouvement part d'un appui pratique, passe par une vérification des engagements, puis se termine par une validation plus claire des rôles.",                ),                (r"\.\s+\.", "."),                (r"\.\s*,", ","),                (r"\s+\.", "."),            ]            .into_iter()            .map(|(pattern, replacement)| {                (                    Regex::new(pattern).expect("period mechanical fragment regex"),                    replacement,                )            })            .collect()        })        .as_slice()
}
pub(crate) fn repair_period_truncated_public_tail(text: &str) -> String {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    for marker in ["(par ex.", "(par exemple", "(ex."] {
        if let Some(index) = lower.rfind(marker) {
            if !trimmed[index..].contains(')') {
                let mut repaired = trimmed[..index]
                    .trim_end()
                    .trim_end_matches([',', ';', ':'])
                    .to_string();
                if !repaired.ends_with(['.', '!', '?']) {
                    repaired.push('.');
                }
                return repaired;
            }
        }
    }
    trimmed.to_string()
}
pub(crate) fn sanitize_period_evidence_summary(value: Option<&Value>, request: &Value) -> Value {
    let generated = value.and_then(Value::as_array).cloned().unwrap_or_default();
    let fallback_items = request["evidence"]
        .as_array()
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let fallback_by_key = fallback_items
        .iter()
        .filter_map(|item| Some((item.get("evidence_key")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let fallback_by_date = fallback_items
        .iter()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let generated_by_key = generated
        .iter()
        .filter_map(|item| Some((item.get("evidence_key")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let generated_by_date = generated
        .iter()
        .filter_map(|item| Some((item.get("date")?.as_str()?.to_string(), item)))
        .collect::<HashMap<_, _>>();
    let source = if generated.is_empty() {
        fallback_items.iter().take(3).collect::<Vec<_>>()
    } else {
        generated
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                item.get("evidence_key")
                    .and_then(Value::as_str)
                    .and_then(|key| fallback_by_key.get(key).copied())
                    .or_else(|| {
                        item.get("date")
                            .and_then(Value::as_str)
                            .and_then(|date| fallback_by_date.get(date).copied())
                    })
                    .or_else(|| fallback_items.get(index))
            })
            .collect::<Vec<_>>()
    };
    Value::Array(
        source
            .into_iter()
            .enumerate()
            .map(|(index, fallback)| {
                let key = fallback
                    .get("evidence_key")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let date = fallback.get("date").and_then(Value::as_str).unwrap_or("");
                let generated_item = generated
                    .get(index)
                    .or_else(|| generated_by_key.get(key).copied())
                    .or_else(|| generated_by_date.get(date).copied());
                json!({
                    "date": fallback["date"],
                    "evidence_key": fallback["evidence_key"],
                    "label": sanitize_period_public_string(
                        generated_item
                            .and_then(|item| item.get("label"))
                            .and_then(Value::as_str)
                            .filter(|label| !label.trim().is_empty())
                            .or_else(|| fallback.get("human_label").and_then(Value::as_str))
                            .unwrap_or("Repère de période")
                    )
                })
            })
            .collect(),
    )
}

use super::*;
pub(crate) fn build_period_events(
    evidence: &[Value],
    period_resolution: &Value,
) -> Result<Vec<Value>, GenerationError> {
    let included = period_resolution["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect::<HashSet<_>>();
    let theme_counts = evidence
        .iter()
        .filter_map(|item| item.get("theme_code").and_then(Value::as_str))
        .fold(HashMap::<&str, usize>::new(), |mut counts, theme| {
            *counts.entry(theme).or_default() += 1;
            counts
        });
    let mut out = Vec::new();
    for item in evidence {
        let date = item["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        if !included.contains(date) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_EVENT_OUTSIDE_WINDOW"));
        }
        let evidence_key = item["evidence_key"].as_str().unwrap_or("");
        let event_type = if item["fact_type"].as_str() == Some("moon_house_by_day") {
            "moon_house_by_day"
        } else if item["fact_type"].as_str() == Some("transit_to_natal")
            && item.get("orb_deg").and_then(|v| v.as_f64()).unwrap_or(9.0) <= 1.0
        {
            "transit_exact"
        } else if item["fact_type"].as_str() == Some("transit_context") {
            "transit_context"
        } else {
            "transit_active"
        };
        let theme_code = item["theme_code"].as_str().unwrap_or("organization");
        let score = period_event_score(item, event_type);
        let theme_density_score =
            period_theme_density_score(score, *theme_counts.get(theme_code).unwrap_or(&1));
        out.push(json!({            "event_key": format!("event:{evidence_key}"),            "event_type": event_type,            "date": date,            "snapshot_key": item.get("snapshot_key").cloned().unwrap_or(Value::Null),            "theme_code": item["theme_code"],            "tone": item["tone"],            "aspect": item.get("aspect").cloned().unwrap_or(Value::Null),            "score": score,            "theme_density_score": theme_density_score,            "fact_type": item.get("fact_type").cloned().unwrap_or(Value::Null),            "transiting_object": item.get("transiting_object").cloned().unwrap_or(Value::Null),            "natal_target": item.get("natal_target").cloned().unwrap_or(Value::Null),            "natal_house": item.get("natal_house").cloned().unwrap_or(Value::Null),            "natal_focus_hint": item.get("natal_focus_hint").cloned().unwrap_or(Value::Null),            "personalization_hint": item.get("personalization_hint").cloned().unwrap_or(Value::Null),            "evidence_keys": [evidence_key]        }));
    }
    if out.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    out.sort_by(period_event_sort);
    Ok(out)
}
pub(crate) fn build_daily_plans(
    included_dates: &[String],
    events: &[Value],
) -> Result<Vec<Value>, GenerationError> {
    let mut out = Vec::new();
    let mut theme_counts = HashMap::<String, usize>::new();
    for date in included_dates {
        let event = select_daily_plan_event(date, events, &theme_counts)?;
        let theme = event["theme_code"].as_str().unwrap_or("organization");
        *theme_counts.entry(theme.to_string()).or_default() += 1;
        let theme_label = period_theme_public_label(theme);
        let tone = event["tone"].as_str().unwrap_or("focused");
        let evidence_keys = event["evidence_keys"].clone();
        let style = period_style_variant_for_theme(theme);
        let personalization_hint = event
            .get("personalization_hint")
            .and_then(Value::as_str)
            .unwrap_or_else(|| period_event_personalization_hint(event));
        let natal_focus_hint = event
            .get("natal_focus_hint")
            .and_then(Value::as_str)
            .unwrap_or(personalization_hint);
        out.push(json!({            "date": date,            "day_label": public_day_label(date),            "theme_code": theme,            "theme_label": theme_label,            "tone": tone,            "summary_hint": format!("Synthèse journalière centrée sur {theme_label} avec une nuance natale lisible."),            "advice_hint": period_advice_hint(theme, natal_focus_hint),            "style_variant_code": style.code,            "avoid_terms": style.avoid_terms,            "natal_focus_hint": natal_focus_hint,            "personalization_hint": personalization_hint,            "evidence_keys": evidence_keys        }));
    }
    Ok(out)
}
pub(crate) fn select_daily_plan_event<'a>(
    date: &str,
    events: &'a [Value],
    theme_counts: &HashMap<String, usize>,
) -> Result<&'a Value, GenerationError> {
    let candidates = events
        .iter()
        .filter(|event| event["date"].as_str() == Some(date))
        .collect::<Vec<_>>();
    let candidates = if candidates.is_empty() {
        events.iter().collect::<Vec<_>>()
    } else {
        candidates
    };
    let best = candidates
        .iter()
        .copied()
        .min_by(|left, right| period_event_sort(left, right))
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    let Some(best_theme) = best["theme_code"].as_str() else {
        return Ok(best);
    };
    if theme_counts.get(best_theme).copied().unwrap_or(0) < PREMIUM_MAX_SAME_DAILY_THEME {
        return Ok(best);
    }
    Ok(candidates
        .iter()
        .copied()
        .filter(|event| {
            let theme = event["theme_code"].as_str().unwrap_or("");
            theme != best_theme
                && theme_counts.get(theme).copied().unwrap_or(0) < PREMIUM_MAX_SAME_DAILY_THEME
        })
        .min_by(|left, right| period_event_sort(left, right))
        .unwrap_or(best))
}
pub(crate) fn period_internal_tone(
    theme: &str,
    fact_type: &str,
    aspect: Option<&str>,
) -> &'static str {
    match aspect {
        Some("square") | Some("opposition") => "careful",
        Some("trine") | Some("sextile") => "supportive",
        Some("conjunction") => "active",
        _ => match (theme, fact_type) {
            ("relationship", _) => "supportive",
            ("energy", _) | ("communication", _) => "active",
            ("integration", _) => "mixed",
            ("clarity", _) | ("organization", _) | ("routine", _) => "focused",
            _ => "focused",
        },
    }
}
pub(crate) fn period_event_score(item: &Value, event_type: &str) -> f64 {
    let orb = item.get("orb_deg").and_then(Value::as_f64);
    let base = match event_type {
        "transit_exact" => 0.98 - orb.unwrap_or(1.0).min(1.0) * 0.08,
        "transit_active" => 0.90 - orb.unwrap_or(6.0).min(6.0) * 0.025,
        "moon_house_by_day" => {
            0.60 + item
                .get("natal_house")
                .and_then(Value::as_i64)
                .map_or(0.0, |_| 0.05)
        }
        "transit_context" => 0.45 + context_object_bonus(item["transiting_object"].as_str()),
        _ => 0.50,
    };
    round2(base.min(1.0))
}
pub(crate) fn period_theme_density_score(base_score: f64, theme_count: usize) -> f64 {
    let repetition_bonus = ((theme_count.saturating_sub(1)).min(3) as f64) * 0.03;
    round2((base_score + repetition_bonus).min(1.0))
}
pub(crate) fn context_object_bonus(object: Option<&str>) -> f64 {
    match object {
        Some("sun") | Some("jupiter") => 0.12,
        Some("venus") | Some("mars") | Some("mercury") => 0.08,
        Some("moon") => 0.05,
        _ => 0.0,
    }
}
pub(crate) fn period_event_sort(left: &Value, right: &Value) -> std::cmp::Ordering {
    let left_score = left.get("score").and_then(Value::as_f64).unwrap_or(0.0);
    let right_score = right.get("score").and_then(Value::as_f64).unwrap_or(0.0);
    right_score
        .partial_cmp(&left_score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            left.get("date")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(right.get("date").and_then(Value::as_str).unwrap_or(""))
        })
}
pub(crate) fn is_period_watch_event(event: &Value) -> bool {
    let tone = event.get("tone").and_then(Value::as_str);
    let aspect = event.get("aspect").and_then(Value::as_str);
    tone == Some("careful") || matches!(aspect, Some("square") | Some("opposition"))
}
pub(crate) fn is_period_best_candidate(event: &Value) -> bool {
    if is_period_watch_event(event) {
        return false;
    }
    let natal_house = event.get("natal_house").and_then(Value::as_i64);
    !matches!(natal_house, Some(8 | 12))
}
pub(crate) fn build_period_key_day_markers(events: &[Value], limit: usize) -> Vec<Value> {
    let Some(top_score) = events.first().and_then(|event| event["score"].as_f64()) else {
        return Vec::new();
    };
    let min_score = top_score - 0.08;
    let theme_counts = period_theme_counts(events);
    let mut candidates = events
        .iter()
        .filter(|event| {
            let score = event["score"].as_f64().unwrap_or(0.0);
            score >= 0.60 && score >= min_score
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_score = left["score"].as_f64().unwrap_or(0.0);
        let right_score = right["score"].as_f64().unwrap_or(0.0);
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let left_theme = left["theme_code"].as_str().unwrap_or("");
                let right_theme = right["theme_code"].as_str().unwrap_or("");
                theme_counts
                    .get(left_theme)
                    .unwrap_or(&usize::MAX)
                    .cmp(theme_counts.get(right_theme).unwrap_or(&usize::MAX))
            })
            .then_with(|| {
                left["date"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(right["date"].as_str().unwrap_or(""))
            })
    });
    build_period_day_markers_from_events(
        &candidates,
        limit,
        "Jour clé",
        PeriodMarkerRole::Key,
        None,
        None,
    )
}
#[derive(Clone, Copy)]
pub(crate) enum PeriodMarkerRole {
    Key,
    Best,
    Watch,
}
pub(crate) fn build_period_day_markers_from_events(
    events: &[Value],
    limit: usize,
    title: &str,
    role: PeriodMarkerRole,
    exclude_dates: Option<&HashSet<String>>,
    fallback_reason: Option<&str>,
) -> Vec<Value> {
    let mut seen_dates = HashSet::new();
    let mut theme_occurrences = HashMap::<String, usize>::new();
    events        .iter()        .filter(|event| {            let date = event.get("date").and_then(Value::as_str).unwrap_or("");            !exclude_dates.map_or(false, |dates| dates.contains(date))                && seen_dates.insert(date.to_string())        })        .take(limit)        .map(|event| {            let occurrence_index = period_marker_theme_occurrence(event, &mut theme_occurrences);            json!({                "date": event["date"],                "title": title,                "reason": period_marker_reason(role, event, occurrence_index),                "evidence_keys": event["evidence_keys"],                "fallback_reason": fallback_reason.map_or(Value::Null, |reason| json!(reason))            })        })        .collect()
}
pub(crate) fn build_period_best_day_markers(
    events: &[Value],
    watch_dates: &HashSet<String>,
    key_dates: &HashSet<String>,
    limit: usize,
) -> Vec<Value> {
    let mut used_themes = HashSet::new();
    let mut used_dates = HashSet::new();
    let mut out = Vec::new();
    for event in events {
        let date = event["date"].as_str().unwrap_or("");
        let theme = event["theme_code"].as_str().unwrap_or("");
        if watch_dates.contains(date)
            || key_dates.contains(date)
            || !is_period_best_candidate(event)
            || !used_dates.insert(date.to_string())
            || !used_themes.insert(theme.to_string())
        {
            continue;
        }
        out.push(build_period_marker(
            event,
            period_best_day_title(theme),
            PeriodMarkerRole::Best,
            None,
        ));
        if out.len() == limit {
            break;
        }
    }
    out
}
pub(crate) fn build_period_watch_day_markers(events: &[Value], limit: usize) -> Vec<Value> {
    let tension_candidates = events
        .iter()
        .filter(|event| is_period_watch_event(event))
        .cloned()
        .collect::<Vec<_>>();
    build_period_day_markers_from_events(
        &tension_candidates,
        limit,
        "Jour de vigilance",
        PeriodMarkerRole::Watch,
        None,
        None,
    )
}
pub(crate) fn build_period_watch_summary_plan(
    watch_days: &[Value],
    premium: bool,
    watch_windows: &[Value],
) -> Value {
    if watch_days.is_empty() {
        if premium && !watch_windows.is_empty() {
            return json!({                "status": "low",                "text": "Aucune fenêtre de vigilance forte ne ressort, mais certains moments demandent de limiter la dispersion et de garder une marge.",                "evidence_keys": watch_windows                    .iter()                    .flat_map(|window| window["evidence_keys"].as_array().into_iter().flatten())                    .filter_map(Value::as_str)                    .collect::<Vec<_>>()            });
        }
        return json!({            "status": "none",            "text": FREE_PERIOD_NONE_WATCH_SUMMARY,            "evidence_keys": []        });
    }
    json!({        "status": "active",        "text": "Un point de vigilance ressort et mérite une attention mesurée.",        "evidence_keys": watch_days            .iter()            .flat_map(|day| day["evidence_keys"].as_array().into_iter().flatten())            .filter_map(Value::as_str)            .collect::<Vec<_>>()    })
}
pub(crate) fn build_period_best_windows(
    events: &[Value],
    scan_plan: &Value,
    limit: usize,
) -> Vec<Value> {
    let snapshot_keys = scan_plan_snapshot_keys_by_date(scan_plan);
    let mut out = Vec::new();
    let mut used_themes = HashSet::new();
    let mut used_dates = HashSet::new();
    for event in events
        .iter()
        .filter(|event| is_period_best_candidate(event))
    {
        let theme = event["theme_code"].as_str().unwrap_or("organization");
        let date = event["date"].as_str().unwrap_or("");
        if used_themes.contains(theme) || used_dates.contains(date) {
            continue;
        }
        let Some(window) = build_period_window(event, &snapshot_keys, false, 1) else {
            continue;
        };
        out.push(window);
        used_themes.insert(theme.to_string());
        used_dates.insert(date.to_string());
        if out.len() == limit {
            return out;
        }
    }
    for event in events
        .iter()
        .filter(|event| is_period_best_candidate(event))
    {
        if out.len() == limit {
            break;
        }
        let Some(window) = build_period_window(event, &snapshot_keys, false, 1) else {
            continue;
        };
        let already_used = out
            .iter()
            .any(|existing| existing["source_snapshot_keys"] == window["source_snapshot_keys"]);
        if !already_used {
            out.push(window);
        }
    }
    out
}
pub(crate) fn build_period_watch_windows(
    events: &[Value],
    scan_plan: &Value,
    best_windows: &[Value],
    limit: usize,
) -> Vec<Value> {
    let snapshot_keys = scan_plan_snapshot_keys_by_date(scan_plan);
    let best_keys = best_windows
        .iter()
        .flat_map(|window| {
            window["source_snapshot_keys"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect::<HashSet<_>>();
    let mut out = Vec::new();
    let candidates = events
        .iter()
        .filter(|event| is_period_watch_event(event))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Vec::new();
    }
    let mut theme_occurrences = HashMap::<String, usize>::new();
    let mut used_themes = HashSet::new();
    let mut used_dates = HashSet::new();
    for event in candidates {
        let theme = event["theme_code"].as_str().unwrap_or("organization");
        let date = event["date"].as_str().unwrap_or("");
        if used_themes.contains(theme) || used_dates.contains(date) {
            continue;
        }
        let occurrence_index = {
            let count = theme_occurrences
                .entry(period_editorial_theme_key(theme).to_string())
                .or_default();
            *count += 1;
            *count
        };
        let Some(window) = build_period_window(event, &snapshot_keys, true, occurrence_index)
        else {
            continue;
        };
        let overlaps_best = window["source_snapshot_keys"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|key| best_keys.contains(key));
        if overlaps_best {
            continue;
        }
        out.push(window);
        used_themes.insert(theme.to_string());
        used_dates.insert(date.to_string());
        if out.len() == limit {
            break;
        }
    }
    out
}
pub(crate) fn build_period_window(
    event: &Value,
    snapshot_keys: &HashMap<String, Vec<(String, String)>>,
    watch: bool,
    _occurrence_index: usize,
) -> Option<Value> {
    let date = event["date"].as_str()?;
    let snapshots = snapshot_keys.get(date)?;
    let event_snapshot = event
        .get("snapshot_key")
        .and_then(Value::as_str)
        .and_then(|key| {
            snapshots
                .iter()
                .position(|(_, snapshot_key)| snapshot_key == key)
        })
        .unwrap_or(0);
    let (start_label, snapshot_key) = snapshots.get(event_snapshot)?.clone();
    let end_label = snapshots
        .get(event_snapshot + 1)
        .map(|(time, _)| time.clone())
        .unwrap_or_else(|| "00:00".to_string());
    let theme = event["theme_code"].as_str().unwrap_or("organization");
    let tone = event["tone"].as_str().unwrap_or("focused");
    let evidence_keys = event["evidence_keys"].clone();
    if watch {
        Some(
            json!({            "date": date,            "time_range_label": format!("{start_label}–{end_label}"),            "source_snapshot_keys": [snapshot_key],            "title": period_watch_window_title(theme, &start_label),            "theme": period_theme_public_label(theme),            "tone": period_tone_public_label(tone),            "watch_point": period_watch_window_point(theme),            "evidence_keys": evidence_keys        }),
        )
    } else {
        Some(
            json!({            "date": date,            "time_range_label": format!("{start_label}–{end_label}"),            "source_snapshot_keys": [snapshot_key],            "title": period_best_window_title(theme, &start_label),            "theme": period_theme_public_label(theme),            "tone": period_tone_public_label(tone),            "reason": period_best_window_reason(theme),            "best_for": period_best_window_best_for(theme, &start_label),            "evidence_keys": evidence_keys        }),
        )
    }
}
pub(crate) fn scan_plan_snapshot_keys_by_date(
    scan_plan: &Value,
) -> HashMap<String, Vec<(String, String)>> {
    let mut by_date: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for snapshot in scan_plan["snapshots"].as_array().into_iter().flatten() {
        let Some(date) = snapshot["date"].as_str() else {
            continue;
        };
        let time = snapshot["reference_time_local"]
            .as_str()
            .unwrap_or("12:00")
            .to_string();
        let key = snapshot["snapshot_key"].as_str().unwrap_or("").to_string();
        by_date
            .entry(date.to_string())
            .or_default()
            .push((time, key));
    }
    for items in by_date.values_mut() {
        items.sort_by(|left, right| left.0.cmp(&right.0));
    }
    by_date
}
pub(crate) fn period_best_window_title(theme: &str, start_label: &str) -> &'static str {
    match (theme, start_label) {
        ("relationship", "00:00") => "Apaiser une attente personnelle",
        ("relationship", "06:00") => "Ouvrir un échange utile",
        ("relationship", "12:00") => "Clarifier une attente relationnelle",
        ("relationship", _) => "Retrouver une fluidité relationnelle",
        ("energy", "00:00") => "Relancer l'élan sans brusquer",
        ("energy", "06:00") => "Passer à l'action courte",
        ("energy", "12:00") => "Canaliser l'énergie disponible",
        ("energy", _) => "Transformer l'élan en décision",
        ("communication", "00:00") => "Préparer une parole nette",
        ("communication", "06:00") => "Formuler le message essentiel",
        ("communication", "12:00") => "Mettre les mots au bon endroit",
        ("communication", _) => "Répondre avec plus de précision",
        ("clarity", "00:00") => "Reprendre l'initiative personnelle",
        ("clarity", "06:00") => "Clarifier le cap visible",
        ("clarity", "12:00") => "Choisir une suite simple",
        ("clarity", _) => "Retrouver une impulsion créative",
        ("integration", "00:00") => "Stabiliser une base intérieure",
        ("integration", "06:00") => "Consolider ce qui doit durer",
        ("integration", "12:00") => "Relier les décisions au cadre",
        ("integration", _) => "Préparer une suite plus stable",
        (_, "00:00") => "Reprendre l'initiative personnelle",
        (_, "06:00") => "Clarifier le cap visible",
        (_, "12:00") => "Stabiliser une décision utile",
        _ => "Retrouver une impulsion créative",
    }
}
pub(crate) fn period_watch_window_title(theme: &str, start_label: &str) -> &'static str {
    let _ = start_label;
    period_public_theme_field(theme, "watch_window_title", "Ralentir avant de répondre")
}
pub(crate) fn period_best_window_reason(theme: &str) -> &'static str {
    match theme {        "relationship" => "À utiliser pour nommer un besoin, confirmer une attente ou réparer un malentendu simple.",        "energy" => "À réserver à une action courte : lancer, terminer ou limiter un effort avant dispersion.",        "communication" => "À utiliser pour préparer une phrase claire, envoyer un message ciblé ou cadrer une réponse.",        "clarity" => "À privilégier pour choisir entre deux options, clarifier une preuve ou mettre une priorité au net.",        "integration" => "À garder pour consolider un engagement, vérifier sa tenue ou réduire ce qui surcharge.",        _ => "À utiliser pour confirmer une ressource, fermer une tâche pratique ou poser une preuve simple.",    }
}
pub(crate) fn period_watch_window_point(theme: &str) -> &'static str {
    period_public_theme_field(
        theme,
        "watch_window_point",
        "Gardez une marge avant de transformer l'impression en décision définitive.",
    )
}
pub(crate) fn period_best_window_best_for(theme: &str, start_label: &str) -> Vec<&'static str> {
    match (theme, start_label) {
        ("relationship", "00:00") => vec![
            "apaiser une attente personnelle",
            "préparer un échange sensible",
            "retrouver une disponibilité affective",
        ],
        ("relationship", "06:00") => vec![
            "ouvrir un échange utile",
            "clarifier une attente",
            "réparer un malentendu simple",
        ],
        ("relationship", "12:00") => vec![
            "poser un accord concret",
            "nommer un besoin relationnel",
            "ajuster une attente partagée",
        ],
        ("relationship", _) => vec![
            "fluidifier une relation",
            "répondre avec nuance",
            "consolider un lien utile",
        ],
        ("energy", "00:00") => vec![
            "préparer l'élan du jour",
            "choisir une action courte",
            "éviter de démarrer trop vite",
        ],
        ("energy", "06:00") => vec![
            "lancer une action courte",
            "débloquer une décision pratique",
            "poser une limite concrète",
        ],
        ("energy", "12:00") => vec![
            "canaliser l'effort utile",
            "traiter un point actif",
            "agir sans disperser l'énergie",
        ],
        ("energy", _) => vec![
            "transformer l'élan en décision",
            "conclure une action simple",
            "récupérer après l'effort",
        ],
        ("communication", "00:00") => vec![
            "préparer une formulation",
            "ordonner les arguments",
            "clarifier l'intention du message",
        ],
        ("communication", "06:00") => vec![
            "envoyer un message clair",
            "préparer une réponse",
            "nommer une priorité",
        ],
        ("communication", "12:00") => vec![
            "ajuster une réponse",
            "tenir un échange précis",
            "réduire les explications inutiles",
        ],
        ("communication", _) => vec![
            "répondre avec précision",
            "clore une discussion utile",
            "poser un cadre verbal",
        ],
        ("clarity", "00:00") => vec![
            "reprendre l'initiative personnelle",
            "poser un repère simple",
            "préparer le rythme du jour",
        ],
        ("clarity", "06:00") => vec![
            "clarifier le cap visible",
            "organiser la prochaine étape",
            "rendre une priorité lisible",
        ],
        ("clarity", "12:00") => vec![
            "trier les options",
            "choisir une suite simple",
            "mettre à jour une priorité",
        ],
        ("clarity", _) => vec![
            "retrouver une impulsion créative",
            "assouplir une décision",
            "préserver un élan durable",
        ],
        ("integration", "00:00") => vec![
            "stabiliser une base intérieure",
            "préparer une consolidation",
            "faire le point avant d'élargir",
        ],
        ("integration", "06:00") => vec![
            "consolider une avancée",
            "revenir à l'essentiel",
            "stabiliser une décision",
        ],
        ("integration", "12:00") => vec![
            "relier une décision au cadre",
            "vérifier la tenue d'un engagement",
            "ordonner ce qui doit durer",
        ],
        ("integration", _) => vec![
            "préparer une suite stable",
            "assimiler une étape",
            "réduire ce qui surcharge",
        ],
        (_, "00:00") => vec![
            "reprendre l'initiative personnelle",
            "poser un repère simple",
            "préparer le rythme du jour",
        ],
        (_, "06:00") => vec![
            "clarifier le cap visible",
            "organiser la prochaine étape",
            "rendre une priorité lisible",
        ],
        (_, "12:00") => vec![
            "stabiliser une décision utile",
            "trier les options concrètes",
            "réduire la dispersion",
        ],
        _ => vec![
            "retrouver une impulsion créative",
            "assouplir une décision",
            "préserver un élan durable",
        ],
    }
}
pub(crate) fn build_period_premium_scores(request: &Value) -> Value {
    let events = request["period_events"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let event_score = events
        .first()
        .and_then(|event| event["score"].as_f64())
        .unwrap_or(0.0);
    let tension_score = events
        .iter()
        .filter(|event| is_period_watch_event(event))
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0, f64::max);
    let support_score = events
        .iter()
        .filter(|event| !is_period_watch_event(event))
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0, f64::max);
    json!({        "event_score": round2(event_score),        "day_score": round2(event_score * 0.92),        "window_score": round2(support_score.max(tension_score) * 0.95),        "domain_score": round2(period_domain_coverage_score(&events)),        "tension_score": round2(tension_score),        "support_score": round2(support_score),        "clarity_score": round2(period_theme_score(&events, "clarity")),        "relationship_score": round2(period_theme_score(&events, "relationship")),        "energy_score": round2(period_theme_score(&events, "energy")),        "decision_score": round2(period_theme_score(&events, "communication").max(period_theme_score(&events, "clarity"))),        "integration_score": round2(period_theme_score(&events, "integration"))    })
}
pub(crate) fn period_theme_score(events: &[Value], theme: &str) -> f64 {
    events
        .iter()
        .filter(|event| event["theme_code"].as_str() == Some(theme))
        .filter_map(|event| event["score"].as_f64())
        .fold(0.0, f64::max)
}
pub(crate) fn period_domain_coverage_score(events: &[Value]) -> f64 {
    if events.is_empty() {
        return 0.0;
    }
    let distinct_themes = events
        .iter()
        .filter_map(|event| event["theme_code"].as_str())
        .collect::<HashSet<_>>()
        .len() as f64;
    let evidence_coverage = (events.len() as f64 / 50.0).min(1.0);
    let theme_coverage = (distinct_themes / 6.0).min(1.0);
    round2((theme_coverage * 0.7) + (evidence_coverage * 0.3))
}
pub(crate) fn build_period_marker(
    event: &Value,
    title: &str,
    role: PeriodMarkerRole,
    fallback_reason: Option<&str>,
) -> Value {
    json!({        "date": event["date"],        "title": title,        "reason": period_marker_reason(role, event, 1),        "evidence_keys": event["evidence_keys"],        "fallback_reason": fallback_reason.map_or(Value::Null, |reason| json!(reason))    })
}
pub(crate) fn period_marker_theme_occurrence(
    event: &Value,
    theme_occurrences: &mut HashMap<String, usize>,
) -> usize {
    let theme = event
        .get("theme_code")
        .and_then(Value::as_str)
        .unwrap_or("organization");
    let count = theme_occurrences
        .entry(period_editorial_theme_key(theme).to_string())
        .or_default();
    *count += 1;
    *count
}
pub(crate) fn period_marker_reason(
    role: PeriodMarkerRole,
    event: &Value,
    occurrence_index: usize,
) -> String {
    let date = event.get("date").and_then(Value::as_str).unwrap_or("");
    let theme = event
        .get("theme_code")
        .and_then(Value::as_str)
        .unwrap_or("principal");
    let focus = period_public_focus_text(event);
    let arc_rule = period_editorial_arc_rule(theme, occurrence_index);
    let action = period_editorial_rule_field(&arc_rule, "action_mode", "prioriser");
    let situation = period_editorial_reader_situation(&arc_rule, theme, action);
    match role {
        PeriodMarkerRole::Key => format!(
            "{} sert de repère pour {}. {} {}",
            public_day_label(date),
            action,
            situation,
            period_marker_key_focus_sentence(&focus)
        ),
        PeriodMarkerRole::Best => format!(
            "{} {}",
            period_best_marker_intro(date, theme),
            period_marker_best_focus_sentence(&focus, date)
        ),
        PeriodMarkerRole::Watch => format!(
            "{} {} {}",
            period_watch_marker_intro(date),
            situation,
            period_marker_watch_focus_sentence(&focus)
        ),
    }
}
pub(crate) fn period_best_marker_intro(date: &str, theme: &str) -> String {
    let date_label = public_day_label(date);
    match period_text_variant_index(date, 3) {
        0 => format!(
            "{date_label} ouvre une opportunité pour {}.",
            period_best_marker_public_use(theme)
        ),
        1 => format!(
            "{date_label} offre un bon appui pour {}.",
            period_best_marker_public_use(theme)
        ),
        _ => format!(
            "{date_label} aide à passer au concret pour {}.",
            period_best_marker_public_use(theme)
        ),
    }
}
pub(crate) fn period_watch_marker_intro(date: &str) -> String {
    let date_label = public_day_label(date);
    match period_text_variant_index(date, 3) {
        0 => format!("{date_label} demande de ralentir."),
        1 => format!("{date_label} demande un dernier contrôle."),
        _ => format!("{date_label} mérite une marge de prudence."),
    }
}
pub(crate) fn period_best_marker_public_use(theme: &str) -> &'static str {
    match theme {        "relationship" => "apaiser un lien, nommer un besoin personnel ou confirmer une attente simple",        "energy" => "transformer l'élan en action courte sans brusquer le rythme",        "communication" => "envoyer un message net, demander une précision ou cadrer un échange",        "clarity" => "mettre au net ce qui compte et choisir une suite vérifiable",        "integration" => "consolider un engagement, une limite ou une décision déjà amorcée",        _ => "sécuriser ce qui soutient la semaine : ressource, rendez-vous, preuve ou tâche pratique",    }
}
pub(crate) fn period_marker_key_focus_sentence(focus: &str) -> String {
    let parts = period_focus_parts(focus, 2);
    match parts.as_slice() {
        [] => "Gardez le cadre vérifiable avant d'élargir.".to_string(),
        [one] => format!("Traitez d'abord ce point : {one}."),
        [one, two, ..] => {
            format!("Gardez deux repères concrets : {one} et {two}.")
        }
    }
}
pub(crate) fn period_marker_best_focus_sentence(focus: &str, date: &str) -> String {
    let parts = period_focus_parts(focus, 2);
    let variant = period_text_variant_index(date, 3);
    match parts.as_slice() {
        [] => match variant {
            0 => "Servez-vous-en pour confirmer une base concrète.".to_string(),
            1 => "C'est un bon appui pour poser une preuve simple.".to_string(),
            _ => "La fenêtre soutient une avancée pratique et vérifiable.".to_string(),
        },
        [one] => match variant {
            0 => format!("Ce jour aide à sécuriser un point précis : {one}."),
            1 => format!("Appuyez-vous dessus pour avancer concrètement : {one}."),
            _ => format!("La marge favorable sert à poser une preuve simple : {one}."),
        },
        [one, two, ..] => match variant {
            0 => format!("Ce jour aide à sécuriser deux points : {one}, puis {two}."),
            1 => format!("Appuyez-vous dessus pour avancer concrètement : {one}, puis {two}."),
            _ => format!("La marge favorable sert à poser une preuve simple : {one}, puis {two}."),
        },
    }
}
pub(crate) fn period_marker_watch_focus_sentence(focus: &str) -> String {
    let parts = period_focus_parts(focus, 2);
    match parts.as_slice() {
        [] => "Vérifiez délai et charge avant d'accepter.".to_string(),
        [one] => format!("Vérifiez {one} avant d'accepter."),
        [one, two, ..] => {
            format!("Vérifiez {one} et {two} avant d'accepter.")
        }
    }
}
pub(crate) fn naturalize_period_focus(focus: &str) -> String {
    let parts = period_focus_parts(focus, 3);
    match parts.as_slice() {
        [one] => format!("Le geste utile consiste à {one}."),
        [one, two] => format!("Le geste utile consiste à {one}, puis à {two}."),
        [one, two, three, ..] => {
            format!("Le geste utile consiste à {one}, à {two} ou à {three}.")
        }
        _ => "Choisissez un geste simple et vérifiable.".to_string(),
    }
}
pub(crate) fn period_best_day_title(theme: &str) -> &'static str {
    period_public_theme_field(theme, "best_day_title", "Jour favorable")
}
pub(crate) fn period_theme_counts(events: &[Value]) -> HashMap<&str, usize> {
    events
        .iter()
        .filter_map(|event| event["theme_code"].as_str())
        .fold(HashMap::new(), |mut counts, theme| {
            *counts.entry(theme).or_default() += 1;
            counts
        })
}
pub(crate) fn build_period_domain_sections(evidence: &[Value], max_sections: usize) -> Vec<Value> {
    let mut by_theme: HashMap<String, Vec<&Value>> = HashMap::new();
    for item in evidence {
        let theme = item["theme_code"].as_str().unwrap_or("organization");
        by_theme.entry(theme.to_string()).or_default().push(item);
    }
    let mut themes = by_theme
        .into_iter()
        .map(|(theme, items)| {
            let score = items.len() as f64
                + items
                    .iter()
                    .filter_map(|item| item.get("orb_deg").and_then(Value::as_f64))
                    .map(|orb| (6.0 - orb).max(0.0) / 10.0)
                    .sum::<f64>();
            (theme, items, score)
        })
        .collect::<Vec<_>>();
    themes.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    themes        .into_iter()        .take(max_sections)        .map(|(theme, items, _)| {            let first = items.first().copied().unwrap_or(&Value::Null);            let evidence_keys = items                .iter()                .filter_map(|item| item["evidence_key"].as_str())                .take(3)                .collect::<Vec<_>>();            let label = period_theme_public_label(&theme);            let natal_hint = first["natal_focus_hint"]                .as_str()                .unwrap_or("Relier ce domaine à une priorité concrète.");            let personalization = first["personalization_hint"].as_str().unwrap_or(natal_hint);            json!({                "domain": label,                "title": period_domain_title(&theme),                "focus": period_domain_focus(&theme, personalization),                "natal_focus_hint": natal_hint,                "personalization_hint": personalization,                "evidence_keys": evidence_keys            })        })        .collect::<Vec<_>>()
}

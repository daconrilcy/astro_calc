use super::*;

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

pub(crate) fn build_period_events(
    evidence: &[Value],
    period_resolution: &Value,
) -> Result<Vec<Value>, GenerationError> {
    let included_dates = period_resolution["included_dates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    let theme_counts = evidence.iter().fold(HashMap::<&str, usize>::new(), |mut counts, item| {
        let theme = item["theme_code"].as_str().unwrap_or("organization");
        *counts.entry(theme).or_default() += 1;
        counts
    });
    let mut events = Vec::new();
    for item in evidence {
        let date = item["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
        if !included_dates.contains(date) {
            return Err(horoscope_error("HOROSCOPE_PERIOD_EVENT_OUTSIDE_WINDOW"));
        }
        let evidence_key = item["evidence_key"].as_str().unwrap_or("");
        let event_type = match item["fact_type"].as_str() {
            Some("moon_house_by_day") => "moon_house_by_day",
            Some("transit_context") => "transit_context",
            Some("transit_to_natal")
                if item.get("orb_deg").and_then(Value::as_f64).unwrap_or(9.0) <= 1.0 =>
            {
                "transit_exact"
            }
            _ => "transit_active",
        };
        let theme_code = item["theme_code"].as_str().unwrap_or("organization");
        let score = period_event_score(item, event_type);
        let theme_density_score =
            period_theme_density_score(score, *theme_counts.get(theme_code).unwrap_or(&1));
        events.push(json!({
            "event_key": format!("event:{evidence_key}"),
            "event_type": event_type,
            "date": date,
            "snapshot_key": item.get("snapshot_key").cloned().unwrap_or(Value::Null),
            "theme_code": item["theme_code"],
            "tone": item["tone"],
            "aspect": item.get("aspect").cloned().unwrap_or(Value::Null),
            "score": score,
            "theme_density_score": theme_density_score,
            "fact_type": item.get("fact_type").cloned().unwrap_or(Value::Null),
            "transiting_object": item.get("transiting_object").cloned().unwrap_or(Value::Null),
            "natal_target": item.get("natal_target").cloned().unwrap_or(Value::Null),
            "natal_house": item.get("natal_house").cloned().unwrap_or(Value::Null),
            "natal_focus_hint": item.get("natal_focus_hint").cloned().unwrap_or(Value::Null),
            "personalization_hint": item
                .get("personalization_hint")
                .cloned()
                .unwrap_or(Value::Null),
            "evidence_keys": [evidence_key]
        }));
    }
    if events.is_empty() {
        return Err(horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"));
    }
    events.sort_by(period_event_sort);
    Ok(events)
}

pub(crate) fn is_period_watch_event(event: &Value) -> bool {
    let tone = event["tone"].as_str();
    let aspect = event["aspect"].as_str();
    tone == Some("careful") || matches!(aspect, Some("square") | Some("opposition"))
}

pub(crate) fn is_period_best_candidate(event: &Value) -> bool {
    if is_period_watch_event(event) {
        return false;
    }
    !matches!(event["natal_house"].as_i64(), Some(8 | 12))
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
        let snapshot_key = snapshot["snapshot_key"].as_str().unwrap_or("").to_string();
        by_date
            .entry(date.to_string())
            .or_default()
            .push((time, snapshot_key));
    }
    for snapshots in by_date.values_mut() {
        snapshots.sort_by(|left, right| left.0.cmp(&right.0));
    }
    by_date
}

pub(crate) fn build_period_window(
    event: &Value,
    snapshot_keys: &HashMap<String, Vec<(String, String)>>,
    watch: bool,
    _occurrence_index: usize,
) -> Option<Value> {
    let date = event["date"].as_str()?;
    let snapshots = snapshot_keys.get(date)?;
    let event_snapshot_index = event["snapshot_key"]
        .as_str()
        .and_then(|key| {
            snapshots
                .iter()
                .position(|(_, snapshot_key)| snapshot_key == key)
        })
        .unwrap_or(0);
    let (start_label, source_snapshot_key) = snapshots.get(event_snapshot_index)?.clone();
    let end_label = snapshots
        .get(event_snapshot_index + 1)
        .map(|(time, _)| time.clone())
        .unwrap_or_else(|| "00:00".to_string());
    let theme = event["theme_code"].as_str().unwrap_or("organization");
    let tone = event["tone"].as_str().unwrap_or("focused");
    let evidence_keys = event["evidence_keys"].clone();
    Some(if watch {
        json!({
            "date": date,
            "time_range_label": format!("{start_label}–{end_label}"),
            "source_snapshot_keys": [source_snapshot_key],
            "title": period_watch_window_title(theme, &start_label),
            "theme": period_theme_public_label(theme),
            "tone": period_tone_public_label(tone),
            "watch_point": period_watch_window_point(theme),
            "evidence_keys": evidence_keys
        })
    } else {
        json!({
            "date": date,
            "time_range_label": format!("{start_label}–{end_label}"),
            "source_snapshot_keys": [source_snapshot_key],
            "title": period_best_window_title(theme, &start_label),
            "theme": period_theme_public_label(theme),
            "tone": period_tone_public_label(tone),
            "reason": period_best_window_reason(theme),
            "best_for": period_best_window_best_for(theme, &start_label),
            "evidence_keys": evidence_keys
        })
    })
}

fn period_best_window_title(theme: &str, start_label: &str) -> String {
    format!("{} {}", period_theme_public_label(theme), start_label)
}

fn period_watch_window_title(theme: &str, start_label: &str) -> String {
    format!("{} {}", period_theme_public_label(theme), start_label)
}

fn period_best_window_reason(theme: &str) -> String {
    period_public_theme_field(theme, "domain_focus", theme)
}

fn period_watch_window_point(theme: &str) -> String {
    period_public_theme_field(theme, "watch_window_point", theme)
}

fn period_best_window_best_for(theme: &str, start_label: &str) -> Vec<String> {
    vec![
        format!("{} {}", period_theme_public_label(theme), start_label),
        period_public_theme_field(theme, "domain_focus", theme),
    ]
}

fn period_event_score(item: &Value, event_type: &str) -> f64 {
    let orb = item.get("orb_deg").and_then(Value::as_f64);
    let base = match event_type {
        "transit_exact" => 0.98 - orb.unwrap_or(1.0).min(1.0) * 0.08,
        "transit_active" => 0.90 - orb.unwrap_or(6.0).min(6.0) * 0.025,
        "moon_house_by_day" => 0.60 + item["natal_house"].as_i64().map_or(0.0, |_| 0.05),
        "transit_context" => 0.45 + context_object_bonus(item["transiting_object"].as_str()),
        _ => 0.50,
    };
    round2(base.min(1.0))
}

fn period_theme_density_score(base_score: f64, theme_count: usize) -> f64 {
    let repetition_bonus = ((theme_count.saturating_sub(1)).min(3) as f64) * 0.03;
    round2((base_score + repetition_bonus).min(1.0))
}

fn context_object_bonus(object: Option<&str>) -> f64 {
    match object {
        Some("sun") | Some("jupiter") => 0.12,
        Some("venus") | Some("mars") | Some("mercury") => 0.08,
        Some("moon") => 0.05,
        _ => 0.0,
    }
}

fn period_event_sort(left: &Value, right: &Value) -> Ordering {
    let left_score = left["score"].as_f64().unwrap_or(0.0);
    let right_score = right["score"].as_f64().unwrap_or(0.0);
    right_score
        .partial_cmp(&left_score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| {
            left["date"]
                .as_str()
                .unwrap_or("")
                .cmp(right["date"].as_str().unwrap_or(""))
        })
}

use super::*;

pub(crate) fn period_theme_public_label(theme_code: &str) -> &'static str {
    period_public_theme_field(theme_code, "public_label", "priorité principale")
}
pub(crate) fn period_theme_public_label_if_code(theme: &str) -> String {
    period_public_theme_labels()
        .get(period_editorial_theme_key(theme))
        .cloned()
        .unwrap_or_else(|| theme.to_string())
}
pub(crate) fn period_public_theme_labels() -> &'static HashMap<String, String> {
    static THEME_LABELS: OnceLock<HashMap<String, String>> = OnceLock::new();
    THEME_LABELS.get_or_init(|| {
        rows(PERIOD_PUBLIC_THEMES_JSON)
            .unwrap_or_default()
            .into_iter()
            .filter(|row| {
                row.get("is_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                Some((
                    row.get("theme_code")?.as_str()?.to_string(),
                    row.get("public_label")?.as_str()?.to_string(),
                ))
            })
            .collect::<HashMap<_, _>>()
    })
}
pub(crate) fn period_domain_title(theme_code: &str) -> &'static str {
    period_public_theme_field(theme_code, "domain_title", "Priorité utile")
}
pub(crate) fn period_domain_focus(theme_code: &str, personalization: &str) -> String {
    let focus = period_public_theme_field(
        theme_code,
        "domain_focus",
        "Garder une priorité simple, visible et reliée à la situation réelle.",
    );
    format!("{focus} {personalization}")
}
pub(crate) fn period_public_theme_field(
    theme_code: &str,
    field: &str,
    fallback: &'static str,
) -> &'static str {
    let theme_code = period_editorial_theme_key(theme_code);
    static THEME_FIELDS: OnceLock<HashMap<String, HashMap<String, &'static str>>> = OnceLock::new();
    let fields = THEME_FIELDS.get_or_init(|| {
        rows(PERIOD_PUBLIC_THEMES_JSON)
            .unwrap_or_default()
            .into_iter()
            .filter(|row| {
                row.get("is_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                let code = row.get("theme_code")?.as_str()?.to_string();
                let mut values = HashMap::new();
                for field in [
                    "public_label",
                    "domain_title",
                    "domain_focus",
                    "best_day_title",
                    "watch_window_title",
                    "watch_window_point",
                ] {
                    if let Some(value) = row.get(field).and_then(Value::as_str) {
                        values.insert(
                            field.to_string(),
                            Box::leak(value.to_string().into_boxed_str()) as &'static str,
                        );
                    }
                }
                Some((code, values))
            })
            .collect::<HashMap<_, _>>()
    });
    fields
        .get(theme_code)
        .or_else(|| fields.get("default"))
        .and_then(|row| row.get(field).copied())
        .unwrap_or(fallback)
}
#[derive(Clone)]
pub(crate) struct PeriodNatalFocus {
    pub(crate) label: String,
    pub(crate) hint: String,
}
pub(crate) fn period_natal_focus_code(fact: &Value) -> String {
    if let Some(target) = fact.get("natal_target").and_then(Value::as_str) {
        if !target.trim().is_empty() {
            return target.to_string();
        }
    }
    if let Some(house) = fact.get("natal_house").and_then(Value::as_i64) {
        if (1..=12).contains(&house) {
            return format!("natal_house_{house}");
        }
    }
    "natal_house_6".to_string()
}
pub(crate) fn period_natal_focus(code: &str) -> PeriodNatalFocus {
    period_natal_focus_labels()        .get(code)        .cloned()        .unwrap_or_else(|| PeriodNatalFocus {            label: "une priorité concrète".to_string(),            hint: "Situations associées : choisir une priorité simple, vérifier une limite, alléger une charge, garder une marge de décision.".to_string(),        })
}
pub(crate) fn period_natal_focus_labels() -> &'static HashMap<String, PeriodNatalFocus> {
    static LABELS: OnceLock<HashMap<String, PeriodNatalFocus>> = OnceLock::new();
    LABELS.get_or_init(|| {
        serde_json::from_str::<Value>(NATAL_FOCUS_LABELS_JSON)
            .ok()
            .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
            .into_iter()
            .flatten()
            .filter(|row| {
                row.get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                Some((
                    row.get("focus_code")?.as_str()?.to_string(),
                    PeriodNatalFocus {
                        label: row.get("label_fr")?.as_str()?.to_string(),
                        hint: row.get("hint_fr")?.as_str()?.to_string(),
                    },
                ))
            })
            .collect()
    })
}
#[derive(Clone)]
pub(crate) struct PeriodStyleVariant {
    pub(crate) code: String,
    pub(crate) avoid_terms: Value,
}
pub(crate) fn period_style_variant_for_theme(theme: &str) -> PeriodStyleVariant {
    let code = match theme {
        "relationship" => "relation",
        "energy" => "action",
        "communication" => "communication",
        "clarity" => "clarity",
        "integration" => "integration",
        "routine" => "perspective",
        _ => "anchor",
    };
    period_style_variants()
        .get(code)
        .cloned()
        .unwrap_or_else(|| PeriodStyleVariant {
            code: code.to_string(),
            avoid_terms: json!(["restez concret", "gardez une marge"]),
        })
}
pub(crate) fn period_style_variants() -> &'static HashMap<String, PeriodStyleVariant> {
    static VARIANTS: OnceLock<HashMap<String, PeriodStyleVariant>> = OnceLock::new();
    VARIANTS.get_or_init(|| {
        serde_json::from_str::<Value>(PERIOD_STYLE_VARIANTS_JSON)
            .ok()
            .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
            .into_iter()
            .flatten()
            .filter(|row| {
                row.get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                let code = row.get("style_variant_code")?.as_str()?.to_string();
                Some((
                    code.clone(),
                    PeriodStyleVariant {
                        code,
                        avoid_terms: row.get("avoid_terms").cloned().unwrap_or_else(|| json!([])),
                    },
                ))
            })
            .collect()
    })
}
pub(crate) fn period_event_personalization_hint(event: &Value) -> &str {
    event["theme_code"].as_str().map_or(        "Situations associées : choisir une priorité simple, vérifier une limite, alléger une charge.",        |theme| match theme {            "relationship" => "Situations associées : répondre à une attente, préserver un lien, ajuster un accord.",            "energy" => "Situations associées : agir court, doser l'effort, éviter une réaction brusque.",            "communication" => "Situations associées : préparer un message, différer une réponse, vérifier une information.",            "clarity" => "Situations associées : nommer ce qui compte, trier deux options, éviter de conclure trop vite.",            "integration" => "Situations associées : vérifier un délai, réduire une promesse, confirmer seulement si le cadre est clair.",            _ => "Situations associées : choisir une priorité simple, vérifier une limite, alléger une charge.",        },    )
}
pub(crate) fn period_advice_hint(theme: &str, natal_focus_hint: &str) -> String {
    let advice = match theme {
        "relationship" => {
            "Cherchez une réponse relationnelle précise plutôt qu'un accord de façade."
        }
        "energy" => "Choisissez une action courte, assumée et proportionnée.",
        "communication" => "Formulez le message utile avant d'élargir la discussion.",
        "clarity" => "Nommez ce qui compte avant de décider.",
        "integration" => "Reliez ce qui avance à une limite ou un engagement réaliste.",
        "routine" => "Allégez une habitude avant d'en ajouter une autre.",
        _ => "Hiérarchisez une priorité et laissez le reste au second plan.",
    };
    format!("{advice} {natal_focus_hint}")
}
pub(crate) fn period_tone_public_label(tone_code: &str) -> String {
    period_tone_labels()
        .get(tone_code)
        .cloned()
        .unwrap_or_else(|| "nuancé".to_string())
}
pub(crate) fn period_tone_public_label_if_code(tone: &str) -> String {
    let normalized = tone.trim().to_lowercase();
    if normalized.is_empty() {
        return "nuancé".to_string();
    }
    if let Some(label) = period_tone_labels().get(normalized.as_str()) {
        return label.clone();
    }
    if period_public_tone_labels().contains(&normalized) {
        return normalized;
    }
    for fragment in normalized.split(['/', ',', '|', '→', '+']) {
        let fragment = fragment.trim();
        if fragment.is_empty() {
            continue;
        }
        if let Some(label) = period_tone_labels().get(fragment) {
            return label.clone();
        }
        if period_public_tone_labels().contains(fragment) {
            return fragment.to_string();
        }
        if let Some(label) = period_tone_public_label_for_short_synonym(fragment) {
            return label.to_string();
        }
    }
    period_tone_public_label_for_short_synonym(normalized.as_str())
        .unwrap_or(tone)
        .to_string()
}
pub(crate) fn period_tone_public_label_for_short_synonym(tone: &str) -> Option<&'static str> {
    match tone {
        "focalisé" | "focalisee" | "focalisée" | "concentre" | "concentré" | "concentree"
        | "concentrée" => Some("concentré"),
        "supportif" | "supportive" | "soutien" | "soutenant" => Some("soutenant"),
        "actif" | "active" | "dynamique" => Some("dynamique"),
        "mixte" | "mixed" | "nuance" | "nuancé" | "nuancee" | "nuancée" => Some("nuancé"),
        "vigilant" | "careful" | "prudent" | "prudente" => Some("vigilant"),
        "fluide" | "fluid" => Some("fluide"),
        "tendu" | "tendue" | "tense" | "sous tension" => Some("sous tension"),
        _ => None,
    }
}
pub(crate) fn period_public_tone_labels() -> &'static HashSet<String> {
    static PUBLIC_TONE_LABELS: OnceLock<HashSet<String>> = OnceLock::new();
    PUBLIC_TONE_LABELS.get_or_init(|| period_tone_labels().values().cloned().collect())
}
pub(crate) fn period_tone_labels() -> &'static HashMap<String, String> {
    static TONE_LABELS: OnceLock<HashMap<String, String>> = OnceLock::new();
    TONE_LABELS.get_or_init(|| {
        serde_json::from_str::<Value>(TONE_LABELS_JSON)
            .ok()
            .and_then(|value| value.get("data").and_then(Value::as_array).cloned())
            .into_iter()
            .flatten()
            .filter(|row| {
                row.get("is_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .filter_map(|row| {
                Some((
                    row.get("tone_code")?.as_str()?.to_string(),
                    row.get("label_fr")?.as_str()?.to_string(),
                ))
            })
            .collect::<HashMap<_, _>>()
    })
}
pub(crate) fn normalize_period_public_tones(request: &Value, response: &mut Value) {
    let tone_by_date = request["daily_plans"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|day| {
            Some((
                day.get("date")?.as_str()?.to_string(),
                period_tone_public_label(day.get("tone")?.as_str()?),
            ))
        })
        .collect::<HashMap<_, _>>();
    if let Some(days) = response
        .get_mut("daily_timeline")
        .and_then(Value::as_array_mut)
    {
        for day in days {
            if let Some(date) = day.get("date").and_then(Value::as_str) {
                if let Some(label) = tone_by_date.get(date) {
                    day["tone"] = json!(label);
                    continue;
                }
            }
            if let Some(tone) = day.get("tone").and_then(Value::as_str) {
                day["tone"] = json!(period_tone_public_label_if_code(tone));
            }
        }
    }
}
pub(crate) fn validate_period_public_tones(response: &Value) -> Result<(), GenerationError> {
    let allowed = period_public_tone_labels();
    for day in response["daily_timeline"].as_array().into_iter().flatten() {
        let tone = day["tone"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK"))?;
        if !allowed.contains(tone) {
            return Err(quality_error(
                "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
                json!({ "field": "daily_timeline.tone", "tone": tone }),
            ));
        }
    }
    Ok(())
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct PeriodWordLimits {
    pub(crate) target_min: usize,
    pub(crate) target_max: usize,
    pub(crate) hard_limit: usize,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct PeriodDetailProfile {
    pub(crate) max_main_events: usize,
    pub(crate) max_evidence: usize,
    pub(crate) max_key_days: usize,
    pub(crate) max_best_days: usize,
    pub(crate) max_watch_days: usize,
    pub(crate) max_domain_sections: usize,
    pub(crate) max_best_windows: usize,
    pub(crate) max_watch_windows: usize,
    pub(crate) include_best_days: bool,
    pub(crate) include_watch_days: bool,
    pub(crate) include_daily_timeline: bool,
    pub(crate) include_domain_sections: bool,
    pub(crate) include_best_windows: bool,
    pub(crate) include_watch_windows: bool,
    pub(crate) include_strategy_section: bool,
    pub(crate) word_limits: PeriodWordLimits,
}
pub(crate) fn period_detail_profile(
    detail_profile_code: &str,
) -> Result<PeriodDetailProfile, GenerationError> {
    let row = rows(DETAIL_PROFILES_JSON)?
        .into_iter()
        .find(|row| {
            row.get("detail_profile_code").and_then(Value::as_str) == Some(detail_profile_code)
                && row
                    .get("is_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"))?;
    let max_domain_sections = if detail_profile_code == "premium_rich" {
        5
    } else {
        4
    };
    Ok(PeriodDetailProfile {
        max_main_events: row
            .get("max_main_events")
            .and_then(Value::as_u64)
            .unwrap_or(8) as usize,
        max_evidence: row
            .get("max_evidence")
            .and_then(Value::as_u64)
            .unwrap_or(20) as usize,
        max_key_days: row.get("max_key_days").and_then(Value::as_u64).unwrap_or(2) as usize,
        max_best_days: row
            .get("max_best_days")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize,
        max_watch_days: row
            .get("max_watch_days")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize,
        max_domain_sections,
        max_best_windows: row
            .get("max_best_windows")
            .and_then(Value::as_u64)
            .unwrap_or(3) as usize,
        max_watch_windows: row
            .get("max_watch_windows")
            .and_then(Value::as_u64)
            .unwrap_or(3) as usize,
        include_best_days: row
            .get("include_best_days")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_watch_days: row
            .get("include_watch_days")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_daily_timeline: row
            .get("include_daily_timeline")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_domain_sections: row
            .get("include_domain_sections")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        include_best_windows: row
            .get("include_best_windows")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        include_watch_windows: row
            .get("include_watch_windows")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        include_strategy_section: row
            .get("include_strategy_section")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        word_limits: PeriodWordLimits {
            target_min: row
                .get("target_words_min")
                .and_then(Value::as_u64)
                .unwrap_or(800) as usize,
            target_max: row
                .get("target_words_max")
                .and_then(Value::as_u64)
                .unwrap_or(1200) as usize,
            hard_limit: row
                .get("hard_limit_words")
                .and_then(Value::as_u64)
                .unwrap_or(1500) as usize,
        },
    })
}
pub(crate) fn period_basic_word_limits() -> PeriodWordLimits {
    period_detail_profile("basic_standard")
        .map(|profile| profile.word_limits)
        .expect("json_db/horoscope_detail_profiles.json must define basic_standard word limits")
}
pub(crate) fn period_word_limits_for_request(request: &Value) -> PeriodWordLimits {
    request["detail_profile_code"]
        .as_str()
        .and_then(|code| period_detail_profile(code).ok())
        .map(|profile| profile.word_limits)
        .unwrap_or_else(period_basic_word_limits)
}
pub fn period_writer_max_output_tokens(request: &Value) -> u32 {
    if is_period_writer_request_v2(request) {
        return PERIOD_V2_MAX_OUTPUT_TOKENS;
    }
    let limits = period_word_limits_for_request(request);
    ((limits.hard_limit as u32).saturating_mul(3)).saturating_add(500)
}
#[doc(hidden)]
pub fn period_writer_reasoning_effort(request: &Value) -> Option<ReasoningEffort> {
    if is_period_writer_request_v2(request) || is_free_period_request(request) {
        Some(ReasoningEffort::Minimal)
    } else {
        match request["service_code"].as_str() {
            Some(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE) => Some(ReasoningEffort::Minimal),
            _ => None,
        }
    }
}
pub(crate) fn period_effective_min_word_count(request: &Value, limits: &PeriodWordLimits) -> usize {
    if is_period_writer_request_v2(request) {
        limits.target_min.saturating_sub(700)
    } else {
        limits.target_min
    }
}
pub fn validate_period_public_word_count(
    request: &Value,
    response: &Value,
    public_text: &str,
) -> Result<(), GenerationError> {
    if response["quality"]["provider"].as_str() == Some("fake") {
        return Ok(());
    }
    let limits = period_word_limits_for_request(request);
    let effective_min = period_effective_min_word_count(request, &limits);
    let word_count = public_text.split_whitespace().count();
    if word_count < effective_min || word_count > limits.hard_limit {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_WORD_COUNT_OUT_OF_RANGE",
            json!({                "word_count": word_count,                "target_words_min": limits.target_min,                "effective_words_min": effective_min,                "target_words_max": limits.target_max,                "hard_limit_words": limits.hard_limit            }),
        ));
    }
    Ok(())
}
pub(crate) fn period_object_public_label(object_code: &str) -> &'static str {
    match object_code {
        "sun" => "le Soleil",
        "moon" => "la Lune",
        "mercury" => "Mercure",
        "venus" => "Vénus",
        "mars" => "Mars",
        "jupiter" => "Jupiter",
        "saturn" => "Saturne",
        _ => "un facteur astrologique",
    }
}
pub(crate) fn public_day_label(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .map(|date| {
            let label = match date.weekday() {
                chrono::Weekday::Mon => "Lundi",
                chrono::Weekday::Tue => "Mardi",
                chrono::Weekday::Wed => "Mercredi",
                chrono::Weekday::Thu => "Jeudi",
                chrono::Weekday::Fri => "Vendredi",
                chrono::Weekday::Sat => "Samedi",
                chrono::Weekday::Sun => "Dimanche",
            };
            format!("{label} {}", date.format("%d/%m"))
        })
        .unwrap_or_else(|| date.to_string())
}
pub(crate) fn premium_period(
    public: &HoroscopePublicRequest,
    service_code: &str,
    calculation: &Value,
) -> Value {
    let mut period = calculation.get("period").cloned().unwrap_or_else(
        || json!({            "date": public.date,            "timezone": public.timezone        }),
    );
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        if let Some(label) = public
            .location
            .as_ref()
            .and_then(|location| location.label.as_ref())
            .filter(|label| !label.trim().is_empty())
        {
            period["location_label"] = json!(label);
        }
    }
    period
}
pub(crate) fn build_best_slots(request: &Value) -> Vec<Value> {
    premium_ranked_slots(request, false)
}
pub(crate) fn build_watch_slots(request: &Value) -> Vec<Value> {
    premium_ranked_slots(request, true)
}
pub(crate) fn premium_ranked_slots(request: &Value, watch: bool) -> Vec<Value> {
    let slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let mut ranked = slots
        .iter()
        .copied()
        .filter(|slot| {
            let tone = slot.get("tone").and_then(|v| v.as_str()).unwrap_or("");
            if watch {
                tone.contains("tense") || tone.contains("careful")
            } else {
                !tone.contains("tense") && !tone.contains("careful")
            }
        })
        .take(3)
        .collect::<Vec<_>>();
    if ranked.is_empty() {
        ranked = slots.iter().rev().copied().take(3).collect();
    }
    ranked
        .into_iter()
        .map(|slot| premium_slot_summary(slot, watch))
        .collect()
}

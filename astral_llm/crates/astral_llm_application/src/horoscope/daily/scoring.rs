use super::*;
pub fn score_calculation(calculation: &Value) -> Result<Vec<ScoredSignal>, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
    validate_premium_calculation_local_chart(service_code, calculation)?;
    let refs = ReferenceData::load(service_code)?;
    let mut out = Vec::new();
    let slots = calculation
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    for slot in slots {
        let slot_id = slot
            .get("slot_code")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        for fact in slot
            .get("transits_to_natal")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            out.push(score_fact(&refs, slot_id, fact)?);
        }
    }
    out.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.evidence_key.cmp(&b.evidence_key))
    });
    Ok(out)
}
pub fn aggregate_themes(signals: &[ScoredSignal]) -> Vec<Value> {
    let mut totals: HashMap<String, f64> = HashMap::new();
    for signal in signals {
        *totals.entry(signal.theme_code.clone()).or_default() += signal.priority_score;
    }
    let mut themes = totals.into_iter().collect::<Vec<_>>();
    themes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    themes
        .into_iter()
        .map(|(theme_code, score)| json!({ "theme_code": theme_code, "score": round2(score) }))
        .collect()
}
pub(crate) fn score_fact(
    refs: &ReferenceData,
    slot_id: &str,
    fact: &Value,
) -> Result<ScoredSignal, GenerationError> {
    let evidence_key = fact_string(fact, "evidence_key")?;
    let transiting_object = fact_string(fact, "transiting_object")?;
    if !refs.supported_objects.contains(&transiting_object) {
        return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
    }
    let natal_target = fact
        .get("natal_target")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let aspect = fact
        .get("aspect")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    if let Some(aspect) = &aspect {
        if !refs.supported_aspects.contains(aspect) {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
    }
    let orb_deg = fact.get("orb_deg").and_then(|v| v.as_f64());
    let object_weight = refs.weight(&refs.object_weights, &transiting_object);
    let target_weight = natal_target
        .as_deref()
        .map(|target| refs.weight(&refs.target_weights, target))
        .unwrap_or(1.0);
    let aspect_weight = aspect
        .as_deref()
        .map(|aspect| refs.weight(&refs.aspect_weights, aspect))
        .unwrap_or(1.0);
    let orb_weight = refs.orb_weight(orb_deg.unwrap_or(6.0));
    let exactness_bonus = if orb_deg.unwrap_or(9.0) <= refs.scoring.exact_orb_bonus_max_deg {
        refs.scoring.exactness_bonus
    } else {
        0.0
    };
    let priority_score =
        object_weight * target_weight * aspect_weight * orb_weight + exactness_bonus;
    let theme_code = refs.theme_for(
        &transiting_object,
        aspect.as_deref(),
        natal_target.as_deref(),
    );
    let tone = aspect
        .as_deref()
        .and_then(|aspect| refs.aspect_tones.get(aspect))
        .cloned()
        .unwrap_or_else(|| "mixed".into());
    Ok(ScoredSignal {
        evidence_key,
        fact_type: fact
            .get("fact_type")
            .and_then(|v| v.as_str())
            .unwrap_or("transit_to_natal")
            .into(),
        slot_id: slot_id.into(),
        source: fact
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("calculator")
            .into(),
        transiting_object,
        natal_target,
        aspect,
        orb_deg,
        natal_house: fact.get("natal_house").and_then(|v| v.as_i64()),
        theme_code,
        priority_score: round2(priority_score),
        intensity: refs.intensity(priority_score),
        tone,
        duration_class: refs.scoring.default_duration_class.clone(),
        confidence_score: refs.scoring.default_confidence_score,
        human_label: "Preuve astrologique retenue pour l'horoscope quotidien".into(),
        score_breakdown: json!({            "transiting_object_weight": object_weight,            "natal_target_weight": target_weight,            "aspect_weight": aspect_weight,            "orb_weight": orb_weight,            "house_weight": refs.scoring.default_house_weight,            "theme_repetition_bonus": 0.0,            "exactness_bonus": exactness_bonus,            "weak_signal_penalty": 0.0        }),
    })
}
pub(crate) fn build_slot_plans(
    refs: &ReferenceData,
    calculation: &Value,
    signals: &[ScoredSignal],
) -> Result<Vec<SlotInterpretationPlan>, GenerationError> {
    let slots = calculation
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    let mut plans = Vec::new();
    for slot in slots {
        let slot_code = slot
            .get("slot_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
        let public_label = refs.slot_label(slot_code);
        let mut slot_signals = signals
            .iter()
            .filter(|signal| {
                signal.slot_id == slot_code
                    && signal.priority_score >= refs.shortlist.min_priority_score
            })
            .cloned()
            .collect::<Vec<_>>();
        slot_signals.sort_by(|a, b| {
            b.priority_score
                .partial_cmp(&a.priority_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.evidence_key.cmp(&b.evidence_key))
        });
        slot_signals.truncate(refs.shortlist.max_main_signals_per_slot);
        if slot_signals.is_empty() {
            plans.push(SlotInterpretationPlan {
                slot_code: slot_code.to_string(),
                slot_label: public_label,
                specificity: "fallback".into(),
                theme_code: None,
                tone: None,
                intensity: None,
                main_signal_keys: Vec::new(),
                required_evidence_keys: Vec::new(),
                advice_axis: None,
                avoid_axis: None,
                watch_point: None,
                best_for: Vec::new(),
                fallback_reason: Some("no_slot_specific_signal_above_threshold".into()),
            });
            continue;
        }
        let primary = &slot_signals[0];
        let axis = refs.advice_axis(&primary.theme_code);
        let evidence_keys = slot_signals
            .iter()
            .take(refs.shortlist.max_required_evidence_per_slot)
            .map(|signal| signal.evidence_key.clone())
            .collect::<Vec<_>>();
        plans.push(SlotInterpretationPlan {
            slot_code: slot_code.to_string(),
            slot_label: public_label,
            specificity: "specific".into(),
            theme_code: Some(primary.theme_code.clone()),
            tone: Some(
                axis.tone_hint
                    .clone()
                    .unwrap_or_else(|| primary.tone.clone()),
            ),
            intensity: Some(primary.intensity.clone()),
            main_signal_keys: evidence_keys.clone(),
            required_evidence_keys: evidence_keys,
            advice_axis: Some(axis.advice_axis.clone()),
            avoid_axis: axis.avoid_axis.clone(),
            watch_point: axis.watch_point.clone(),
            best_for: axis.best_for.clone(),
            fallback_reason: None,
        });
    }
    let expected = if refs.service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        1
    } else if refs.service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        12
    } else {
        3
    };
    if plans.len() != expected {
        return Err(horoscope_error("HOROSCOPE_CALCULATION_FAILED"));
    }
    Ok(plans)
}
pub(crate) fn render_fake_slot(slot: &Value) -> Result<Value, GenerationError> {
    let slot_code = slot
        .get("slot_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let title = slot
        .get("slot_label")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| slot_label(slot_code));
    let theme_code = slot
        .get("theme_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tone = slot.get("tone").and_then(|v| v.as_str()).unwrap_or("mixed");
    let evidence_keys = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let best_for = slot
        .get("best_for")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let internal_watch_point = slot
        .get("watch_point")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let watch_point = public_watch_point_for_theme(theme_code)?
        .or_else(|| {
            if internal_watch_point.contains("avoid_") {
                None
            } else {
                Some(internal_watch_point.to_string())
            }
        })
        .unwrap_or_default();
    let (theme, text, advice) = match slot_code {        "morning" => (            "Organisation",            "La Lune met l'accent sur l'organisation et les gestes utiles. C'est un bon moment pour clarifier une priorité concrète, ranger une tâche ou reprendre un point simple sans ouvrir trop de sujets à la fois.",            "Choisissez une action vérifiable et terminez-la avant de passer à la suivante.",        ),        "afternoon" => (            "Limites émotionnelles",            "Un contact tendu entre Mars et la Lune natale peut rendre l'après-midi plus réactif. Ce créneau demande de ralentir les réponses, surtout si une discussion devient imprécise ou chargée.",            "Si une tension monte, reformulez d'abord ce que vous avez compris avant de répondre.",        ),        "evening" => (            "Dialogue",            "Vénus soutient Mercure natal et adoucit le climat relationnel du soir. L'enjeu n'est pas de tout résoudre, mais de rouvrir un échange simple, concret et moins défensif.",            "Revenez sur un point précis plutôt que sur toute l'histoire.",        ),        _ => (            "Repère du jour",            "Le climat astrologique du slot donne un repère simple pour ajuster le rythme sans surinterpréter la journée.",            "Gardez une action courte, observable et reliée au moment.",        ),    };
    Ok(
        json!({        "slot_code": slot_code,        "title": title,        "theme": if theme_code.is_empty() { theme } else { theme },        "tone": tone,        "text": text,        "advice": advice,        "best_for": best_for,        "watch_point": watch_point,        "evidence_keys": evidence_keys    }),
    )
}

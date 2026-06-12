use super::*;
pub fn build_interpretation_request(
    public: &HoroscopePublicRequest,
    calculation: &Value,
    signals: &[ScoredSignal],
) -> Result<Value, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
    let refs = ReferenceData::load(service_code)?;
    let shortlist = refs.shortlist.clone();
    let slot_plans = build_slot_plans(&refs, calculation, signals)?;
    let selected_keys = slot_plans
        .iter()
        .flat_map(|slot| slot.required_evidence_keys.iter())
        .cloned()
        .collect::<HashSet<_>>();
    let mut selected_signals = signals
        .iter()
        .filter(|signal| signal.priority_score >= shortlist.min_priority_score)
        .filter(|signal| selected_keys.contains(&signal.evidence_key))
        .cloned()
        .collect::<Vec<_>>();
    selected_signals.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.evidence_key.cmp(&b.evidence_key))
    });
    if selected_signals.is_empty() {
        return Err(horoscope_error("HOROSCOPE_NO_SIGNIFICANT_SIGNAL"));
    }
    let mut main_signals = selected_signals.clone();
    main_signals.truncate(shortlist.max_main_signals);
    let evidence = selected_signals
        .iter()
        .take(shortlist.max_evidence)
        .map(|signal| serde_json::to_value(signal).expect("signal serializes"))
        .collect::<Vec<_>>();
    build_interpretation_request_from_signals(
        public,
        calculation,
        &refs,
        slot_plans,
        main_signals,
        evidence,
    )
}
pub(crate) fn build_interpretation_request_from_signals(
    public: &HoroscopePublicRequest,
    calculation: &Value,
    refs: &ReferenceData,
    slot_plans: Vec<SlotInterpretationPlan>,
    main_signals: Vec<ScoredSignal>,
    evidence: Vec<Value>,
) -> Result<Value, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
    let shortlist = refs.shortlist.clone();
    let dominant_themes = aggregate_themes(&main_signals)
        .into_iter()
        .take(shortlist.max_dominant_themes)
        .collect::<Vec<_>>();
    let overview_evidence = main_signals
        .iter()
        .take(3)
        .map(|signal| signal.evidence_key.clone())
        .collect::<Vec<_>>();
    let top = main_signals.first();
    let request = json!({        "contract_version": "horoscope_interpretation_request_v1",        "service_code": service_code,        "period": premium_period(public, service_code, calculation),        "target_language": public.target_language,        "day_overview": {            "dominant_theme": top.map(|signal| signal.theme_code.as_str()).unwrap_or("daily_focus"),            "tone": top.map(|signal| signal.tone.as_str()).unwrap_or("mixed"),            "intensity": top.map(|signal| signal.intensity.as_str()).unwrap_or("medium"),            "summary_hint": "Introduire la tonalite generale sans recopier ce texte dans chaque slot.",            "evidence_keys": overview_evidence        },        "slots": slot_plans,        "main_signals": main_signals,        "dominant_themes": dominant_themes,        "evidence": evidence    });
    let request = if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        let mut request = request;
        request["best_slots"] = json!(build_best_slots(&request));
        request["watch_slots"] = json!(build_watch_slots(&request));
        request["domain_sections"] = json!(build_domain_sections(&request));
        request
    } else {
        request
    };
    validate_interpretation_request_schema(&request)?;
    Ok(request)
}

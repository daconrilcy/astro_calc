use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::french_typography::french_elision_violations;

use astral_llm_domain::{GenerationError, GenerationErrorCode};
use chrono::NaiveDate;
use chrono_tz::Tz;
use jsonschema::JSONSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";
pub const HOROSCOPE_FREE_DAILY_SERVICE_CODE: &str = "horoscope_free_daily";
pub const HOROSCOPE_SERVICE_CODE: &str = HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE;

const TIME_SLOTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_time_slot_profiles.json");
const OBJECT_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_transiting_object_weights.json");
const TARGET_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_natal_target_weights.json");
const ASPECT_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_aspect_weights.json");
const ORB_BANDS_JSON: &str = include_str!("../../../../../json_db/horoscope_orb_weight_bands.json");
const THEME_MAPPINGS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_signal_theme_mappings.json");
const THEME_ADVICE_AXES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_theme_advice_axes.json");
const SHORTLIST_JSON: &str =
    include_str!("../../../../../json_db/horoscope_shortlist_profiles.json");
const INTENSITY_JSON: &str = include_str!("../../../../../json_db/horoscope_intensity_bands.json");
const DURATION_CLASSES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_duration_classes.json");
const SCORING_PARAMETERS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_scoring_parameters.json");
const SUPPORTED_OBJECTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_supported_objects.json");
const SUPPORTED_ASPECTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_supported_aspects.json");
const INTERPRETATION_REQUEST_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_interpretation_request_v1.schema.json");
const RESPONSE_SCHEMA_JSON: &str =
    include_str!("../../../../../contracts/llm/horoscope_response_v1.schema.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopePublicRequest {
    pub date: String,
    pub timezone: String,
    pub target_language: String,
    pub chart_calculation_id: String,
    #[serde(default = "default_audience")]
    pub audience_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotProfile {
    pub service_code: String,
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
    pub slot_label: Option<String>,
    pub is_public: Option<bool>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoredSignal {
    pub evidence_key: String,
    pub fact_type: String,
    pub slot_id: String,
    pub source: String,
    pub transiting_object: String,
    pub natal_target: Option<String>,
    pub aspect: Option<String>,
    pub orb_deg: Option<f64>,
    pub natal_house: Option<i64>,
    pub theme_code: String,
    pub priority_score: f64,
    pub intensity: String,
    pub tone: String,
    pub duration_class: String,
    pub confidence_score: f64,
    pub human_label: String,
    pub score_breakdown: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotInterpretationPlan {
    pub slot_code: String,
    pub slot_label: String,
    pub specificity: String,
    pub theme_code: Option<String>,
    pub tone: Option<String>,
    pub intensity: Option<String>,
    pub main_signal_keys: Vec<String>,
    pub required_evidence_keys: Vec<String>,
    pub advice_axis: Option<String>,
    pub avoid_axis: Option<String>,
    pub watch_point: Option<String>,
    pub best_for: Vec<String>,
    pub fallback_reason: Option<String>,
}

pub struct HoroscopeBasicDailyNatalOrchestrator;
pub struct HoroscopeFreeDailyOrchestrator;
pub struct HoroscopeDailyNatalOrchestrator;

impl HoroscopeBasicDailyNatalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
            calculator,
            payload,
        )
        .await
    }
}

impl HoroscopeFreeDailyOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_FREE_DAILY_SERVICE_CODE,
            calculator,
            payload,
        )
        .await
    }
}

impl HoroscopeDailyNatalOrchestrator {
    pub async fn execute(
        service_code: &str,
        calculator: &astral_llm_infra::CalculatorClient,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        let public = validate_public_request(payload)?;
        let calculation_request = build_calculation_request_for_service(service_code, &public)?;
        let calculation = calculator
            .calculate_horoscope_daily_natal(&calculation_request)
            .await
            .map_err(|err| {
                GenerationError::with_details(
                    GenerationErrorCode::ProviderUnavailable,
                    format!("HOROSCOPE_CALCULATOR_UNAVAILABLE: {}", err.detail().message),
                    Value::Null,
                )
            })?;

        let signals = score_calculation(&calculation)?;
        let interpretation = build_interpretation_request(&public, &calculation, &signals)?;
        let response = fake_writer_response(&interpretation)?;
        validate_horoscope_response_schema(&response)?;
        validate_response_evidence(&interpretation, &response)?;

        Ok(json!({
            "calculation": calculation,
            "interpretation_request": interpretation,
            "reading": response
        }))
    }
}

pub fn validate_public_request(payload: &Value) -> Result<HoroscopePublicRequest, GenerationError> {
    let request: HoroscopePublicRequest =
        serde_json::from_value(payload.clone()).map_err(|err| {
            GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("HOROSCOPE_PAYLOAD_INVALID: {err}"),
                Value::Null,
            )
        })?;
    if request.chart_calculation_id.trim().is_empty() {
        return Err(horoscope_error("HOROSCOPE_NATAL_CHART_REQUIRED"));
    }
    NaiveDate::parse_from_str(&request.date, "%Y-%m-%d").map_err(|_| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PAYLOAD_INVALID: date must be YYYY-MM-DD",
            Value::Null,
        )
    })?;
    if request.timezone.parse::<Tz>().is_err() {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PAYLOAD_INVALID: timezone must be an IANA timezone",
            Value::Null,
        ));
    }
    Ok(request)
}

pub fn build_calculation_request(
    request: &HoroscopePublicRequest,
) -> Result<serde_json::Value, GenerationError> {
    build_calculation_request_for_service(HOROSCOPE_SERVICE_CODE, request)
}

pub fn build_calculation_request_for_service(
    service_code: &str,
    request: &HoroscopePublicRequest,
) -> Result<serde_json::Value, GenerationError> {
    validate_supported_service_code(service_code)?;
    let slots = slot_profiles(service_code)?;
    Ok(json!({
        "contract_version": "horoscope_calculation_request_v1",
        "service_code": service_code,
        "chart_calculation_id": request.chart_calculation_id,
        "period": {
            "date": request.date,
            "timezone": request.timezone
        },
        "slots": slots.into_iter().map(|slot| json!({
            "slot_code": slot.slot_code,
            "start_local_time": slot.start_local_time,
            "end_local_time": slot.end_local_time,
            "reference_local_time": slot.reference_local_time
        })).collect::<Vec<_>>()
    }))
}

pub fn score_calculation(calculation: &Value) -> Result<Vec<ScoredSignal>, GenerationError> {
    let service_code = service_code_from_value(calculation)?;
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
    let mut filtered = signals
        .iter()
        .filter(|signal| signal.priority_score >= shortlist.min_priority_score)
        .filter(|signal| selected_keys.contains(&signal.evidence_key))
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.evidence_key.cmp(&b.evidence_key))
    });
    filtered.truncate(shortlist.max_main_signals);
    let main_signals = if filtered.is_empty() {
        return Err(horoscope_error("HOROSCOPE_NO_SIGNIFICANT_SIGNAL"));
    } else {
        filtered
    };
    let evidence = main_signals
        .iter()
        .take(shortlist.max_evidence)
        .map(|signal| serde_json::to_value(signal).expect("signal serializes"))
        .collect::<Vec<_>>();
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
    let request = json!({
        "contract_version": "horoscope_interpretation_request_v1",
        "service_code": service_code,
        "period": calculation.get("period").cloned().unwrap_or_else(|| json!({
            "date": public.date,
            "timezone": public.timezone
        })),
        "target_language": public.target_language,
        "day_overview": {
            "dominant_theme": top.map(|signal| signal.theme_code.as_str()).unwrap_or("daily_focus"),
            "tone": top.map(|signal| signal.tone.as_str()).unwrap_or("mixed"),
            "intensity": top.map(|signal| signal.intensity.as_str()).unwrap_or("medium"),
            "summary_hint": "Introduire la tonalite generale sans recopier ce texte dans chaque slot.",
            "evidence_keys": overview_evidence
        },
        "slots": slot_plans,
        "main_signals": main_signals,
        "dominant_themes": dominant_themes,
        "evidence": evidence
    });
    validate_interpretation_request_schema(&request)?;
    Ok(request)
}

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

fn fake_writer_response(request: &Value) -> Result<Value, GenerationError> {
    let service_code = request
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        return fake_writer_free_response(request);
    }
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let rendered_slots = slots
        .iter()
        .map(render_fake_slot)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Une journée à ajuster avec précision",
            "text": "La journée avance en trois temps distincts : organiser le cadre, ralentir les réactions, puis rouvrir une parole plus souple. Les preuves astrologiques retenues dessinent une progression concrète sans transformer le climat du jour en promesse événementielle."
        },
        "slots": rendered_slots,
        "watch_points": ["Réactivité émotionnelle en milieu de journée"],
        "opportunities": ["Conversation plus fluide en fin de journée"],
        "evidence_summary": evidence.iter().map(|item| json!({
            "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
            "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
        })).collect::<Vec<_>>(),
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed",
            "evidence_coverage": 1.0,
            "slot_diversity_passed": true,
            "french_typography_passed": true,
            "generic_language_passed": true
        }
    }))
}

fn validate_free_response_evidence(
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

fn fake_writer_free_response(request: &Value) -> Result<Value, GenerationError> {
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let slot = request
        .get("slots")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence_keys = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Votre tendance du jour",
            "text": "La Lune met l'accent sur l'organisation, les priorités simples et les gestes utiles. La journée gagne à rester concrète : choisir une tâche mesurable, clarifier ce qui doit vraiment avancer, puis éviter de multiplier les intentions. Cette lecture reste volontairement synthétique, avec une preuve astrologique centrale plutôt qu'un découpage horaire."
        },
        "advice": "Choisissez une action vérifiable et avancez étape par étape.",
        "watch_point": "Ne cherchez pas à tout régler en même temps.",
        "evidence_keys": evidence_keys,
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed",
            "evidence_coverage": 1.0,
            "slot_diversity_passed": "not_applicable",
            "french_typography_passed": true,
            "generic_language_passed": true
        }
    }))
}

fn score_fact(
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
        score_breakdown: json!({
            "transiting_object_weight": object_weight,
            "natal_target_weight": target_weight,
            "aspect_weight": aspect_weight,
            "orb_weight": orb_weight,
            "house_weight": refs.scoring.default_house_weight,
            "theme_repetition_bonus": 0.0,
            "exactness_bonus": exactness_bonus,
            "weak_signal_penalty": 0.0
        }),
    })
}

fn build_slot_plans(
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
    } else {
        3
    };
    if plans.len() != expected {
        return Err(horoscope_error("HOROSCOPE_CALCULATION_FAILED"));
    }
    Ok(plans)
}

fn render_fake_slot(slot: &Value) -> Result<Value, GenerationError> {
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
    let watch_point = slot
        .get("watch_point")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let (theme, text, advice) = match slot_code {
        "morning" => (
            "Organisation",
            "La Lune met l'accent sur l'organisation et les gestes utiles. C'est un bon moment pour clarifier une priorité concrète, ranger une tâche ou reprendre un point simple sans ouvrir trop de sujets à la fois.",
            "Choisissez une action vérifiable et terminez-la avant de passer à la suivante.",
        ),
        "afternoon" => (
            "Limites émotionnelles",
            "Un contact tendu entre Mars et la Lune natale peut rendre l'après-midi plus réactif. Ce créneau demande de ralentir les réponses, surtout si une discussion devient imprécise ou chargée.",
            "Si une tension monte, reformulez d'abord ce que vous avez compris avant de répondre.",
        ),
        "evening" => (
            "Dialogue",
            "Vénus soutient Mercure natal et adoucit le climat relationnel du soir. L'enjeu n'est pas de tout résoudre, mais de rouvrir un échange simple, concret et moins défensif.",
            "Revenez sur un point précis plutôt que sur toute l'histoire.",
        ),
        _ => (
            "Repère du jour",
            "Le climat astrologique du slot donne un repère simple pour ajuster le rythme sans surinterpréter la journée.",
            "Gardez une action courte, observable et reliée au moment.",
        ),
    };
    Ok(json!({
        "slot_code": slot_code,
        "title": title,
        "theme": if theme_code.is_empty() { theme } else { theme },
        "tone": tone,
        "text": text,
        "advice": advice,
        "best_for": best_for,
        "watch_point": watch_point,
        "evidence_keys": evidence_keys
    }))
}

#[derive(Clone)]
struct ReferenceData {
    service_code: String,
    slot_profiles: Vec<SlotProfile>,
    object_weights: HashMap<String, f64>,
    target_weights: HashMap<String, f64>,
    aspect_weights: HashMap<String, f64>,
    aspect_tones: HashMap<String, String>,
    supported_objects: HashSet<String>,
    supported_aspects: HashSet<String>,
    orb_bands: Vec<OrbBand>,
    theme_mappings: Vec<ThemeMapping>,
    intensity_bands: Vec<IntensityBand>,
    duration_classes: HashSet<String>,
    advice_axes: HashMap<String, ThemeAdviceAxis>,
    scoring: ScoringParameters,
    shortlist: ShortlistProfile,
}

#[derive(Clone)]
struct OrbBand {
    min: f64,
    max: f64,
    weight: f64,
}

#[derive(Clone)]
struct IntensityBand {
    code: String,
    min: f64,
    max: f64,
}

#[derive(Clone)]
struct ThemeMapping {
    object: String,
    aspect: Option<String>,
    target: Option<String>,
    theme: String,
}

#[derive(Clone)]
struct ShortlistProfile {
    max_main_signals: usize,
    max_main_signals_per_slot: usize,
    max_dominant_themes: usize,
    max_evidence: usize,
    max_required_evidence_per_slot: usize,
    min_priority_score: f64,
}

#[derive(Clone)]
struct ThemeAdviceAxis {
    advice_axis: String,
    avoid_axis: Option<String>,
    best_for: Vec<String>,
    watch_point: Option<String>,
    tone_hint: Option<String>,
}

#[derive(Clone)]
struct ScoringParameters {
    exact_orb_bonus_max_deg: f64,
    exactness_bonus: f64,
    default_house_weight: f64,
    default_confidence_score: f64,
    default_duration_class: String,
    unknown_orb_weight: f64,
}

impl ReferenceData {
    fn load(service_code: &str) -> Result<Self, GenerationError> {
        validate_supported_service_code(service_code)?;
        let aspect_rows = rows(ASPECT_WEIGHTS_JSON)?;
        let refs = Self {
            service_code: service_code.to_string(),
            slot_profiles: slot_profiles(service_code)?,
            object_weights: weight_map(OBJECT_WEIGHTS_JSON, "object_code")?,
            target_weights: weight_map(TARGET_WEIGHTS_JSON, "target_code")?,
            supported_objects: enabled_codes(SUPPORTED_OBJECTS_JSON, "object_code")?,
            supported_aspects: enabled_codes(SUPPORTED_ASPECTS_JSON, "aspect_code")?,
            aspect_weights: aspect_rows
                .iter()
                .filter_map(|row| {
                    Some((
                        row.get("aspect_code")?.as_str()?.to_string(),
                        row.get("weight")?.as_f64()?,
                    ))
                })
                .collect(),
            aspect_tones: aspect_rows
                .iter()
                .filter_map(|row| {
                    Some((
                        row.get("aspect_code")?.as_str()?.to_string(),
                        row.get("tone")?.as_str()?.to_string(),
                    ))
                })
                .collect(),
            orb_bands: rows(ORB_BANDS_JSON)?
                .into_iter()
                .filter_map(|row| {
                    Some(OrbBand {
                        min: row.get("min_orb_deg")?.as_f64()?,
                        max: row.get("max_orb_deg")?.as_f64()?,
                        weight: row.get("weight")?.as_f64()?,
                    })
                })
                .collect(),
            theme_mappings: rows(THEME_MAPPINGS_JSON)?
                .into_iter()
                .filter_map(|row| {
                    Some(ThemeMapping {
                        object: row.get("match_object")?.as_str()?.to_string(),
                        aspect: row
                            .get("match_aspect")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        target: row
                            .get("match_natal_target")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        theme: row.get("theme_code")?.as_str()?.to_string(),
                    })
                })
                .collect(),
            intensity_bands: rows(INTENSITY_JSON)?
                .into_iter()
                .filter_map(|row| {
                    Some(IntensityBand {
                        code: row.get("band_code")?.as_str()?.to_string(),
                        min: row.get("min_score")?.as_f64()?,
                        max: row.get("max_score")?.as_f64()?,
                    })
                })
                .collect(),
            duration_classes: rows(DURATION_CLASSES_JSON)?
                .into_iter()
                .filter_map(|row| row.get("duration_class")?.as_str().map(str::to_string))
                .collect(),
            advice_axes: advice_axes()?,
            scoring: scoring_parameters(service_code)?,
            shortlist: shortlist_profile(service_code)?,
        };
        refs.validate()?;
        Ok(refs)
    }

    fn weight(&self, map: &HashMap<String, f64>, key: &str) -> f64 {
        map.get(key).copied().unwrap_or(1.0)
    }

    fn orb_weight(&self, orb: f64) -> f64 {
        self.orb_bands
            .iter()
            .find(|band| orb >= band.min && orb <= band.max)
            .map(|band| band.weight)
            .unwrap_or(self.scoring.unknown_orb_weight)
    }

    fn intensity(&self, score: f64) -> String {
        self.intensity_bands
            .iter()
            .find(|band| score >= band.min && score < band.max)
            .map(|band| band.code.clone())
            .unwrap_or_else(|| "medium".into())
    }

    fn theme_for(&self, object: &str, aspect: Option<&str>, target: Option<&str>) -> String {
        self.theme_mappings
            .iter()
            .find(|mapping| {
                mapping.object == object
                    && mapping.aspect.as_deref() == aspect
                    && mapping.target.as_deref() == target
            })
            .or_else(|| {
                self.theme_mappings.iter().find(|mapping| {
                    mapping.object == object
                        && mapping.aspect.is_none()
                        && mapping.target.as_deref() == target
                })
            })
            .map(|mapping| mapping.theme.clone())
            .unwrap_or_else(|| "daily_focus".into())
    }

    fn advice_axis(&self, theme_code: &str) -> ThemeAdviceAxis {
        self.advice_axes
            .get(theme_code)
            .cloned()
            .unwrap_or_else(|| ThemeAdviceAxis {
                advice_axis: "observe_before_acting".into(),
                avoid_axis: Some("overgeneralizing_the_day".into()),
                best_for: vec!["orientation".into()],
                watch_point: Some("avoid_turning_a_small_signal_into_a_prediction".into()),
                tone_hint: Some("measured".into()),
            })
    }

    fn validate(&self) -> Result<(), GenerationError> {
        if !self
            .duration_classes
            .contains(&self.scoring.default_duration_class)
        {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
        Ok(())
    }

    fn slot_label(&self, slot_code: &str) -> String {
        self.slot_profiles
            .iter()
            .find(|slot| slot.slot_code == slot_code)
            .and_then(|slot| slot.slot_label.clone())
            .unwrap_or_else(|| slot_label(slot_code).to_string())
    }
}

fn slot_profiles(service_code: &str) -> Result<Vec<SlotProfile>, GenerationError> {
    let mut slots = rows(TIME_SLOTS_JSON)?
        .into_iter()
        .filter(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .map(serde_json::from_value)
        .collect::<Result<Vec<SlotProfile>, _>>()
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    slots.sort_by_key(|slot| slot.sort_order);
    Ok(slots)
}

fn shortlist_profile(service_code: &str) -> Result<ShortlistProfile, GenerationError> {
    let row = rows(SHORTLIST_JSON)?
        .into_iter()
        .find(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?;
    Ok(ShortlistProfile {
        max_main_signals: row
            .get("max_main_signals")
            .and_then(|v| v.as_u64())
            .unwrap_or(6) as usize,
        max_main_signals_per_slot: row
            .get("max_main_signals_per_slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize,
        max_dominant_themes: row
            .get("max_dominant_themes")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as usize,
        max_evidence: row
            .get("max_evidence")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize,
        max_required_evidence_per_slot: row
            .get("max_required_evidence_per_slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize,
        min_priority_score: row
            .get("min_priority_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5),
    })
}

fn advice_axes() -> Result<HashMap<String, ThemeAdviceAxis>, GenerationError> {
    Ok(rows(THEME_ADVICE_AXES_JSON)?
        .into_iter()
        .filter_map(|row| {
            let theme_code = row.get("theme_code")?.as_str()?.to_string();
            let best_for = row
                .get("best_for")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .map(str::to_string)
                .collect::<Vec<_>>();
            Some((
                theme_code,
                ThemeAdviceAxis {
                    advice_axis: row.get("advice_axis")?.as_str()?.to_string(),
                    avoid_axis: row
                        .get("avoid_axis")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    best_for,
                    watch_point: row
                        .get("watch_point")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    tone_hint: row
                        .get("tone_hint")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                },
            ))
        })
        .collect())
}

fn scoring_parameters(service_code: &str) -> Result<ScoringParameters, GenerationError> {
    let scoring_rows = rows(SCORING_PARAMETERS_JSON)?;
    let row = scoring_rows
        .iter()
        .find(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .cloned()
        .or_else(|| {
            scoring_rows.into_iter().find(|row| {
                row.get("service_code").and_then(|v| v.as_str())
                    == Some(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
            })
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?;
    Ok(ScoringParameters {
        exact_orb_bonus_max_deg: row
            .get("exact_orb_bonus_max_deg")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        exactness_bonus: row
            .get("exactness_bonus")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        default_house_weight: row
            .get("default_house_weight")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        default_confidence_score: row
            .get("default_confidence_score")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
        default_duration_class: row
            .get("default_duration_class")
            .and_then(|v| v.as_str())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?
            .to_string(),
        unknown_orb_weight: row
            .get("unknown_orb_weight")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?,
    })
}

fn weight_map(raw: &str, key: &str) -> Result<HashMap<String, f64>, GenerationError> {
    Ok(rows(raw)?
        .into_iter()
        .filter_map(|row| {
            Some((
                row.get(key)?.as_str()?.to_string(),
                row.get("weight")?.as_f64()?,
            ))
        })
        .collect())
}

fn enabled_codes(raw: &str, key: &str) -> Result<HashSet<String>, GenerationError> {
    Ok(rows(raw)?
        .into_iter()
        .filter(|row| row.get("is_enabled_v1").and_then(|v| v.as_bool()) == Some(true))
        .filter_map(|row| row.get(key)?.as_str().map(str::to_string))
        .collect())
}

fn validate_slot_specificity(slot: &Value) -> Result<(), GenerationError> {
    let specificity = slot
        .get("specificity")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let required = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let fallback_reason = slot.get("fallback_reason").and_then(|v| v.as_str());
    match specificity {
        "specific" => {
            if required.is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_EVIDENCE_MISSING",
                    json!({ "reason": "specific_without_required_evidence" }),
                ));
            }
        }
        "shared" => {
            if required.is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_EVIDENCE_MISSING",
                    json!({ "reason": "shared_without_required_evidence" }),
                ));
            }
            let has_differentiator = ["tone", "intensity", "advice_axis", "watch_point"]
                .iter()
                .any(|key| slot.get(*key).and_then(|v| v.as_str()).is_some())
                || slot
                    .get("best_for")
                    .and_then(|v| v.as_array())
                    .map(|items| !items.is_empty())
                    .unwrap_or(false);
            if !has_differentiator {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_THEME_DUPLICATED",
                    json!({ "reason": "shared_without_differentiator" }),
                ));
            }
        }
        "fallback" => {
            if !required.is_empty() || fallback_reason.unwrap_or("").trim().is_empty() {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_FALLBACK_INVALID",
                    json!({ "reason": "fallback_requires_empty_evidence_and_reason" }),
                ));
            }
        }
        _ => return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID")),
    }
    Ok(())
}

fn validate_slot_evidence_alignment(
    request_slot: &Value,
    response_keys: &[Value],
) -> Result<(), GenerationError> {
    let required = request_slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .collect::<HashSet<_>>();
    let specificity = request_slot
        .get("specificity")
        .and_then(|v| v.as_str())
        .unwrap_or("specific");
    if specificity != "fallback" {
        for key in response_keys.iter().filter_map(|v| v.as_str()) {
            if !required.contains(key) {
                return Err(quality_error(
                    "HOROSCOPE_EVIDENCE_MISMATCH",
                    json!({ "reason": "slot_uses_unplanned_evidence", "evidence_key": key }),
                ));
            }
        }
    }
    Ok(())
}

fn validate_public_slot_text(slot: &Value) -> Result<(), GenerationError> {
    let mut public_text = String::new();
    for key in ["title", "theme", "tone", "text", "advice", "watch_point"] {
        if let Some(value) = slot.get(key).and_then(|v| v.as_str()) {
            public_text.push_str(value);
            public_text.push('\n');
        }
    }
    for forbidden in [
        "[morning]",
        "[afternoon]",
        "[evening]",
        "[day]",
        "slot:morning",
        "slot:afternoon",
        "slot:evening",
        "slot:day",
    ] {
        if public_text.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    for generic in [
        "les signaux du jour invitent",
        "rester concret et nuance",
        "l'elan du moment",
        "l’énergie du moment",
    ] {
        if public_text.to_lowercase().contains(generic) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_TOO_GENERIC",
                json!({ "forbidden": generic }),
            ));
        }
    }
    if public_text.contains("Apres-midi")
        || public_text.contains("Repondez")
        || public_text.contains("Conseil:")
        || !french_elision_violations(&public_text).is_empty()
    {
        return Err(quality_error(
            "HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "known_french_typography_violation" }),
        ));
    }
    Ok(())
}

fn validate_public_text_no_technical_codes(public_text: &str) -> Result<(), GenerationError> {
    let lower = public_text.to_lowercase();
    for forbidden in [
        "[morning]",
        "[afternoon]",
        "[evening]",
        "[day]",
        "slot:morning",
        "slot:afternoon",
        "slot:evening",
        "slot:day",
        "slot technique",
        "slot_code",
    ] {
        if lower.contains(forbidden) {
            return Err(quality_error(
                "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
                json!({ "forbidden": forbidden }),
            ));
        }
    }
    if normalized_text(public_text)
        .split_whitespace()
        .any(|token| token == "day")
    {
        return Err(quality_error(
            "HOROSCOPE_PUBLIC_SLOT_CODE_LEAK",
            json!({ "forbidden": "day" }),
        ));
    }
    Ok(())
}

fn validate_free_text_quality(public_text: &str, response: &Value) -> Result<(), GenerationError> {
    for key in ["advice", "watch_point"] {
        if response
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(quality_error(
                "HOROSCOPE_RESPONSE_INVALID",
                json!({ "reason": format!("missing_{key}") }),
            ));
        }
    }
    validate_public_text_no_technical_codes(public_text)?;
    let word_count = public_text.split_whitespace().count();
    if !(40..=190).contains(&word_count) {
        return Err(quality_error(
            "HOROSCOPE_FREE_LENGTH_INVALID",
            json!({ "word_count": word_count }),
        ));
    }
    for generic in [
        "les signaux du jour invitent",
        "rester concret et nuance",
        "l'elan du moment",
        "l’énergie du moment",
    ] {
        if public_text.to_lowercase().contains(generic) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_TOO_GENERIC",
                json!({ "forbidden": generic }),
            ));
        }
    }
    if public_text.contains("Conseil:")
        || public_text.contains("Repondez")
        || !french_elision_violations(public_text).is_empty()
    {
        return Err(quality_error(
            "HOROSCOPE_FRENCH_TYPOGRAPHY_FAILED",
            json!({ "reason": "known_french_typography_violation" }),
        ));
    }
    Ok(())
}

fn validate_astrological_reference(
    slot_code: &str,
    text: &str,
    request_slot: &Value,
) -> Result<(), GenerationError> {
    if request_slot.get("specificity").and_then(|v| v.as_str()) == Some("fallback") {
        return Ok(());
    }
    let lower = text.to_lowercase();
    let has_astro = [
        "lune",
        "mars",
        "vénus",
        "venus",
        "mercure",
        "aspect",
        "maison",
        "transit",
        "astrologique",
        "natal",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if has_astro {
        Ok(())
    } else {
        Err(quality_error(
            "HOROSCOPE_SLOT_ASTRO_REFERENCE_MISSING",
            json!({ "slot_code": slot_code }),
        ))
    }
}

fn validate_day_overview_not_copied(
    request: &Value,
    response_slots: &[Value],
) -> Result<(), GenerationError> {
    let overview = request
        .get("day_overview")
        .and_then(|v| v.get("summary_hint"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if overview.is_empty() {
        return Ok(());
    }
    for slot in response_slots {
        let text = slot.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if normalized_text(text).contains(&normalized_text(overview)) {
            return Err(quality_error(
                "HOROSCOPE_SLOT_REPETITION_FAILED",
                json!({ "reason": "day_overview_copied_into_slot" }),
            ));
        }
    }
    Ok(())
}

fn validate_slot_diversity(texts: &[String]) -> Result<(), GenerationError> {
    for i in 0..texts.len() {
        for j in (i + 1)..texts.len() {
            let a = meaningful_words(&texts[i]);
            let b = meaningful_words(&texts[j]);
            let shared = a.intersection(&b).count();
            let denom = a.len().min(b.len()).max(1);
            if shared as f64 / denom as f64 > 0.60 {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_REPETITION_FAILED",
                    json!({ "reason": "slot_word_overlap_too_high" }),
                ));
            }
            if first_words(&texts[i], 3) == first_words(&texts[j], 3) {
                return Err(quality_error(
                    "HOROSCOPE_SLOT_REPETITION_FAILED",
                    json!({ "reason": "same_opening_trigram" }),
                ));
            }
        }
    }
    Ok(())
}

fn validate_distinct_strings(items: &[String], code: &str) -> Result<(), GenerationError> {
    let normalized = items
        .iter()
        .map(|item| normalized_text(item))
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    let unique = normalized.iter().collect::<HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(quality_error(code, json!({ "reason": "duplicate_text" })));
    }
    Ok(())
}

fn validate_distinct_best_for(items: &[Vec<String>]) -> Result<(), GenerationError> {
    let normalized = items
        .iter()
        .map(|set| {
            let mut values = set
                .iter()
                .map(|value| normalized_text(value))
                .collect::<Vec<_>>();
            values.sort();
            values.join("|")
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let unique = normalized.iter().collect::<HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(quality_error(
            "HOROSCOPE_SLOT_THEME_DUPLICATED",
            json!({ "reason": "best_for_duplicated" }),
        ));
    }
    Ok(())
}

fn meaningful_words(text: &str) -> HashSet<String> {
    let stopwords = [
        "le", "la", "les", "un", "une", "des", "de", "du", "et", "ou", "a", "à", "ce", "c", "est",
        "sur", "pour", "plus", "dans", "avec", "sans", "du", "au", "aux", "en",
    ]
    .into_iter()
    .collect::<HashSet<_>>();
    normalized_text(text)
        .split_whitespace()
        .filter(|word| word.len() > 2 && !stopwords.contains(*word))
        .map(str::to_string)
        .collect()
}

fn first_words(text: &str, count: usize) -> Vec<String> {
    normalized_text(text)
        .split_whitespace()
        .take(count)
        .map(str::to_string)
        .collect()
}

fn normalized_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn rows(raw: &str) -> Result<Vec<Value>, GenerationError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    Ok(value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
}

pub fn validate_interpretation_request_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(
        interpretation_request_schema,
        "HOROSCOPE_RESPONSE_INVALID",
        value,
    )
}

pub fn validate_horoscope_response_schema(value: &Value) -> Result<(), GenerationError> {
    validate_schema(response_schema, "HOROSCOPE_RESPONSE_INVALID", value)
}

fn validate_schema(
    schema: fn() -> &'static JSONSchema,
    code: &str,
    value: &Value,
) -> Result<(), GenerationError> {
    schema().validate(value).map_err(|errors| {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            code,
            json!({ "errors": errors.map(|err| err.to_string()).collect::<Vec<_>>() }),
        )
    })
}

fn interpretation_request_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(INTERPRETATION_REQUEST_SCHEMA_JSON))
}

fn response_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| compile_schema(RESPONSE_SCHEMA_JSON))
}

fn compile_schema(raw: &str) -> JSONSchema {
    let schema: Value = serde_json::from_str(raw).expect("horoscope schema json is valid");
    JSONSchema::compile(&schema).expect("horoscope schema compiles")
}

fn collect_evidence_keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(items) = map.get("evidence_keys").and_then(|v| v.as_array()) {
                out.extend(items.iter().filter_map(|v| v.as_str().map(str::to_string)));
            }
            for child in map.values() {
                collect_evidence_keys(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_evidence_keys(item, out);
            }
        }
        _ => {}
    }
}

fn free_public_text(response: &Value) -> String {
    let mut out = String::new();
    if let Some(summary) = response.get("summary") {
        for key in ["title", "text"] {
            if let Some(value) = summary.get(key).and_then(|v| v.as_str()) {
                out.push_str(value);
                out.push('\n');
            }
        }
    }
    for key in ["advice", "watch_point"] {
        if let Some(value) = response.get(key).and_then(|v| v.as_str()) {
            out.push_str(value);
            out.push('\n');
        }
    }
    out
}

fn service_code_from_value(value: &Value) -> Result<&str, GenerationError> {
    let service_code = value
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))?;
    validate_supported_service_code(service_code)?;
    Ok(service_code)
}

fn validate_supported_service_code(service_code: &str) -> Result<(), GenerationError> {
    if matches!(
        service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE | HOROSCOPE_FREE_DAILY_SERVICE_CODE
    ) {
        return Ok(());
    }
    Err(horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))
}

fn fact_string(fact: &Value, key: &str) -> Result<String, GenerationError> {
    fact.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))
}

fn horoscope_error(code: &str) -> GenerationError {
    GenerationError::with_details(GenerationErrorCode::InvalidInput, code, Value::Null)
}

fn quality_error(code: &str, details: Value) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::PostSafetyValidationFailed,
        code,
        details,
    )
}

fn default_audience() -> String {
    "general".into()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn slot_label(slot_code: &str) -> &'static str {
    match slot_code {
        "morning" => "Matin",
        "afternoon" => "Après-midi",
        "evening" => "Soir",
        _ => "Moment",
    }
}

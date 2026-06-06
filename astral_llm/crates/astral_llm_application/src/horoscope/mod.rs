use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use astral_llm_domain::{GenerationError, GenerationErrorCode};
use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const HOROSCOPE_SERVICE_CODE: &str = "horoscope_basic_daily_natal_3_slots";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

pub struct HoroscopeBasicDailyNatalOrchestrator;

impl HoroscopeBasicDailyNatalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
        payload: &Value,
    ) -> Result<serde_json::Value, GenerationError> {
        let public = validate_public_request(payload)?;
        let calculation_request = build_calculation_request(&public)?;
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
        let response = fake_llm_response(&interpretation)?;
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
    let slots = slot_profiles()?;
    Ok(json!({
        "contract_version": "horoscope_calculation_request_v1",
        "service_code": HOROSCOPE_SERVICE_CODE,
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
    let refs = ReferenceData::load()?;
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
    let refs = ReferenceData::load()?;
    let shortlist = refs.shortlist;
    let filtered = signals
        .iter()
        .filter(|signal| signal.priority_score >= shortlist.min_priority_score)
        .take(shortlist.max_main_signals)
        .cloned()
        .collect::<Vec<_>>();
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
    Ok(json!({
        "contract_version": "horoscope_interpretation_request_v1",
        "service_code": HOROSCOPE_SERVICE_CODE,
        "period": calculation.get("period").cloned().unwrap_or_else(|| json!({
            "date": public.date,
            "timezone": public.timezone
        })),
        "target_language": public.target_language,
        "main_signals": main_signals,
        "dominant_themes": dominant_themes,
        "evidence": evidence
    }))
}

pub fn validate_response_evidence(
    request: &Value,
    response: &Value,
) -> Result<(), GenerationError> {
    if response.get("contract_version").and_then(|v| v.as_str())
        != Some("horoscope_response_v1")
        || response.get("service_code").and_then(|v| v.as_str()) != Some(HOROSCOPE_SERVICE_CODE)
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
    if allowed.is_empty() {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    let slots = response
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if slots.len() != 3 {
        return Err(horoscope_error("HOROSCOPE_RESPONSE_INVALID"));
    }
    for slot in slots {
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

fn fake_llm_response(request: &Value) -> Result<Value, GenerationError> {
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let by_slot = |slot: &str| {
        evidence
            .iter()
            .filter(|item| item.get("slot_id").and_then(|v| v.as_str()) == Some(slot))
            .filter_map(|item| item.get("evidence_key").and_then(|v| v.as_str()))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let slot_text = |label: &str| {
        format!(
            "{label}, les signaux du jour invitent a rester concret et nuance. La priorite est de relier l'elan du moment a ce qui peut etre ajuste sans precipitation."
        )
    };
    Ok(json!({
        "contract_version": "horoscope_response_v1",
        "service_code": HOROSCOPE_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Une journee a ajuster avec precision",
            "text": "La journee met l'accent sur les rythmes ordinaires, les reactions emotionnelles et la qualite du dialogue. Les preuves astrologiques retenues decrivent une progression simple : prendre soin du cadre le matin, poser des limites l'apres-midi, puis retrouver une parole plus souple le soir."
        },
        "slots": [
            {
                "slot_code": "morning",
                "title": "Matin",
                "text": slot_text("Le matin"),
                "advice": "Priorisez une action utile et mesurable.",
                "evidence_keys": by_slot("morning")
            },
            {
                "slot_code": "afternoon",
                "title": "Apres-midi",
                "text": slot_text("L'apres-midi"),
                "advice": "Repondez lentement si une tension monte.",
                "evidence_keys": by_slot("afternoon")
            },
            {
                "slot_code": "evening",
                "title": "Soir",
                "text": slot_text("Le soir"),
                "advice": "Rouvrez le dialogue sur un point concret.",
                "evidence_keys": by_slot("evening")
            }
        ],
        "watch_points": ["Reactivite emotionnelle en milieu de journee"],
        "opportunities": ["Conversation plus fluide en fin de journee"],
        "evidence_summary": evidence.iter().map(|item| json!({
            "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
            "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
        })).collect::<Vec<_>>(),
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed"
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

#[derive(Clone)]
struct ReferenceData {
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
    max_dominant_themes: usize,
    max_evidence: usize,
    min_priority_score: f64,
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
    fn load() -> Result<Self, GenerationError> {
        let aspect_rows = rows(ASPECT_WEIGHTS_JSON)?;
        let refs = Self {
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
            scoring: scoring_parameters()?,
            shortlist: shortlist_profile()?,
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

    fn validate(&self) -> Result<(), GenerationError> {
        if !self
            .duration_classes
            .contains(&self.scoring.default_duration_class)
        {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
        Ok(())
    }
}

fn slot_profiles() -> Result<Vec<SlotProfile>, GenerationError> {
    let mut slots = rows(TIME_SLOTS_JSON)?
        .into_iter()
        .filter(|row| {
            row.get("service_code").and_then(|v| v.as_str()) == Some(HOROSCOPE_SERVICE_CODE)
        })
        .map(serde_json::from_value)
        .collect::<Result<Vec<SlotProfile>, _>>()
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    slots.sort_by_key(|slot| slot.sort_order);
    Ok(slots)
}

fn shortlist_profile() -> Result<ShortlistProfile, GenerationError> {
    let row = rows(SHORTLIST_JSON)?
        .into_iter()
        .find(|row| {
            row.get("service_code").and_then(|v| v.as_str()) == Some(HOROSCOPE_SERVICE_CODE)
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?;
    Ok(ShortlistProfile {
        max_main_signals: row
            .get("max_main_signals")
            .and_then(|v| v.as_u64())
            .unwrap_or(6) as usize,
        max_dominant_themes: row
            .get("max_dominant_themes")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as usize,
        max_evidence: row
            .get("max_evidence")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize,
        min_priority_score: row
            .get("min_priority_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5),
    })
}

fn scoring_parameters() -> Result<ScoringParameters, GenerationError> {
    let row = rows(SCORING_PARAMETERS_JSON)?
        .into_iter()
        .find(|row| {
            row.get("service_code").and_then(|v| v.as_str()) == Some(HOROSCOPE_SERVICE_CODE)
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

fn rows(raw: &str) -> Result<Vec<Value>, GenerationError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    Ok(value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
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

fn fact_string(fact: &Value, key: &str) -> Result<String, GenerationError> {
    fact.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))
}

fn horoscope_error(code: &str) -> GenerationError {
    GenerationError::with_details(GenerationErrorCode::InvalidInput, code, Value::Null)
}

fn default_audience() -> String {
    "general".into()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

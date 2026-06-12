use super::*;
pub(crate) const SERVICES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_services.json");
pub(crate) const TIME_SLOTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_time_slot_profiles.json");
pub(crate) const OBJECT_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_transiting_object_weights.json");
pub(crate) const TARGET_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_natal_target_weights.json");
pub(crate) const ASPECT_WEIGHTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_aspect_weights.json");
pub(crate) const ORB_BANDS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_orb_weight_bands.json");
pub(crate) const TONE_LABELS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_tone_labels.json");
pub(crate) const DETAIL_PROFILES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_detail_profiles.json");
pub(crate) const NATAL_FOCUS_LABELS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_natal_focus_labels.json");
pub(crate) const PERIOD_STYLE_VARIANTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_period_style_variants.json");
pub(crate) const PERIOD_EDITORIAL_ROLES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_period_editorial_roles.json");
pub(crate) const PERIOD_EDITORIAL_ARCS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_period_editorial_arcs.json");
pub(crate) const PERIOD_PUBLIC_THEMES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_period_public_themes.json");
pub(crate) const THEME_MAPPINGS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_signal_theme_mappings.json");
pub(crate) const THEME_ADVICE_AXES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_theme_advice_axes.json");
pub(crate) const DOMAIN_SCORE_MAPPINGS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_domain_score_mappings.json");
pub(crate) const SHORTLIST_JSON: &str =
    include_str!("../../../../../json_db/horoscope_shortlist_profiles.json");
pub(crate) const INTENSITY_JSON: &str =
    include_str!("../../../../../json_db/horoscope_intensity_bands.json");
pub(crate) const DURATION_CLASSES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_duration_classes.json");
pub(crate) const SCORING_PARAMETERS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_scoring_parameters.json");
pub(crate) const SUPPORTED_OBJECTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_supported_objects.json");
pub(crate) const SUPPORTED_ASPECTS_JSON: &str =
    include_str!("../../../../../json_db/horoscope_supported_aspects.json");
pub(crate) const PERIOD_PROFILES_JSON: &str =
    include_str!("../../../../../json_db/astral_time_period_profiles.json");
pub(crate) const SCAN_PROFILES_JSON: &str =
    include_str!("../../../../../json_db/horoscope_scan_profiles.json");
pub fn public_watch_point_for_theme(theme_code: &str) -> Result<Option<String>, GenerationError> {
    if theme_code.trim().is_empty() {
        return Ok(None);
    }
    Ok(advice_axes()?
        .get(theme_code)
        .and_then(|axis| axis.public_watch_point.clone()))
}
#[derive(Clone)]
pub(crate) struct ReferenceData {
    pub(crate) service_code: String,
    pub(crate) service_profile: ServiceProfile,
    pub(crate) slot_profiles: Vec<SlotProfile>,
    pub(crate) object_weights: HashMap<String, f64>,
    pub(crate) target_weights: HashMap<String, f64>,
    pub(crate) aspect_weights: HashMap<String, f64>,
    pub(crate) aspect_tones: HashMap<String, String>,
    pub(crate) supported_objects: HashSet<String>,
    pub(crate) supported_aspects: HashSet<String>,
    pub(crate) orb_bands: Vec<OrbBand>,
    pub(crate) theme_mappings: Vec<ThemeMapping>,
    pub(crate) intensity_bands: Vec<IntensityBand>,
    pub(crate) duration_classes: HashSet<String>,
    pub(crate) advice_axes: HashMap<String, ThemeAdviceAxis>,
    pub(crate) scoring: ScoringParameters,
    pub(crate) shortlist: ShortlistProfile,
}
#[derive(Clone)]
pub(crate) struct ServiceProfile {
    pub(crate) house_system_code: Option<String>,
    pub(crate) period_profile_code: Option<String>,
    pub(crate) detail_profile_code: Option<String>,
    pub(crate) scan_profile_code: Option<String>,
    pub(crate) generation_mode: Option<String>,
}
#[derive(Clone)]
pub(crate) struct ScanProfile {
    pub(crate) granularity: String,
    pub(crate) reference_time_local: String,
    pub(crate) expected_snapshots_per_day: usize,
}
#[derive(Clone)]
pub(crate) struct OrbBand {
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) weight: f64,
}
#[derive(Clone)]
pub(crate) struct IntensityBand {
    pub(crate) code: String,
    pub(crate) min: f64,
    pub(crate) max: f64,
}
#[derive(Clone)]
pub(crate) struct ThemeMapping {
    pub(crate) object: String,
    pub(crate) aspect: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) theme: String,
}
#[derive(Clone)]
pub(crate) struct ShortlistProfile {
    pub(crate) max_main_signals: usize,
    pub(crate) max_main_signals_per_slot: usize,
    pub(crate) max_dominant_themes: usize,
    pub(crate) max_evidence: usize,
    pub(crate) max_required_evidence_per_slot: usize,
    pub(crate) min_priority_score: f64,
}
#[derive(Clone)]
pub(crate) struct ThemeAdviceAxis {
    pub(crate) advice_axis: String,
    pub(crate) avoid_axis: Option<String>,
    pub(crate) best_for: Vec<String>,
    pub(crate) watch_point: Option<String>,
    pub(crate) public_watch_point: Option<String>,
    pub(crate) tone_hint: Option<String>,
}
#[derive(Clone)]
pub(crate) struct ScoringParameters {
    pub(crate) exact_orb_bonus_max_deg: f64,
    pub(crate) exactness_bonus: f64,
    pub(crate) default_house_weight: f64,
    pub(crate) default_confidence_score: f64,
    pub(crate) default_duration_class: String,
    pub(crate) unknown_orb_weight: f64,
}
impl ReferenceData {
    pub(crate) fn load(service_code: &str) -> Result<Self, GenerationError> {
        validate_supported_service_code(service_code)?;
        let aspect_rows = rows(ASPECT_WEIGHTS_JSON)?;
        let refs = Self {
            service_code: service_code.to_string(),
            service_profile: service_profile(service_code)?,
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
    pub(crate) fn weight(&self, map: &HashMap<String, f64>, key: &str) -> f64 {
        map.get(key).copied().unwrap_or(1.0)
    }
    pub(crate) fn orb_weight(&self, orb: f64) -> f64 {
        self.orb_bands
            .iter()
            .find(|band| orb >= band.min && orb <= band.max)
            .map(|band| band.weight)
            .unwrap_or(self.scoring.unknown_orb_weight)
    }
    pub(crate) fn intensity(&self, score: f64) -> String {
        self.intensity_bands
            .iter()
            .find(|band| score >= band.min && score < band.max)
            .map(|band| band.code.clone())
            .unwrap_or_else(|| "medium".into())
    }
    pub(crate) fn theme_for(
        &self,
        object: &str,
        aspect: Option<&str>,
        target: Option<&str>,
    ) -> String {
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
    pub(crate) fn advice_axis(&self, theme_code: &str) -> ThemeAdviceAxis {
        self.advice_axes
            .get(theme_code)
            .cloned()
            .unwrap_or_else(|| ThemeAdviceAxis {
                advice_axis: "observe_before_acting".into(),
                avoid_axis: Some("overgeneralizing_the_day".into()),
                best_for: vec!["orientation".into()],
                watch_point: Some("avoid_turning_a_small_signal_into_a_prediction".into()),
                public_watch_point: Some(
                    "Évitez de transformer un signal bref en prédiction.".into(),
                ),
                tone_hint: Some("measured".into()),
            })
    }
    pub(crate) fn validate(&self) -> Result<(), GenerationError> {
        if !self
            .duration_classes
            .contains(&self.scoring.default_duration_class)
        {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
        if self.advice_axes.values().any(|axis| {
            axis.watch_point
                .as_deref()
                .is_some_and(|value| !value.is_empty())
                && axis
                    .public_watch_point
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
        }) {
            return Err(horoscope_error("HOROSCOPE_SCORING_FAILED"));
        }
        Ok(())
    }
    pub(crate) fn slot_label(&self, slot_code: &str) -> String {
        self.slot_profiles
            .iter()
            .find(|slot| slot.slot_code == slot_code)
            .and_then(|slot| slot.slot_label.clone())
            .unwrap_or_else(|| slot_label(slot_code).to_string())
    }
}
pub(crate) fn slot_profiles(service_code: &str) -> Result<Vec<SlotProfile>, GenerationError> {
    let mut slots = rows(TIME_SLOTS_JSON)?
        .into_iter()
        .filter(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .map(serde_json::from_value)
        .collect::<Result<Vec<SlotProfile>, _>>()
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    slots.sort_by_key(|slot| slot.sort_order);
    Ok(slots)
}
pub(crate) fn service_profile(service_code: &str) -> Result<ServiceProfile, GenerationError> {
    let row = rows(SERVICES_JSON)?
        .into_iter()
        .find(|row| row.get("service_code").and_then(|v| v.as_str()) == Some(service_code))
        .ok_or_else(|| horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))?;
    Ok(ServiceProfile {
        house_system_code: row
            .get("house_system_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        period_profile_code: row
            .get("period_profile_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        detail_profile_code: row
            .get("detail_profile_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        scan_profile_code: row
            .get("scan_profile_code")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        generation_mode: row
            .get("generation_mode")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}
pub(crate) fn period_service_profile(
    service_code: &str,
) -> Result<ServiceProfile, GenerationError> {
    let profile = service_profile(service_code)?;
    if profile.period_profile_code.is_none()
        || profile.detail_profile_code.is_none()
        || profile.scan_profile_code.is_none()
    {
        return Err(horoscope_error("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED"));
    }
    Ok(profile)
}
pub(crate) fn period_generation_mode(
    service_code: &str,
) -> Result<PeriodGenerationMode, GenerationError> {
    let profile = period_service_profile(service_code)?;
    PeriodGenerationMode::parse(profile.generation_mode.as_deref())
}
pub(crate) fn scan_profile(scan_profile_code: &str) -> Result<ScanProfile, GenerationError> {
    let row = rows(SCAN_PROFILES_JSON)?
        .into_iter()
        .find(|row| {
            row.get("scan_profile_code").and_then(Value::as_str) == Some(scan_profile_code)
                && row
                    .get("is_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
        })
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?;
    Ok(ScanProfile {
        granularity: row
            .get("granularity")
            .and_then(Value::as_str)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?
            .to_string(),
        reference_time_local: row
            .get("reference_time_local")
            .and_then(Value::as_str)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?
            .to_string(),
        expected_snapshots_per_day: row
            .get("expected_snapshots_per_day")
            .and_then(Value::as_u64)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))?
            as usize,
    })
}
impl ScanProfile {
    pub(crate) fn reference_times(&self) -> Result<Vec<NaiveTime>, GenerationError> {
        let times = self
            .reference_time_local
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                NaiveTime::parse_from_str(value, "%H:%M")
                    .map_err(|_| horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if times.len() != self.expected_snapshots_per_day || times.is_empty() {
            return Err(horoscope_error("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID"));
        }
        Ok(times)
    }
}
pub(crate) fn shortlist_profile(service_code: &str) -> Result<ShortlistProfile, GenerationError> {
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
pub(crate) fn advice_axes() -> Result<HashMap<String, ThemeAdviceAxis>, GenerationError> {
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
                    public_watch_point: row
                        .get("public_watch_point")
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
pub(crate) fn scoring_parameters(service_code: &str) -> Result<ScoringParameters, GenerationError> {
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
pub(crate) fn weight_map(raw: &str, key: &str) -> Result<HashMap<String, f64>, GenerationError> {
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
pub(crate) fn enabled_codes(raw: &str, key: &str) -> Result<HashSet<String>, GenerationError> {
    Ok(rows(raw)?
        .into_iter()
        .filter(|row| row.get("is_enabled_v1").and_then(|v| v.as_bool()) == Some(true))
        .filter_map(|row| row.get(key)?.as_str().map(str::to_string))
        .collect())
}
pub(crate) fn rows(raw: &str) -> Result<Vec<Value>, GenerationError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| GenerationError::new(GenerationErrorCode::InvalidInput, err.to_string()))?;
    Ok(value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
}
pub(crate) fn fact_string(fact: &Value, key: &str) -> Result<String, GenerationError> {
    fact.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| horoscope_error("HOROSCOPE_CALCULATION_FAILED"))
}
pub(crate) fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
pub(crate) fn slot_label(slot_code: &str) -> &'static str {
    match slot_code {
        "morning" => "Matin",
        "afternoon" => "Après-midi",
        "evening" => "Soir",
        _ => "Moment",
    }
}

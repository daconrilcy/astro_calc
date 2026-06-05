use astral_llm_domain::{
    astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
    engine_params::EngineParams,
    generation_request::{AudienceLevel, GenerateReadingRequest, ProductContext},
    interpretation_profile::NATAL_PROMPTER_PRODUCT,
    output_contract::{ChapterContract, GenerationMode, OutputFormat, ResponseContract},
    AstroCalculationPayload, AstrologerProfile, GenerationError, GenerationErrorCode,
};
use serde_json::Value;

pub const SIMPLIFIED_PROFILE: &str = "natal_simplified";
pub const SIMPLIFIED_PAYLOAD_CONTRACT: &str = "natal_simplified_structured_v1";
pub const SIMPLIFIED_REQUEST_CONTRACT: &str = "astro_simplified_natal_request_v1";
pub const SIMPLIFIED_CHAPTER_IDENTITY: &str = "identity";
pub const SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE: &str = "ambiguous_core_identity";
pub const SUN_SIGN_BLOCKED_CODE: &str = "sun.sign";

pub fn validate_simplified_calculation_request(value: &Value) -> Result<(), GenerationError> {
    let version = value
        .get("request_contract_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::InvalidInput,
                "request_contract_version is required",
            )
        })?;
    if version != SIMPLIFIED_REQUEST_CONTRACT {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("unsupported request_contract_version: {version}"),
            serde_json::json!({ "expected": SIMPLIFIED_REQUEST_CONTRACT }),
        ));
    }
    let date = value
        .pointer("/birth/date")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            GenerationError::new(GenerationErrorCode::InvalidInput, "birth.date is required")
        })?;
    if !date.chars().all(|c| c.is_ascii_digit() || c == '-') || date.len() != 10 {
        return Err(GenerationError::new(
            GenerationErrorCode::InvalidInput,
            "birth.date must be YYYY-MM-DD",
        ));
    }
    if let Some(location) = value.get("birth").and_then(|b| b.get("location")) {
        if location.get("latitude").and_then(|v| v.as_f64()).is_none()
            || location.get("longitude").and_then(|v| v.as_f64()).is_none()
        {
            return Err(GenerationError::new(
                GenerationErrorCode::InvalidInput,
                "birth.location requires latitude and longitude",
            ));
        }
    }
    if value
        .pointer("/birth/time")
        .and_then(|v| v.as_str())
        .is_some()
        && value
            .pointer("/birth/timezone")
            .and_then(|v| v.as_str())
            .is_none()
    {
        return Err(GenerationError::new(
            GenerationErrorCode::InvalidInput,
            "birth.time requires birth.timezone",
        ));
    }
    Ok(())
}

pub fn prompt_constraints_block(controls: &Value) -> String {
    let allowed = join_code_array(controls.get("allowed_fact_codes"));
    let basis_ids = join_code_array(controls.get("allowed_astro_basis_fact_ids"));
    let blocked = join_code_array(controls.get("blocked_interpretation_fact_codes"));
    let excluded = join_code_array(controls.get("excluded_feature_codes"));
    let profile_excluded = join_code_array(controls.get("profile_excluded_feature_codes"));
    let limitation_mentions = join_code_array(controls.get("allowed_limitation_mentions"));
    let houses_calculated = excluded.is_empty() && !profile_excluded.is_empty();

    format!(
        "SIMPLIFIED NATAL CONSTRAINTS (mandatory):\n\
         - Allowed interpretive fact codes (wording only, NOT astro_basis.fact_id): [{allowed}]\n\
         - Allowed astro_basis.fact_id values (use EXCLUSIVELY for astro_basis): [{basis_ids}]\n\
         - Never use allowed_fact_codes (e.g. mercury.sign) as astro_basis.fact_id.\n\
         - Blocked interpretive affirmations (do NOT state these as facts): [{blocked}]\n\
         - Calculation excluded (not computed): [{excluded}]\n\
         - Profile excluded (computed but not used in this simplified reading): [{profile_excluded}]\n\
         - You MAY explain limitations for: [{limitation_mentions}]\n\
         - Never affirm Ascendant, houses, sect, or house placements when profile-excluded.\n\
         - For blocked signs (e.g. sun.sign, moon.sign), explain uncertainty — do not pick one sign.\n\
         - Wording: partial / simplified / indicative reading — never \"degraded\".{}",
        if houses_calculated {
            "\n- Houses/Ascendant may be calculated but are intentionally omitted from this simplified product tier; say so plainly if relevant."
        } else {
            ""
        }
    )
}

fn join_code_array(value: Option<&Value>) -> String {
    value
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

pub fn sun_sign_blocked(controls: &Value) -> bool {
    controls
        .get("blocked_interpretation_fact_codes")
        .and_then(|v| v.as_array())
        .is_some_and(|items| items.iter().any(|v| v.as_str() == Some(SUN_SIGN_BLOCKED_CODE)))
}

pub fn resolve_simplified_chapter_code(controls: &Value) -> &'static str {
    if sun_sign_blocked(controls) {
        SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE
    } else {
        SIMPLIFIED_CHAPTER_IDENTITY
    }
}

pub fn merge_simplified_forbidden_wording(
    controls: &Value,
    base: Vec<String>,
) -> Vec<String> {
    let mut out = base;
    // Seuls les codes interpretatifs bloques (ex. moon.sign) — pas excluded_feature_codes
    // (sect/houses provoquent des faux positifs substring dans SafetyGuard : "section", etc.).
    if let Some(items) = controls
        .get("blocked_interpretation_fact_codes")
        .and_then(|v| v.as_array())
    {
        for item in items {
            if let Some(code) = item.as_str() {
                if !out.iter().any(|existing| existing == code) {
                    out.push(code.to_string());
                }
            }
        }
    }
    out
}

pub fn build_reading_request(
    calculation: &Value,
    user_language: &str,
    audience_level: AudienceLevel,
) -> Result<GenerateReadingRequest, GenerationError> {
    let payload = calculation
        .pointer("/simplified_payload/payload")
        .ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::InvalidInput,
                "calculator response missing simplified_payload.payload",
            )
        })?
        .clone();

    let mut data = payload;
    let mut forbidden_wording = Vec::new();
    let mut custom_instructions = None;
    let mut chapter_code = SIMPLIFIED_CHAPTER_IDENTITY;
    if let Some(controls) = calculation.get("llm_payload") {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("llm_controls".into(), controls.clone());
            scrub_simplified_payload_for_llm(obj, controls);
        }
        forbidden_wording = merge_simplified_forbidden_wording(controls, forbidden_wording);
        chapter_code = resolve_simplified_chapter_code(controls);
        if sun_sign_blocked(controls) {
            custom_instructions = Some(
                "Le Soleil est ambigu (sun.sign bloqué). N'affirmez aucun signe solaire. \
                 Expliquez la zone de changement possible entre signes, puis seulement les \
                 placements stables secondaires (Mercure, Vénus, Mars…) avec prudence."
                    .to_string(),
            );
        }
    }

    let chapter_title = if chapter_code == SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE {
        "Identité — Soleil ambigu"
    } else {
        "Identité"
    };

    Ok(GenerateReadingRequest {
        request_id: calculation
            .get("request_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        idempotency_key: None,
        product_context: ProductContext {
            product_code: NATAL_PROMPTER_PRODUCT.to_string(),
            interpretation_profile_code: Some(SIMPLIFIED_PROFILE.to_string()),
            user_language: user_language.to_string(),
            audience_level,
        },
        astro_result: AstroCalculationPayload {
            contract_version: SIMPLIFIED_PAYLOAD_CONTRACT.to_string(),
            chart_type: "natal".to_string(),
            data,
        },
        astrologer_profile: AstrologerProfile {
            profile_id: None,
            name: None,
            tone: ToneProfile::Warm,
            jargon_level: JargonLevel::Beginner,
            wording_style: WordingStyle::Clear,
            preferred_domains: vec![],
            forbidden_wording,
            custom_instructions,
        },
        engine: EngineParams {
            domain_count: Some(1),
            ..EngineParams::default()
        },
        response_contract: ResponseContract {
            output_schema_version: "natal_reading_v1".to_string(),
            generation_mode: GenerationMode::SinglePass,
            format: OutputFormat::StructuredJson,
            chapters: vec![ChapterContract {
                code: chapter_code.to_string(),
                title: chapter_title.to_string(),
                min_words: None,
                max_words: None,
                target_tokens: None,
                required_fields: vec![],
            }],
            global_max_tokens: None,
            include_astro_sources: false,
            include_legal_disclaimer: true,
        },
        safety_policy: None,
    })
}

fn blocked_object_codes(controls: &Value) -> Vec<String> {
    controls
        .get("blocked_interpretation_fact_codes")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|code| code.strip_suffix(".sign"))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn scrub_simplified_payload_for_llm(payload: &mut serde_json::Map<String, Value>, controls: &Value) {
    payload.remove("position_count");
    payload.remove("house_cusp_count");
    payload.remove("aspect_count");

    let blocked = blocked_object_codes(controls);
    if blocked.is_empty() {
        return;
    }

    if let Some(facts) = payload.get_mut("facts").and_then(|v| v.as_array_mut()) {
        facts.retain(|fact| {
            fact.get("object_code")
                .and_then(|v| v.as_str())
                .is_none_or(|code| !blocked.contains(&code.to_string()))
        });
    }
    if let Some(planets) = payload.get_mut("planets").and_then(|v| v.as_object_mut()) {
        for code in &blocked {
            planets.remove(code);
        }
    }
}

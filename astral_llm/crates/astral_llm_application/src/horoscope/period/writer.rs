use super::*;
async fn period_writer_response_inner(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response(request);
    }
    let resolved_run_id = run_id
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let schema = period_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: period_writer_messages(request)?,
        structured_schema: Some(schema),
        reasoning_effort: period_writer_reasoning_effort(request),
        temperature: Some(if is_premium_period_request(request) {
            0.55
        } else {
            0.35
        }),
        max_output_tokens: Some(period_writer_max_output_tokens(request)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: resolved_run_id,
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE)
                .to_string(),
            chapter_code: None,
            prompt_trace_step: Some("horoscope_period_writer".into()),
            prompt_trace_attempt: Some("primary".into()),
            prompt_family: Some("horoscope_period_writer".into()),
            prompt_version: Some("v1".into()),
        },
    };
    tracing::info!(
        service_code = %request["service_code"].as_str().unwrap_or("unknown"),
        max_output_tokens = provider_request.max_output_tokens.unwrap_or_default(),
        "horoscope period writer request"
    );
    let routed = use_case
        .router
        .generate(
            provider_request,
            defaults.provider.clone(),
            &defaults.model,
            false,
            true,
            ModelRouteContext::PrimaryReading,
        )
        .await?;
    if routed.used_provider == ProviderKind::Fake {
        return Err(quality_error(
            "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
            json!({ "provider": "fake" }),
        ));
    }
    let mut response = routed
        .response
        .parsed_json
        .or_else(|| parse_period_provider_json(&routed.response.raw_text))
        .ok_or_else(|| {
            let incomplete_reason =
                period_provider_incomplete_reason(&routed.response.provider_metadata);
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                format!(
                    "HOROSCOPE_PERIOD_RESPONSE_INVALID: provider_response_not_json raw_text_len={}",
                    routed.response.raw_text.len()
                ),
                json!({
                    "reason": "provider_response_not_json",
                    "raw_text_len": routed.response.raw_text.len(),
                    "provider_incomplete_reason": incomplete_reason
                }),
            )
        })?;
    if !response
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        response["quality"] = json!({});
    }
    response["quality"]["provider"] = json!(routed.used_provider.as_str());
    response["quality"]["model"] = json!(routed.response.model_used);
    response["quality"]["fallback_used"] = json!(routed.fallback_used);
    response = postprocess_period_provider_response(request, response);
    validate_period_provider_public_payload(&response)?;
    Ok(response)
}

pub async fn period_writer_response_with_quality_loop(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let resolved_run_id = run_id
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response(request);
    }
    persist_horoscope_run_started(
        use_case,
        &resolved_run_id,
        request["service_code"]
            .as_str()
            .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE),
        "horoscope_period_response",
        "horoscope_period_writer",
        "v1",
        &defaults.provider,
        &defaults.model,
        request,
    )
    .await;
    let started_at = std::time::Instant::now();
    let result = async {
        let mut response = period_writer_response_inner(use_case, request, Some(&resolved_run_id)).await?;
    let mut retries_used = 0_usize;
    for attempt in 0..=PERIOD_V2_QUALITY_MAX_RETRIES {
        match validate_period_response_quality_gates(request, &response) {
            Ok(()) => {
                tracing::info!(
                    service_code = %request["service_code"].as_str().unwrap_or("unknown"),
                    quality_retries_used = retries_used,
                    max_retries = PERIOD_V2_QUALITY_MAX_RETRIES,
                    "horoscope period quality loop completed"
                );
                return Ok(response);
            }
            Err(err) if attempt < PERIOD_V2_QUALITY_MAX_RETRIES => {
                retries_used += 1;
                response = period_style_editor_response(use_case, request, &response, &err, run_id)
                    .await?;
            }
            Err(err) => {
                tracing::warn!(
                    service_code = %request["service_code"].as_str().unwrap_or("unknown"),
                    quality_retries_used = retries_used,
                    max_retries = PERIOD_V2_QUALITY_MAX_RETRIES,
                    final_error = %err.detail().message,
                    "horoscope period quality loop failed"
                );
                return Err(GenerationError::with_details(
                    GenerationErrorCode::PostSafetyValidationFailed,
                    "HOROSCOPE_PERIOD_QUALITY_FAILED",
                    json!({                        "attempts": attempt + 1,                        "max_retries": PERIOD_V2_QUALITY_MAX_RETRIES,                        "issues": [period_v2_failure_issue("/", "quality_failed", "error", &err.detail().message)]                    }),
                ));
            }
        }
    }
    Ok(response)
    }
    .await;
    persist_horoscope_run_finished(
        use_case,
        &resolved_run_id,
        request["service_code"]
            .as_str()
            .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE),
        "horoscope_period_response",
        "horoscope_period_writer",
        "v1",
        &defaults.provider,
        &defaults.model,
        request,
        &result,
        started_at,
    )
    .await;
    result
}
#[doc(hidden)]
pub fn period_response_provider_schema(request: &Value) -> Result<Value, GenerationError> {
    let mut schema: Value = serde_json::from_str(PERIOD_RESPONSE_SCHEMA_JSON).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    schema.as_object_mut().map(|object| {
        object.remove("allOf");
    });
    let free = is_free_period_request(request);
    let premium = is_premium_period_request(request);
    if free {
        {
            let required = schema
                .get_mut("required")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
            *required = vec![
                json!("contract_version"),
                json!("service_code"),
                json!("period_resolution"),
                json!("summary"),
                json!("dominant_theme"),
                json!("key_days"),
                json!("advice"),
                json!("watch_summary"),
                json!("evidence_summary"),
                json!("quality"),
            ];
        }
        let properties = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
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
            properties.remove(field);
        }
        properties["advice"] = json!({ "type": "string" });
        properties["key_days"] = json!({            "type": "array",            "minItems": 1,            "maxItems": 2,            "items": { "$ref": "#/definitions/day_marker" }        });
        properties["evidence_summary"] = json!({            "type": "array",            "minItems": 1,            "maxItems": 3,            "items": { "$ref": "#/definitions/evidence_summary_item" }        });
        properties["watch_summary"] = json!({ "$ref": "#/definitions/free_watch_summary" });
        properties["summary"]["properties"]["title"]["maxLength"] = json!(40);
        properties["summary"]["properties"]["text"]["minLength"] = json!(420);
        properties["summary"]["properties"]["text"]["maxLength"] = json!(900);
        properties["advice"]["minLength"] = json!(120);
        properties["advice"]["maxLength"] = json!(360);
        properties["dominant_theme"]["properties"]["theme"]["maxLength"] = json!(40);
        properties["dominant_theme"]["properties"]["text"]["minLength"] = json!(120);
        properties["dominant_theme"]["properties"]["text"]["maxLength"] = json!(300);
        schema["definitions"]["day_marker"]["properties"]["title"]["maxLength"] = json!(40);
        schema["definitions"]["day_marker"]["properties"]["reason"]["minLength"] = json!(100);
        schema["definitions"]["day_marker"]["properties"]["reason"]["maxLength"] = json!(240);
        schema["definitions"]["day_marker"]["properties"]["fallback_reason"]["maxLength"] =
            json!(80);
        schema["definitions"]["free_watch_summary"]["properties"]["text"]["minLength"] = json!(110);
        schema["definitions"]["free_watch_summary"]["properties"]["text"]["maxLength"] = json!(300);
        schema["definitions"]["evidence_summary_item"]["properties"]["label"]["maxLength"] =
            json!(96);
    } else if premium {
        let required = schema
            .get_mut("required")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
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
            if !required.iter().any(|value| value.as_str() == Some(field)) {
                required.push(json!(field));
            }
        }
        required
            .retain(|value| !matches!(value.as_str(), Some("summary") | Some("dominant_theme")));
        let properties = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
        properties.remove("summary");
        properties.remove("dominant_theme");
    } else {
        {
            let required = schema
                .get_mut("required")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
            for field in [
                "week_overview",
                "best_days",
                "watch_days",
                "daily_timeline",
                "domain_sections",
            ] {
                if !required.iter().any(|value| value.as_str() == Some(field)) {
                    required.push(json!(field));
                }
            }
            required.retain(|value| {
                !matches!(
                    value.as_str(),
                    Some("summary")
                        | Some("dominant_theme")
                        | Some("best_windows")
                        | Some("watch_windows")
                        | Some("strategy")
                )
            });
        }
        let properties = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_RESPONSE_INVALID"))?;
        properties.remove("summary");
        properties.remove("dominant_theme");
        properties.remove("best_windows");
        properties.remove("watch_windows");
        properties.remove("strategy");
        properties["advice"] = json!({
            "type": "object",
            "required": ["main", "best_use", "avoid"],
            "additionalProperties": false,
            "properties": {
                "main": { "type": "string", "minLength": 120, "maxLength": 360 },
                "best_use": { "type": "string", "minLength": 70, "maxLength": 220 },
                "avoid": { "type": "string", "minLength": 70, "maxLength": 220 }
            }
        });
        properties["domain_sections"]["minItems"] = json!(2);
        properties["domain_sections"]["maxItems"] = json!(3);
        properties["evidence_summary"]["minItems"] = json!(1);
        properties["evidence_summary"]["maxItems"] = json!(5);
        properties["week_overview"]["properties"]["title"]["maxLength"] = json!(60);
        properties["week_overview"]["properties"]["text"]["minLength"] = json!(260);
        properties["week_overview"]["properties"]["text"]["maxLength"] = json!(620);
        properties["week_overview"]["properties"]["trajectory"]["minLength"] = json!(120);
        properties["week_overview"]["properties"]["trajectory"]["maxLength"] = json!(320);
        schema["definitions"]["day_marker"]["properties"]["title"]["maxLength"] = json!(60);
        schema["definitions"]["day_marker"]["properties"]["reason"]["minLength"] = json!(80);
        schema["definitions"]["day_marker"]["properties"]["reason"]["maxLength"] = json!(220);
        schema["definitions"]["day_marker"]["properties"]["fallback_reason"]["maxLength"] =
            json!(120);
        schema["definitions"]["timeline_day"]["properties"]["theme"]["maxLength"] = json!(40);
        schema["definitions"]["timeline_day"]["properties"]["tone"]["maxLength"] = json!(40);
        schema["definitions"]["timeline_day"]["properties"]["text"]["minLength"] = json!(180);
        schema["definitions"]["timeline_day"]["properties"]["text"]["maxLength"] = json!(380);
        schema["definitions"]["timeline_day"]["properties"]["advice"]["minLength"] = json!(70);
        schema["definitions"]["timeline_day"]["properties"]["advice"]["maxLength"] = json!(180);
        schema["definitions"]["domain_section"]["properties"]["domain"]["maxLength"] = json!(40);
        schema["definitions"]["domain_section"]["properties"]["title"]["maxLength"] = json!(70);
        schema["definitions"]["domain_section"]["properties"]["text"]["minLength"] = json!(180);
        schema["definitions"]["domain_section"]["properties"]["text"]["maxLength"] = json!(420);
        schema["definitions"]["watch_summary"]["properties"]["text"]["minLength"] = json!(100);
        schema["definitions"]["watch_summary"]["properties"]["text"]["maxLength"] = json!(320);
        schema["definitions"]["evidence_summary_item"]["properties"]["label"]["maxLength"] =
            json!(96);
    }
    Ok(crate::provider_schema_compiler::prepare_strict_json_schema(
        &schema,
    ))
}
pub(crate) fn parse_period_provider_json(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .or_else(|| {
            let trimmed = raw.trim();
            let unfenced = trimmed
                .strip_prefix("```json")
                .or_else(|| trimmed.strip_prefix("```"))
                .and_then(|value| value.strip_suffix("```"))
                .map(str::trim)
                .unwrap_or(trimmed);
            serde_json::from_str::<Value>(unfenced).ok()
        })
        .or_else(|| {
            extract_balanced_json_object(raw).and_then(|json| serde_json::from_str(&json).ok())
        })
}
pub(crate) fn period_provider_incomplete_reason(provider_metadata: &Value) -> Value {
    provider_metadata
        .pointer("/incomplete_details/reason")
        .cloned()
        .unwrap_or(Value::Null)
}
pub(crate) fn extract_balanced_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in raw[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(raw[start..start + offset + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}
pub fn period_writer_messages(request: &Value) -> Result<Vec<PromptMessage>, GenerationError> {
    if is_period_writer_request(request) {
        return period_writer_messages_from_writer_request(request);
    }
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let limits = period_word_limits_for_request(request);
    if is_free_period_request(request) {
        return Ok(vec![
            PromptMessage {
                role: PromptRole::System,
                content: format!(
                    "Tu écris un horoscope Free des 7 prochains jours en français. Retourne uniquement un JSON compact minifié conforme au schéma fourni, sans markdown ni texte autour. N'expose jamais de codes internes. Ne commente jamais le résultat, le JSON, le schéma, la validité, une erreur, un timeout, une troncature, une contrainte, le prompt ou ton propre processus: si quelque chose manque, rends simplement le meilleur JSON final possible sans méta-commentaire. Le texte public doit rester court, concret et lisible, entre {} et {} mots, sans dépasser {} mots.",
                    limits.target_min, limits.target_max, limits.hard_limit
                ),
            },
            PromptMessage {
                role: PromptRole::User,
                content: format!(
                    "Construis horoscope_period_response Free compact. Produis summary, dominant_theme, 1 à 2 key_days, advice, watch_summary et evidence_summary. Les key_days doivent rester de simples repères; les jours sensibles doivent rester lisibles sans réécriture canonique. Si watch_summary.status vaut none, garde evidence_keys vide et reste neutre. Retourne uniquement le JSON final sur une seule ligne. Requête JSON:\n{compact}"
                ),
            },
        ]);
    }
    if is_premium_period_request(request) {
        return Ok(vec![
            PromptMessage {
                role: PromptRole::System,
                content: format!(
                    "Tu écris une lecture Premium d'horoscope de période en français et tu retournes uniquement un objet JSON conforme au schéma fourni. Transforme les appuis de la requête en lecture humaine, fluide et utile, sans imposer de lexique canonique après coup. N'invente aucune preuve. Chaque evidence_key et chaque source_snapshot_key doit provenir de la requête. N'ajoute jamais de commentaire sur le résultat, le JSON, le schéma, la validité, une erreur, un timeout, une troncature, une contrainte, le prompt ou ton propre processus: s'il y a une difficulté, rends uniquement le meilleur JSON final possible. Écris dans un français naturel, précis et incarné. Le texte public doit compter entre {} et {} mots, sans dépasser {} mots.",
                    limits.target_min, limits.target_max, limits.hard_limit
                ),
            },
            PromptMessage {
                role: PromptRole::User,
                content: format!(
                    "Construis horoscope_period_response Premium pour la requête JSON fournie. Donne une vue d'ensemble, des journées différenciées, des fenêtres horaires utiles et une stratégie finale. Les titres et raisons doivent rester naturels et non mécaniques. Les days, windows et sections doivent servir la lecture, pas rejouer une taxonomie. Retourne uniquement le JSON conforme au schéma. Requête JSON:\n{compact}"
                ),
            },
        ]);
    }
    Ok(vec![        PromptMessage {            role: PromptRole::System,            content: format!(                "Tu écris une lecture Basic d'horoscope de période en français. Retourne uniquement un JSON compact minified conforme au schéma fourni: pas de markdown, pas de commentaires, pas de pretty print, pas d'indentation. N'invente aucune preuve: chaque evidence_key publique doit provenir de la requête. N'affiche jamais les codes internes, les clés de preuve, les noms techniques de transits, les theme_code anglais, ni les codes tone anglais. Ne commente jamais le résultat, le JSON, le schéma, la validité, une erreur, un timeout, une troncature, une contrainte, le prompt ou ton propre processus: s'il y a une difficulté, rends uniquement le meilleur JSON final possible sans aucun méta-commentaire. La timeline doit couvrir exactement les 7 dates, avec des formulations variées mais courtes. La lecture publique doit compter entre {} et {} mots, sans dépasser {} mots.",                limits.target_min, limits.target_max, limits.hard_limit            ),        },        PromptMessage {            role: PromptRole::User,            content: format!(                "Construis horoscope_period_response Basic pour cette requête d'interprétation. Produis week_overview synthétique, key_days/best_days/watch_days courts, watch_summary court, 7 daily_timeline en une phrase dense par jour plus un conseil bref, 2 à 3 domain_sections seulement, advice en 3 champs courts, evidence_summary limitée à 1 à 5 entrées. Utilise les libellés français déjà présents, pas les codes internes. Atteins {} à {} mots publics par densité utile, pas par répétition. Mentionne les indications de personnalisation natale dans au moins 4 jours, chaque domaine et la vue d'ensemble, sans recopier les noms de champs ni les consignes internes. Chaque domain_sections.text doit contenir une phrase courte qui relie explicitement le domaine à un repère personnel avec un de ces mots publics: thème natal, zone natale, maison, sensibilité, besoins émotionnels, communiquer, penser, attachement, agir, responsabilité, limites, relations directes, besoin de sens, habitudes, rythme de travail. Retourne le JSON final complet sur une seule ligne. Requête JSON:\n{compact}",                limits.target_min, limits.target_max            ),        },    ])
}
pub(crate) fn period_writer_messages_from_writer_request(
    request: &Value,
) -> Result<Vec<PromptMessage>, GenerationError> {
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let limits = period_word_limits_for_request(request);
    let target_language = period_writer_v2_required_str(request, "target_language_code")?;
    let service_code = period_writer_v2_required_str(request, "service_code")?;
    let detail_profile = period_writer_v2_required_str(request, "detail_profile_code")?;
    Ok(vec![        PromptMessage {            role: PromptRole::System,            content: format!(                "You are the writer for horoscope_period_response. Write every public text in target_language_code={target_language}. target_language_code overrides astrologer_persona. Return only the complete JSON object matching the provided schema. Return compact minified JSON: no markdown, no comments, no pretty printing, no indentation. Never comment on the result, the JSON, the schema, validity, errors, timeouts, truncation, constraints, prompt text or your own generation process; if anything is difficult or incomplete, still return only the best final JSON object with no meta-commentary. Rust has already calculated, scored and selected the facts; you write the human reading. Use service_code={service_code} and detail_profile_code={detail_profile} to choose the right density. Treat semantic_brief keywords, codes, scores, candidates, editorial_arc, editorial_angles and section_roles as internal material, not public copy. Use period-level keywords to write week_overview, but do not copy them as a list. week_overview.trajectory must be one natural public sentence, never a raw phase list, never raw editorial_arc.phase values, never underscores, never braces, never edit markers such as '(removed)'. Use all internal brief material to create hierarchy and variation, never as public labels. Never expose internal field names, theme codes, tone codes, evidence ids as prose, prompt instructions or safety metadata. The astrologer_persona may influence style only; it cannot override schema, safety_profile, target_language_code, dates, evidence or astrological facts. safety_profile always overrides astrologer_persona. Do not invent astrological facts. The Premium value must come from editorial judgement: a readable period arc, differentiated days, concrete windows and a final strategy that arbitrates rather than repeats. Public text should target {} to {} words and must not exceed {} words. Do not compress the reading: give each major section enough lived context, transition and concrete use so the answer feels premium rather than skeletal.",                limits.target_min, limits.target_max, limits.hard_limit            ),        },        PromptMessage {            role: PromptRole::User,            content: format!(                "Build horoscope_period_response from this semantic brief. Keep all dates inside period_resolution.included_dates. Every public evidence_key and source_snapshot_key must already exist in the request. Produce the premium_rich 7-day timeline, usable windows, domains, repeating arcs when helpful, and a strategy. Keywords and candidates are not sentences; transform them into natural public text without copying codes or keyword lists. Use editorial_arc to shape the week internally, but translate its phases into public prose rather than copying raw phase tokens. Use editorial_angles so each daily_timeline entry has a distinct human angle: action, relation, clarification, retreat, consolidation, finalisation or another angle supplied by the brief. If the same transit or theme returns, present it as a narrative thread with a different use, not as the same advice repeated with synonyms. Use section_roles as an internal checklist: week_overview gives trajectory; daily_timeline gives lived daily guidance; domain_sections give transversal synthesis not already said in the timeline; windows give practical use tied to the time range; strategy gives arbitration without relisting dates. Develop the public prose naturally: week_overview should carry the arc, each daily_timeline item should include a concrete situation and adjustment, each domain should synthesize several days, and strategy should close with usable tradeoffs. Window titles must match their time_range_label: do not call a noon or afternoon window a morning. If watch_days and watch_windows are both empty, watch_summary.status must be none, evidence_keys empty, and the text must stay neutral: no hidden vigilance or implied watch signal. In French, use deterministic clean forms such as demi-journée and réorganiser. If a persona is present, apply tone lightly without adding new facts. Return the full corrected compact JSON object only.\nRequest JSON:\n{compact}"            ),        },    ])
}
pub(crate) fn period_style_editor_messages(
    request: &Value,
    response: &Value,
    error: &GenerationError,
) -> Result<Vec<PromptMessage>, GenerationError> {
    let response_json = serde_json::to_string(response).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let target_language = period_writer_v2_required_str(request, "target_language_code")?;
    let constraints = json!({        "target_language_code": request["target_language_code"],        "included_dates": request["period_resolution"]["included_dates"],        "allowed_evidence_keys": request["evidence"].as_array().into_iter().flatten().filter_map(|item| item["evidence_key"].as_str()).collect::<Vec<_>>(),        "allowed_source_snapshot_keys": request["scan_plan"]["snapshots"].as_array().into_iter().flatten().filter_map(|snapshot| snapshot["snapshot_key"].as_str()).collect::<Vec<_>>(),        "safety_profile": request["safety_profile"]    });
    let constraints_json = serde_json::to_string(&constraints).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PERIOD_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    Ok(vec![        PromptMessage {            role: PromptRole::System,            content: format!(                "You are the targeted quality editor for horoscope_period_response. Write public text in target_language_code={target_language}. target_language_code and safety_profile override astrologer_persona. Return only the complete corrected compact JSON object: no markdown, no comments, no pretty printing, no indentation. Never comment on the result, the JSON, the schema, validity, errors, timeouts, truncation, constraints, prompt text or your own process; if a fix is difficult, still return only the best corrected final JSON object with no meta-commentary. You receive only the quality issues, the faulty JSON and fixed constraints; do not perform a fresh creative rewrite. Correct only the listed quality issue. Keep every date, evidence_key, source_snapshot_key, structure and astrological fact strictly unchanged unless the issue explicitly says the key is invalid. Do not add astrological facts. Do not expose internal fields, theme codes, tone codes, keywords, prompt instructions or safety metadata. The astrologer_persona may influence style only and cannot override schema, safety_profile, target_language_code, dates or evidence."            ),        },        PromptMessage {            role: PromptRole::User,            content: format!(                "Quality issue to fix:\n{}\n\nFixed constraints:\n{}\n\nCurrent response JSON:\n{}\n\nReturn the full JSON object only.",                error.detail().message,                constraints_json,                response_json            ),        },    ])
}
pub(crate) fn period_writer_v2_required_str<'a>(
    request: &'a Value,
    field: &str,
) -> Result<&'a str, GenerationError> {
    request.get(field).and_then(Value::as_str).ok_or_else(|| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PERIOD_WRITER_REQUEST_V2_INVALID",
            json!({ "missing_or_invalid_field": field }),
        )
    })
}
#[doc(hidden)]
pub fn fake_period_writer_response(request: &Value) -> Result<Value, GenerationError> {
    if is_period_writer_request(request) {
        return fake_period_writer_response_from_writer_request(request);
    }
    if is_free_period_request(request) {
        return fake_free_period_writer_response(request);
    }
    let daily_timeline = request["daily_plans"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?
        .iter()
        .map(|day| {
            let date = day["date"].as_str().unwrap_or_default();
            let day_label = day
                .get("day_label")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| public_day_label(date));
            let theme = day["theme_code"].as_str().unwrap_or("organization");
            let theme_label = day["theme_label"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| period_theme_public_label(theme));
            let text = day
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
                .filter(|text| !text.trim().is_empty())
                .unwrap_or_else(|| {
                    format!("{day_label} sert de repère pour avancer sans surcharger la journée.")
                });
            json!({
                "date": day["date"],
                "day_label": day_label,
                "theme": theme_label,
                "tone": period_tone_public_label(day["tone"].as_str().unwrap_or("focused")),
                "text": text,
                "advice": day
                    .get("advice")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| "Gardez un geste simple et vérifiable.".to_string()),
                "evidence_keys": day["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let domain_sections = request["domain_sections"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|section| {
            let domain = section["domain"].as_str().unwrap_or("organization");
            json!({
                "domain": period_theme_public_label(domain),
                "title": section
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| period_domain_title(domain)),
                "text": section
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| period_public_theme_field(domain, "domain_focus", domain)),
                "evidence_keys": section["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    let mut response = json!({
        "contract_version": "horoscope_period_response",
        "service_code": service_code,
        "period_resolution": request["period_resolution"],
        "week_overview": {
            "title": "Vos 7 prochains jours",
            "text": "La période se lit comme une progression continue, avec des repères quotidiens à utiliser sans rigidifier la semaine.",
            "trajectory": "Une trajectoire globale relie les jours, les appuis et les moments de prudence."
        },
        "key_days": request["key_days"],
        "best_days": request["best_days"],
        "watch_days": request["watch_days"],
        "watch_summary": request["watch_summary_plan"],
        "daily_timeline": daily_timeline,
        "domain_sections": domain_sections,
        "advice": {
            "main": "Avancez par étapes courtes et gardez une trace de ce qui évolue d'un jour à l'autre.",
            "best_use": "Réserver les appuis aux échanges utiles et aux décisions réversibles.",
            "avoid": "Transformer un signal quotidien en certitude définitive."
        },
        "evidence_summary": request["evidence"].as_array().into_iter().flatten().take(5).map(|item| json!({
            "evidence_key": item["evidence_key"],
            "date": item["date"],
            "label": item["human_label"]
        })).collect::<Vec<_>>(),
        "quality": {
            "daily_timeline_count": 7,
            "evidence_guard_passed": true,
            "best_watch_overlap_passed": true,
            "provider": "fake",
            "model": "fake-model",
            "fallback_used": false,
            "period_contract": "basic_next_7_days"
        }
    });
    if is_premium_period_service(service_code) {
        response["best_windows"] = request["best_windows"].clone();
        response["watch_windows"] = request["watch_windows"].clone();
        response["strategy"] = json!({
            "title": request["strategy"]["title"].as_str().unwrap_or("Stratégie de semaine"),
            "text": "Utilisez les créneaux utiles pour agir court et gardez de l'air dans les moments plus sensibles.",
            "best_use": request["strategy"]["best_use"].as_str().unwrap_or("Réserver les appuis aux échanges utiles."),
            "recovery": request["strategy"]["recovery"].as_str().unwrap_or("Préserver des temps de recul après les moments plus réactifs."),
            "evidence_keys": request["strategy"]["evidence_keys"]
        });
        response["quality"]["period_contract"] = json!("premium_next_7_days");
    }
    repair_period_response_shape(request, &mut response);
    Ok(response)
}
pub(crate) fn fake_period_writer_response_from_writer_request(
    request: &Value,
) -> Result<Value, GenerationError> {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    let evidence = request["evidence"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    let primary = evidence
        .first()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
    let primary_key = primary["evidence_key"].clone();
    let primary_date = primary["date"]
        .as_str()
        .or_else(|| request["period_resolution"]["included_dates"][0].as_str())
        .unwrap_or("2026-06-07");
    let primary_theme =
        period_theme_public_label(primary["theme_code"].as_str().unwrap_or("organization"));
    let key_days = day_markers_from_candidates_v2(
        request
            .pointer("/semantic_brief/key_day_candidates")
            .and_then(Value::as_array),
        primary_date,
        &primary_key,
    );
    if is_free_period_service(service_code) {
        let mut response = json!({
            "contract_version": "horoscope_period_response",
            "service_code": service_code,
            "period_resolution": request["period_resolution"],
            "summary": {
                "title": "Vos 7 prochains jours",
                "text": format!("Cette période donne une boussole générale plutôt qu'un planning détaillé. Le thème {primary_theme} ressort comme fil conducteur et aide à repérer une priorité simple sans figer chaque signal.")
            },
            "dominant_theme": {
                "theme": primary_theme,
                "text": "Le thème dominant sert de repère pour hiérarchiser les décisions sans rigidifier la lecture."
            },
            "key_days": key_days.into_iter().take(2).collect::<Vec<_>>(),
            "advice": "Gardez une seule priorité observable, puis ajustez-la si le même signal revient dans la semaine.",
            "watch_summary": {
                "status": "low",
                "text": "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation.",
                "evidence_keys": [primary_key]
            },
            "evidence_summary": evidence_summary_v2(evidence, 3),
            "quality": quality_v2(service_code, request, 0)
        });
        repair_period_response_shape_v2(request, &mut response);
        return Ok(response);
    }
    let daily_timeline = request
        .pointer("/semantic_brief/daily_signal_summary")
        .and_then(Value::as_array)
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?
        .iter()
        .map(|day| {
            let date = day["date"].as_str().unwrap_or(primary_date);
            let theme_code = day["theme_codes"]
                .as_array()
                .and_then(|items| items.first())
                .and_then(Value::as_str)
                .unwrap_or("organization");
            let tone_code = day["tone_codes"]
                .as_array()
                .and_then(|items| items.first())
                .and_then(Value::as_str)
                .unwrap_or("focused");
            let keys = day["evidence_keys"]
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![primary_key.clone()]);
            let day_label = public_day_label(date);
            json!({
                "date": date,
                "day_label": day_label,
                "theme": period_theme_public_label(theme_code),
                "tone": period_tone_public_label(tone_code),
                "text": format!("{day_label} sert de repère pour utiliser le signal sans le rigidifier."),
                "advice": "Choisissez un geste court, vérifiable et relié au contexte réel.",
                "evidence_keys": keys
            })
        })
        .collect::<Vec<_>>();
    let best_days = day_markers_from_candidates_v2(
        request
            .pointer("/semantic_brief/best_day_candidates")
            .and_then(Value::as_array),
        primary_date,
        &primary_key,
    );
    let watch_days = day_markers_from_candidates_v2(
        request
            .pointer("/semantic_brief/watch_day_candidates")
            .and_then(Value::as_array),
        primary_date,
        &primary_key,
    );
    let domain_sections = request
        .pointer("/semantic_brief/domain_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(if is_premium_period_service(service_code) {
            5
        } else {
            4
        })
        .map(|domain| {
            let code = domain["domain_code"].as_str().unwrap_or("organization");
            json!({
                "domain": period_theme_public_label(code),
                "title": period_domain_title(code),
                "text": domain
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| period_public_theme_field(code, "domain_focus", code)),
                "evidence_keys": domain["evidence_keys"]
            })
        })
        .collect::<Vec<_>>();
    let mut response = json!({
        "contract_version": "horoscope_period_response",
        "service_code": service_code,
        "period_resolution": request["period_resolution"],
        "week_overview": {
            "title": "Vos 7 prochains jours",
            "text": "La période se lit comme une progression: observer les premiers signaux, choisir une priorité concrète, puis ajuster le rythme si une tension ou une opportunité se répète.",
            "trajectory": "Le fil conducteur consiste à relier les priorités à des décisions simples et vérifiables."
        },
        "key_days": key_days,
        "best_days": best_days,
        "watch_days": watch_days,
        "watch_summary": {
            "status": "low",
            "text": "Les moments de prudence demandent surtout de vérifier les limites avant de promettre davantage.",
            "evidence_keys": [primary_key.clone()]
        },
        "daily_timeline": daily_timeline,
        "domain_sections": domain_sections,
        "advice": {
            "main": "Avancez par étapes courtes et gardez une trace de ce qui évolue d'un jour à l'autre.",
            "best_use": "Utilisez les appuis pour confirmer une décision ou finaliser une tâche concrète.",
            "avoid": "Transformer un signal quotidien en certitude définitive."
        },
        "evidence_summary": evidence_summary_v2(evidence, 5),
        "quality": quality_v2(service_code, request, 7)
    });
    if is_premium_period_service(service_code) {
        let best_windows = window_markers_from_candidates_v2(request, "best", &HashSet::new());
        let best_window_identities = best_windows
            .iter()
            .filter_map(period_window_identity)
            .collect::<HashSet<_>>();
        let watch_windows =
            window_markers_from_candidates_v2(request, "watch", &best_window_identities);
        response["best_windows"] = json!(best_windows);
        response["watch_windows"] = json!(watch_windows);
        response["strategy"] = json!({
            "title": "Stratégie de semaine",
            "text": "Utilisez les créneaux utiles pour agir court et gardez de l'air dans les moments plus sensibles.",
            "best_use": "Réserver les appuis aux échanges utiles, aux preuves concrètes et aux décisions réversibles.",
            "recovery": "Préserver des temps de recul après les moments plus réactifs.",
            "evidence_keys": [primary_key]
        });
    }
    repair_period_response_shape_v2(request, &mut response);
    Ok(response)
}
pub(crate) fn day_markers_from_candidates_v2(
    candidates: Option<&Vec<Value>>,
    fallback_date: &str,
    fallback_evidence_key: &Value,
) -> Vec<Value> {
    let mut out = candidates
        .into_iter()
        .flatten()
        .take(4)
        .map(|candidate| {
            let title = candidate
                .get("title")
                .and_then(Value::as_str)
                .filter(|title| !title.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    candidate["date"]
                        .as_str()
                        .map(public_day_label)
                        .unwrap_or_else(|| "Repère".to_string())
                });
            json!({
                "date": candidate["date"],
                "title": title,
                "reason": candidate
                    .get("reason")
                    .and_then(Value::as_str)
                    .filter(|reason| !reason.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| "Repère utile pour lire la suite de la période.".to_string()),
                "evidence_keys": candidate["evidence_keys"],
                "fallback_reason": null
            })
        })
        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push(json!({
            "date": fallback_date,
            "title": public_day_label(fallback_date),
            "reason": "Repère utile pour lire la suite de la période.",
            "evidence_keys": [fallback_evidence_key.clone()],
            "fallback_reason": null
        }));
    }
    out
}
pub(crate) fn window_markers_from_candidates_v2(
    request: &Value,
    candidate_type: &str,
    excluded_identities: &HashSet<String>,
) -> Vec<Value> {
    let all_windows = request
        .pointer("/semantic_brief/window_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut windows = all_windows
        .iter()
        .filter(|window| {
            let tone = window["tone_code"].as_str().unwrap_or("");
            if candidate_type == "watch" {
                tone == "careful"
            } else {
                tone != "careful"
            }
        })
        .filter(|window| {
            period_window_identity(window)
                .map(|identity| !excluded_identities.contains(&identity))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    if windows.is_empty() {
        windows = all_windows
            .iter()
            .filter(|window| {
                period_window_identity(window)
                    .map(|identity| !excluded_identities.contains(&identity))
                    .unwrap_or(true)
            })
            .take(1)
            .collect();
    }
    let limit = if candidate_type == "best" && windows.len() > 1 {
        2
    } else {
        3
    };
    windows
        .into_iter()
        .take(limit)
        .map(|window| {
            let date = window["date"].as_str().unwrap_or("");
            let title = format!(
                "{} {}",
                public_day_label(date),
                window["time_range_label"].as_str().unwrap_or("")
            );
            let theme = window["theme_code"]
                .as_str()
                .map(period_theme_public_label)
                .unwrap_or_else(|| "thème".to_string());
            let tone = window["tone_code"]
                .as_str()
                .map(period_tone_public_label)
                .unwrap_or_else(|| "ton".to_string());
            if candidate_type == "watch" {
                json!({
                    "date": window["date"],
                    "time_range_label": window["time_range_label"],
                    "source_snapshot_keys": window["source_snapshot_keys"],
                    "title": title,
                    "theme": theme,
                    "tone": tone,
                    "watch_point": window
                        .get("watch_point")
                        .and_then(Value::as_str)
                        .filter(|text| !text.trim().is_empty())
                        .unwrap_or("Repère à vérifier avant de répondre."),
                    "evidence_keys": window["evidence_keys"]
                })
            } else {
                json!({
                    "date": window["date"],
                    "time_range_label": window["time_range_label"],
                    "source_snapshot_keys": window["source_snapshot_keys"],
                    "title": title,
                    "theme": theme,
                    "tone": tone,
                    "reason": window
                        .get("reason")
                        .and_then(Value::as_str)
                        .filter(|text| !text.trim().is_empty())
                        .unwrap_or("Repère utile pour agir sans surcharge."),
                    "best_for": window
                        .get("best_for")
                        .cloned()
                        .unwrap_or_else(|| json!([theme])),
                    "evidence_keys": window["evidence_keys"]
                })
            }
        })
        .collect()
}
pub(crate) fn evidence_summary_v2(evidence: &[Value], limit: usize) -> Vec<Value> {
    evidence
        .iter()
        .take(limit)
        .map(|item| {
            json!({
                "evidence_key": item["evidence_key"],
                "date": item["date"],
                "label": item
                    .get("human_label")
                    .and_then(Value::as_str)
                    .filter(|label| !label.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        format!(
                            "{} / {}",
                            period_theme_public_label(item["theme_code"].as_str().unwrap_or("organization")),
                            period_tone_public_label(
                                item["tone_code"]
                                    .as_str()
                                    .or_else(|| item["tone"].as_str())
                                    .unwrap_or("focused")
                            )
                        )
                    })
            })
        })
        .collect()
}
pub(crate) fn quality_v2(service_code: &str, _request: &Value, daily_count: usize) -> Value {
    json!({        "daily_timeline_count": daily_count,        "evidence_guard_passed": true,        "best_watch_overlap_passed": true,        "provider": "fake",        "model": "fake-model",        "fallback_used": false,        "period_contract": if is_free_period_service(service_code) {            "free_next_7_days"        } else if is_premium_period_service(service_code) {            "premium_next_7_days"        } else {            "basic_next_7_days"        }    })
}
pub(crate) fn fake_free_period_writer_response(request: &Value) -> Result<Value, GenerationError> {
    let evidence = request["evidence"]
        .as_array()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING"))?;
    let primary = evidence
        .first()
        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_FREE_EVIDENCE_MISSING"))?;
    let evidence_key = primary["evidence_key"].clone();
    let date = primary["date"]
        .as_str()
        .or_else(|| request["period_resolution"]["included_dates"][0].as_str())
        .unwrap_or("2026-06-07");
    let theme = period_theme_public_label(primary["theme_code"].as_str().unwrap_or("organization"));
    let key_days = request["key_days"]
        .as_array()
        .into_iter()
        .flatten()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    let key_days = if key_days.is_empty() {
        vec![json!({
            "date": date,
            "title": public_day_label(date),
            "reason": format!("Le thème {} ressort plus nettement et donne un repère utile sans en faire un verdict.", theme),
            "evidence_keys": [evidence_key.clone()],
            "fallback_reason": null
        })]
    } else {
        key_days
    };
    Ok(
        json!({        "contract_version": "horoscope_period_response",        "service_code": HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,        "period_resolution": request["period_resolution"],        "summary": {            "title": "Vos 7 prochains jours",            "text": format!("Les prochains jours donnent surtout une tendance à comprendre plutôt qu'un planning à suivre. Autour du {date}, le climat met l'accent sur {theme} : une priorité simple, un échange à clarifier ou une routine à stabiliser peut devenir le fil conducteur. L'intérêt est de repérer ce qui demande de l'attention sans découper chaque journée ni chercher une fenêtre idéale. Gardez une marge pour ajuster votre rythme, observez les moments où les émotions accélèrent les décisions, puis revenez à une action concrète. Cette lecture reste volontairement compacte : elle sert de boussole générale pour choisir ce qui mérite d'être traité maintenant et ce qui peut attendre.")        },        "dominant_theme": {            "theme": theme,            "text": format!("Le thème dominant est {theme}. Il invite à privilégier une décision simple, reliée à vos priorités concrètes, plutôt qu'une dispersion sur plusieurs sujets.")        },        "key_days": key_days,        "advice": "Choisissez une seule priorité observable et gardez assez de souplesse pour l'ajuster. Notez ce qui se répète avant de conclure.",        "watch_summary": {            "status": "low",            "text": "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation.",            "evidence_keys": [evidence_key]        },        "evidence_summary": evidence.iter().take(3).map(|item| json!({            "evidence_key": item["evidence_key"],            "date": item["date"],            "label": item["human_label"]        })).collect::<Vec<_>>(),        "quality": {            "daily_timeline_count": 0,            "evidence_guard_passed": true,            "best_watch_overlap_passed": true,            "provider": "fake",            "model": "fake-model",            "fallback_used": false,            "period_contract": "free_next_7_days"        }    }),
    )
}

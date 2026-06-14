use super::*;

pub async fn daily_writer_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_writer_response(request);
    }

    let schema = daily_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: daily_writer_messages(request)?,
        structured_schema: Some(schema),
        reasoning_effort: Some(ReasoningEffort::Minimal),
        temperature: Some(0.4),
        max_output_tokens: Some(daily_writer_max_output_tokens(request)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: run_id
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
                .to_string(),
            chapter_code: None,
        },
    };

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
            "HOROSCOPE_DAILY_REAL_PROVIDER_REQUIRED",
            json!({ "provider": "fake" }),
        ));
    }

    let mut response = match routed
        .response
        .parsed_json
        .clone()
        .or_else(|| parse_period_provider_json(&routed.response.raw_text))
    {
        Some(response) => response,
        None => {
            daily_writer_repair_non_json_response(
                use_case,
                request,
                &routed.response.raw_text,
                run_id,
            )
            .await?
        }
    };
    if !response
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        response["quality"] = json!({});
    }
    response["quality"]["provider"] = json!(routed.used_provider.as_str());
    response["quality"]["model"] = json!(routed.response.model_used);
    response["quality"]["fallback_used"] = json!(routed.fallback_used);
    repair_daily_response_shape(request, &mut response);
    repair_premium_daily_editorial_repetition(&mut response);
    Ok(reprocess_horoscope_daily_payload(response))
}

async fn daily_writer_repair_non_json_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    raw_text: &str,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return Err(quality_error(
            "HOROSCOPE_DAILY_REAL_PROVIDER_REQUIRED",
            json!({ "provider": "fake" }),
        ));
    }

    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: daily_writer_json_repair_messages(request, raw_text)?,
        structured_schema: Some(daily_response_provider_schema(request)?),
        reasoning_effort: Some(ReasoningEffort::Minimal),
        temperature: Some(0.1),
        max_output_tokens: Some(daily_writer_max_output_tokens(request).max(4000)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: run_id
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE)
                .to_string(),
            chapter_code: Some("daily_json_repair".to_string()),
        },
    };
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
            "HOROSCOPE_DAILY_REAL_PROVIDER_REQUIRED",
            json!({ "provider": "fake" }),
        ));
    }

    let mut repaired = routed
        .response
        .parsed_json
        .or_else(|| parse_period_provider_json(&routed.response.raw_text))
        .ok_or_else(|| {
            let incomplete_reason =
                period_provider_incomplete_reason(&routed.response.provider_metadata);
            GenerationError::with_details(
                GenerationErrorCode::PostSafetyValidationFailed,
                format!(
                    "HOROSCOPE_RESPONSE_INVALID: provider_response_not_json raw_text_len={}",
                    routed.response.raw_text.len()
                ),
                json!({
                    "reason": "provider_response_not_json",
                    "raw_text_len": routed.response.raw_text.len(),
                    "provider_incomplete_reason": incomplete_reason,
                    "repair_attempted": true
                }),
            )
        })?;
    if !repaired
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        repaired["quality"] = json!({});
    }
    repaired["quality"]["provider"] = json!(routed.used_provider.as_str());
    repaired["quality"]["model"] = json!(routed.response.model_used);
    repaired["quality"]["fallback_used"] = json!(routed.fallback_used);
    repaired["quality"]["repair_attempted"] = json!(true);
    Ok(repaired)
}

pub fn daily_response_provider_schema(request: &Value) -> Result<Value, GenerationError> {
    let schema: Value = serde_json::from_str(RESPONSE_SCHEMA_JSON).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            format!("HOROSCOPE_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let service_code = request
        .get("service_code")
        .and_then(Value::as_str)
        .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE);
    let branch_index = match service_code {
        HOROSCOPE_FREE_DAILY_SERVICE_CODE => 1,
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE => 2,
        _ => 0,
    };
    let branch = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .and_then(|branches| branches.get(branch_index))
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let mut required = branch.get("required").cloned().unwrap_or_else(|| json!([]));
    if let Some(items) = required.as_array_mut() {
        items.retain(|item| item.as_str() != Some("quality"));
    }
    let mut properties = branch
        .get("properties")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if let Some(object) = properties.as_object_mut() {
        object.remove("quality");
    }
    let mut schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "horoscope_response",
        "definitions": schema.get("definitions").cloned().unwrap_or_else(|| json!({})),
        "type": "object",
        "required": required,
        "additionalProperties": false,
        "properties": properties
    });
    if branch_index == 0 {
        schema["properties"]["watch_points"] = json!({
            "type": "array",
            "items": { "type": "string" }
        });
        schema["properties"]["opportunities"] = json!({
            "type": "array",
            "items": { "type": "string" }
        });
        schema["properties"]["evidence_summary"] = json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["evidence_key", "theme_code"],
                "additionalProperties": false,
                "properties": {
                    "evidence_key": { "type": "string" },
                    "theme_code": { "type": "string" }
                }
            }
        });
    }
    if branch_index == 2 {
        schema["properties"]["advice"]["properties"]["main"]["maxLength"] = json!(180);
        schema["properties"]["advice"]["properties"]["best_use"]["maxLength"] = json!(160);
        schema["properties"]["advice"]["properties"]["avoid"]["maxLength"] = json!(160);
        schema["definitions"]["summary"]["properties"]["title"]["maxLength"] = json!(40);
        schema["definitions"]["summary"]["properties"]["text"]["maxLength"] = json!(220);
        schema["properties"]["evidence_summary"] = json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["evidence_key", "theme_code"],
                "additionalProperties": false,
                "properties": {
                    "evidence_key": { "type": "string" },
                    "theme_code": { "type": "string" }
                }
            }
        });
        schema["definitions"]["premium_slot_summary"]["properties"]["title"]["maxLength"] =
            json!(40);
        schema["definitions"]["premium_slot_summary"]["properties"]["reason"]["maxLength"] =
            json!(180);
        schema["definitions"]["premium_timeline_slot"]["properties"]["title"]["maxLength"] =
            json!(56);
        schema["definitions"]["premium_timeline_slot"]["properties"]["theme"]["maxLength"] =
            json!(32);
        schema["definitions"]["premium_timeline_slot"]["properties"]["tone"]["maxLength"] =
            json!(24);
        schema["definitions"]["premium_timeline_slot"]["properties"]["text"]["maxLength"] =
            json!(180);
        schema["definitions"]["premium_timeline_slot"]["properties"]["advice"]["maxLength"] =
            json!(120);
        schema["definitions"]["premium_timeline_slot"]["properties"]["watch_point"]["maxLength"] =
            json!(120);
        schema["definitions"]["premium_timeline_slot"]["properties"]["fallback_reason"]
            ["maxLength"] = json!(80);
        schema["definitions"]["domain_section"]["properties"]["title"]["maxLength"] = json!(48);
        schema["definitions"]["domain_section"]["properties"]["text"]["maxLength"] = json!(220);
    }
    Ok(crate::provider_schema_compiler::prepare_strict_json_schema(
        &schema,
    ))
}

pub fn daily_writer_messages(request: &Value) -> Result<Vec<PromptMessage>, GenerationError> {
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let service_code = request
        .get("service_code")
        .and_then(Value::as_str)
        .unwrap_or(HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE);
    let slot_instruction = if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        "Produis un horoscope quotidien Free sans slots publics, avec summary, advice, watch_point et evidence_keys uniquement. Le texte public doit citer une référence astrologique issue des preuves, par exemple la Lune, Mars, Vénus, Mercure, un transit, un aspect ou une maison."
    } else if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        "Produis un horoscope quotidien Premium avec timeline, best_slots, watch_slots, domain_sections et advice. Retourne un JSON compact minified: une seule ligne, sans markdown, sans commentaires, sans indentation. Garde la densité courte et utile: summary.text en 1 à 2 phrases; chaque timeline.text en 1 phrase courte; chaque timeline.advice et watch_point en 1 phrase courte; chaque reason de best_slots et watch_slots en 1 phrase courte liée au créneau; chaque domain_sections.text en 1 à 2 phrases maximum. Les 12 entrées timeline doivent avoir des titres et angles rédactionnels distincts. Les reason de best_slots et watch_slots doivent être spécifiques au créneau, jamais copiées-collées entre deux créneaux. Dans domain_sections, garde le champ technique domain tel quel, mais n'écris jamais ce code anglais dans title ou text. Évite les formulations mécaniques répétées comme clarifier, concret, tension, ralentir les réponses ou lire par séquences plus de deux fois dans l'ensemble de la lecture."
    } else {
        "Produis exactement trois slots publics correspondant aux labels Matin, Après-midi et Soir. Chaque slot.text doit citer une référence astrologique publique issue de ses preuves, par exemple la Lune, Mars, Vénus, Mercure, un transit, un aspect ou une maison."
    };

    Ok(vec![
        PromptMessage {
            role: PromptRole::System,
            content: "Tu rédiges un horoscope quotidien personnalisé en français. Retourne uniquement un objet JSON compact minified conforme au schéma fourni horoscope_response: pas de markdown, pas de commentaires, pas de pretty print, pas d'indentation. N'invente aucune preuve astrologique: chaque evidence_key publique doit provenir de la requête. N'affiche jamais les codes internes, les noms de champs, les clés de preuve, les theme_code anglais, les codes tone anglais, ni les consignes internes.".to_string(),
        },
        PromptMessage {
            role: PromptRole::User,
            content: format!(
                "{slot_instruction} Le résumé doit introduire la tonalité générale sans recopier day_overview. Les textes doivent rester concrets, personnalisés par les signaux fournis, sans promesse événementielle. Utilise uniquement les libellés français déjà fournis pour les titres publics. Requête JSON:\n{compact}"
            ),
        },
    ])
}

fn daily_writer_json_repair_messages(
    request: &Value,
    raw_text: &str,
) -> Result<Vec<PromptMessage>, GenerationError> {
    let compact = serde_json::to_string(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_RESPONSE_INVALID: {err}"),
            Value::Null,
        )
    })?;
    Ok(vec![
        PromptMessage {
            role: PromptRole::System,
            content: "Tu répares une sortie d'horoscope quotidien. Retourne uniquement un objet JSON compact minified conforme au schéma horoscope_response fourni: pas de markdown, pas de commentaire, pas de texte hors JSON. Si la sortie précédente est inutilisable, régénère depuis la requête JSON en respectant les preuves et les champs attendus.".to_string(),
        },
        PromptMessage {
            role: PromptRole::User,
            content: format!(
                "La réponse précédente du provider n'était pas un JSON valide. Produit maintenant uniquement le JSON final conforme au schéma.\nRequête JSON:\n{compact}\nSortie invalide précédente:\n{}",
                truncate_daily_repair_raw_text(raw_text, 6000)
            ),
        },
    ])
}

fn truncate_daily_repair_raw_text(raw_text: &str, max_chars: usize) -> String {
    let mut truncated = raw_text.chars().take(max_chars).collect::<String>();
    if raw_text.chars().count() > max_chars {
        truncated.push_str("\n[truncated]");
    }
    truncated
}

pub(crate) fn daily_writer_max_output_tokens(request: &Value) -> u32 {
    match request.get("service_code").and_then(Value::as_str) {
        Some(HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE) => 12_000,
        Some(HOROSCOPE_FREE_DAILY_SERVICE_CODE) => 4_000,
        _ => 8_000,
    }
}

pub(crate) fn build_domain_sections(request: &Value) -> Vec<Value> {
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .take(4)
        .filter_map(|item| item.get("evidence_key").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect::<Vec<_>>();
    premium_domain_rows()
        .unwrap_or_default()
        .into_iter()
        .map(|(domain, title)| {
            json!({
                "domain": domain,
                "title": title,
                "text": "Cette section relie les meilleurs repères horaires aux preuves astrologiques retenues, sans promettre d'événement.",
                "evidence_keys": evidence
            })
        })
        .collect()
}

pub(crate) fn premium_domain_rows() -> Result<Vec<(String, String)>, GenerationError> {
    let mut rows = rows(DOMAIN_SCORE_MAPPINGS_JSON)?
        .into_iter()
        .filter(|row| {
            row.get("service_code").and_then(|v| v.as_str())
                == Some(HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE)
        })
        .filter_map(|row| {
            Some((
                row.get("domain_code")?.as_str()?.to_string(),
                row.get("domain_title")?.as_str()?.to_string(),
                row.get("sort_order")?.as_i64()?,
            ))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|(_, _, sort_order)| *sort_order);
    Ok(rows
        .into_iter()
        .map(|(domain, title, _)| (domain, title))
        .collect())
}

pub(crate) fn fake_writer_response(request: &Value) -> Result<Value, GenerationError> {
    let service_code = request
        .get("service_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    if service_code == HOROSCOPE_FREE_DAILY_SERVICE_CODE {
        return fake_writer_free_response(request);
    }
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return fake_writer_premium_response(request);
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
    let response = json!({
        "contract_version": "horoscope_response",
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
    });
    Ok(reprocess_horoscope_daily_payload(response))
}

pub(crate) fn fake_writer_premium_response(request: &Value) -> Result<Value, GenerationError> {
    let period = request
        .get("period")
        .cloned()
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let slots = request
        .get("slots")
        .and_then(|v| v.as_array())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let timeline = slots
        .iter()
        .enumerate()
        .map(|(idx, slot)| render_fake_premium_timeline_slot(slot, idx))
        .collect::<Result<Vec<_>, _>>()?;
    let best_slots = request
        .get("best_slots")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let watch_slots = request
        .get("watch_slots")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let domain_sections = request
        .get("domain_sections")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|section| {
            json!({
                "domain": section.get("domain").cloned().unwrap_or_else(|| json!("daily")),
                "title": section.get("title").cloned().unwrap_or_else(|| json!("Repères du jour")),
                "text": "Les preuves astrologiques retenues donnent un repère pratique pour organiser ce domaine sans annoncer d'événement certain.",
                "evidence_keys": section.get("evidence_keys").cloned().unwrap_or_else(|| json!([]))
            })
        })
        .collect::<Vec<_>>();
    let evidence = request
        .get("evidence")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(|item| {
            json!({
                "evidence_key": item.get("evidence_key").cloned().unwrap_or(Value::Null),
                "theme_code": item.get("theme_code").cloned().unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();

    let response = json!({
        "contract_version": "horoscope_response",
        "service_code": HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Votre météo astrologique détaillée",
            "text": "La journée se lit par créneaux courts : certains moments favorisent l'organisation, d'autres demandent de ralentir la réponse émotionnelle. Les repères ci-dessous s'appuient sur les preuves astrologiques sélectionnées et restent des indications pratiques, non des promesses d'événements."
        },
        "best_slots": best_slots,
        "watch_slots": watch_slots,
        "timeline": timeline,
        "domain_sections": domain_sections,
        "advice": {
            "main": "Utilisez les créneaux les plus fluides pour les décisions concrètes et gardez les moments tendus pour observer avant d'agir.",
            "best_use": "Planifier, prioriser et formuler les échanges importants quand la tonalité est plus claire.",
            "avoid": "Transformer un signal bref en certitude ou répondre trop vite pendant un créneau de vigilance."
        },
        "evidence_summary": evidence,
        "quality": {
            "provider": "fake",
            "evidence_guard": "passed",
            "timeline_count": 12,
            "timeline_order_passed": true,
            "premium_rich_bounds": "fake_structural_only"
        }
    });
    Ok(reprocess_horoscope_daily_payload(response))
}

pub(crate) fn render_fake_premium_timeline_slot(
    slot: &Value,
    index: usize,
) -> Result<Value, GenerationError> {
    let label = slot
        .get("slot_label")
        .and_then(|v| v.as_str())
        .ok_or_else(|| horoscope_error("HOROSCOPE_RESPONSE_INVALID"))?;
    let evidence_keys = slot
        .get("required_evidence_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let tone = slot.get("tone").and_then(|v| v.as_str()).unwrap_or("mixed");
    let best_for = slot
        .get("best_for")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let theme_code = slot
        .get("theme_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let internal_watch_point = slot.get("watch_point").and_then(|v| v.as_str());
    let watch_point = public_watch_point_for_theme(theme_code)?
        .or_else(|| {
            internal_watch_point.and_then(|value| {
                if value.contains("avoid_") {
                    None
                } else {
                    Some(value.to_string())
                }
            })
        })
        .unwrap_or_else(|| "Gardez un repère simple et vérifiable.".to_string());
    Ok(json!({
        "slot_label": label,
        "title": premium_timeline_title(index),
        "theme": premium_timeline_theme(index),
        "tone": tone,
        "text": premium_timeline_text(index),
        "advice": premium_timeline_advice(index),
        "best_for": best_for,
        "watch_point": watch_point,
        "evidence_keys": evidence_keys
    }))
}

pub(crate) fn premium_timeline_title(index: usize) -> &'static str {
    match index % 4 {
        0 => "Clarté pratique",
        1 => "Rythme à canaliser",
        2 => "Réactivité à modérer",
        _ => "Dialogue à simplifier",
    }
}

pub(crate) fn premium_timeline_theme(index: usize) -> &'static str {
    match index % 4 {
        0 => "Organisation",
        1 => "Énergie",
        2 => "Émotion",
        _ => "Relation",
    }
}

pub(crate) fn premium_timeline_text(index: usize) -> &'static str {
    match index % 4 {
        0 => "La Lune donne un repère concret pour organiser une priorité sans disperser l'attention.",
        1 => "Le climat du créneau soutient une action courte, à condition de garder un cadre mesurable.",
        2 => "Mars rend la réaction plus vive : mieux vaut vérifier le détail avant de répondre.",
        _ => "Vénus adoucit l'échange si vous revenez à un sujet précis plutôt qu'à toute l'histoire.",
    }
}

pub(crate) fn premium_timeline_advice(index: usize) -> &'static str {
    match index % 4 {
        0 => "Choisissez une tâche utile et terminez-la avant d'en ouvrir une autre.",
        1 => "Gardez le mouvement, mais limitez le nombre de décisions simultanées.",
        2 => "Respirez avant de répondre et reformulez ce qui manque.",
        _ => "Préférez une phrase simple à une explication trop longue.",
    }
}

pub(crate) fn fake_writer_free_response(request: &Value) -> Result<Value, GenerationError> {
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
    let response = json!({
        "contract_version": "horoscope_response",
        "service_code": HOROSCOPE_FREE_DAILY_SERVICE_CODE,
        "period": period,
        "summary": {
            "title": "Votre tendance du jour",
            "text": "La Lune met l'accent sur l'organisation, les priorités simples et les gestes utiles. La journée gagne à rester concrète : choisir une tâche mesurable, clarifier ce qui doit vraiment avancer, puis éviter de multiplier les intentions."
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
    });
    Ok(reprocess_horoscope_daily_payload(response))
}

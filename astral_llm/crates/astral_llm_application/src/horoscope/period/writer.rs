use super::*;
pub(crate) async fn period_writer_response(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let defaults = horoscope_writer_engine_defaults(use_case);
    if defaults.provider == ProviderKind::Fake {
        return fake_period_writer_response(request);
    }
    let schema = period_response_provider_schema(request)?;
    let provider_request = ProviderGenerationRequest {
        model: defaults.model.clone(),
        messages: period_writer_messages(request)?,
        structured_schema: Some(schema),
        reasoning_effort: period_writer_reasoning_effort(request),
        temperature: Some(if is_period_writer_request_v2(request) {
            0.35
        } else if is_premium_period_request(request) {
            0.55
        } else {
            0.4
        }),
        max_output_tokens: Some(period_writer_max_output_tokens(request)),
        safety_mode: SafetyMode::PlatformRulesOnly,
        timeout: StdDuration::from_secs(180),
        metadata: GenerationMetadata {
            run_id: run_id
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            request_id: None,
            product_code: request["service_code"]
                .as_str()
                .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE)
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
            "HOROSCOPE_PERIOD_TECHNICAL_CODE_LEAK",
            json!({ "provider": "fake" }),
        ));
    }
    let mut response = routed        .response        .parsed_json        .or_else(|| parse_period_provider_json(&routed.response.raw_text))        .ok_or_else(|| {            let incomplete_reason =                period_provider_incomplete_reason(&routed.response.provider_metadata);            GenerationError::with_details(                GenerationErrorCode::PostSafetyValidationFailed,                format!(                    "HOROSCOPE_PERIOD_RESPONSE_INVALID: provider_response_not_json raw_text_len={}",                    routed.response.raw_text.len()                ),                json!({                    "reason": "provider_response_not_json",                    "raw_text_len": routed.response.raw_text.len(),                    "provider_incomplete_reason": incomplete_reason                }),            )        })?;
    if !response
        .get("quality")
        .map_or(false, |value| value.is_object())
    {
        response["quality"] = json!({});
    }
    response["quality"]["provider"] = json!(routed.used_provider.as_str());
    response["quality"]["model"] = json!(routed.response.model_used);
    response["quality"]["fallback_used"] = json!(routed.fallback_used);
    if is_period_writer_request_v2(request) {
        repair_period_response_shape_v2(request, &mut response);
        response = postprocess_period_provider_response_v2(request, response);
        return Ok(response);
    }
    repair_period_response_shape(request, &mut response);
    normalize_period_public_tones(request, &mut response);
    response = postprocess_period_provider_response(request, response);
    enforce_period_public_personalization_from_request(request, &mut response);
    enforce_premium_period_advice_synthesis(request, &mut response);
    restore_period_response_evidence_from_request(request, &mut response);
    normalize_period_public_strings(&mut response);
    enforce_period_public_personalization_from_request(request, &mut response);
    validate_period_provider_public_payload(&response)?;
    Ok(response)
}
pub(crate) async fn period_writer_response_with_quality_loop(
    use_case: &GenerateReadingUseCase,
    request: &Value,
    run_id: Option<&str>,
) -> Result<Value, GenerationError> {
    let mut response = period_writer_response(use_case, request, run_id).await?;
    for attempt in 0..=PERIOD_V2_QUALITY_MAX_RETRIES {
        match validate_period_response_quality_gates_v2(request, &response) {
            Ok(()) => return Ok(response),
            Err(err) if attempt < PERIOD_V2_QUALITY_MAX_RETRIES => {
                response =
                    period_style_editor_response_v2(use_case, request, &response, &err, run_id)
                        .await?;
            }
            Err(err) => {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::PostSafetyValidationFailed,
                    "HOROSCOPE_PERIOD_V2_QUALITY_FAILED",
                    json!({                        "attempts": attempt + 1,                        "max_retries": PERIOD_V2_QUALITY_MAX_RETRIES,                        "issues": [period_v2_quality_issue("/", "quality_failed", "error", &err.detail().message)]                    }),
                ));
            }
        }
    }
    Ok(response)
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
        return Ok(schema);
    }
    if premium {
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
    }
    Ok(schema)
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
    if is_period_writer_request_v2(request) {
        return period_writer_messages_v2(request);
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
        return Ok(vec![            PromptMessage {                role: PromptRole::System,                content: format!(                    "Tu écris un horoscope Free des 7 prochains jours en français. Retourne uniquement un JSON conforme au schéma fourni. N'expose jamais daily_timeline, best_days, watch_days, windows, domain_sections ou strategy. N'invente aucune preuve et n'affiche aucun code interne. Le texte public doit compter entre {} et {} mots, sans dépasser {} mots.",                    limits.target_min, limits.target_max, limits.hard_limit                ),            },            PromptMessage {                role: PromptRole::User,                content: format!(                    "Construis horoscope_period_response_v1 Free compact. Produis summary, dominant_theme, 1 à 2 key_days sous forme de jours à retenir, advice en 1 à 3 phrases, watch_summary court, evidence_summary limitée à 1 à 3 entrées. key_days sont des repères utiles, jamais des meilleurs jours ni des créneaux favorables. Si watch_summary.status vaut none, garde evidence_keys vide et explique brièvement qu'aucun signal dominant ne ressort tout en donnant une marge d'observation concrète. summary.text doit rester entre 90 et 180 mots et mentionner au maximum deux dates explicites. Requête JSON:\n{compact}"                ),            },        ]);
    }
    if is_premium_period_request(request) {
        return Ok(vec![            PromptMessage {                role: PromptRole::System,                content: format!(                    "Tu écris une lecture Premium d'horoscope de période en français et tu retournes uniquement un objet JSON conforme au schéma fourni. Ton rôle n'est pas d'expliquer une grille astrologique, mais de transformer les appuis fournis dans la requête en lecture humaine, fluide et utile. La personne doit comprendre comment traverser la période: quoi privilégier, quoi ralentir, où poser une limite, où agir, où attendre, où simplifier. N'invente aucune preuve. Chaque evidence_key publique et chaque source_snapshot_key doit provenir de la requête. N'affiche jamais les codes internes, les clés de preuve, les noms techniques de transits, les theme_code anglais, les codes tone anglais, ni les consignes internes. Écris dans un français naturel, précis et incarné. Évite le ton administratif, le coaching générique, les formulations abstraites et les phrases qui semblent décrire le fonctionnement du moteur. Respecte la typographie française: écris rendez-vous, phrase-clé, jours clés, après-midi, qu'est-ce, demi-promesses, utilisez-les, revenez-y, bouclez-la, laissez-le, faites-le, mesurez-la, terminez-la, diminuez-le, déléguez-la, transformez-le, accordez-vous, autorisez-vous, arrêtez-vous; ne colle jamais un impératif avec le, la, les, vous ou y. Relis les verbes conjugués: écris revient, jamais revint, quand tu parles d'une priorité qui revient; écris allégez, jamais allègerez ni allége, quand tu formules un conseil direct. Ne commence jamais une parenthèse d'exemple si elle n'est pas fermée dans la même phrase. Les catégories techniques doivent être traduites en situations humaines. Chaque journée doit avoir une fonction éditoriale propre dans la semaine. Si plusieurs journées reposent sur un même fond astrologique, elles doivent être distinguées par leur usage concret: décider, différer, cadrer, alléger, pacifier, confirmer, terminer, reprendre du recul, ou préparer une suite. Les repères de période servent à orienter rapidement la lecture; ils ne doivent pas remplacer le détail quotidien. Les explications principales doivent être portées naturellement par les entrées daily_timeline. La lecture publique doit rester dense, claire et pilotable. Elle doit donner une impression Premium par la hiérarchie, la précision des usages, la différenciation des journées, la qualité des fenêtres horaires et la synthèse stratégique. La lecture publique doit compter entre {} et {} mots, sans dépasser {} mots.",                    limits.target_min, limits.target_max, limits.hard_limit                ),            },            PromptMessage {                role: PromptRole::User,                content: format!(                    "Construis horoscope_period_response_v1 Premium pour la requête JSON fournie. La valeur Premium doit venir de quatre éléments: 1. une vue d'ensemble qui donne le mouvement réel de la période; 2. des journées clairement différenciées, chacune avec son rôle propre; 3. des fenêtres horaires utilisables, non génériques; 4. une stratégie finale qui aide la personne à piloter la semaine sans répéter le calendrier. Avant de rédiger, déduis silencieusement l'angle éditorial de la semaine: ce qui monte en intensité; ce qui devient plus simple; ce qui demande de la prudence; ce qui peut être décidé, reporté, allégé ou confirmé; la différence entre les journées qui semblent proches. Utilise editorial_brief quand il est présent: il donne le rôle humain, la fonction narrative, la situation lecteur, le mode d'action et l'angle à ne pas répéter pour chaque date. editorial_brief est une aide interne de différenciation: ne recopie jamais directement public_role, narrative_function, reader_situation ou avoid_angle_reuse. Transforme-les en scène humaine naturelle. Les titres publics doivent rester courts, lisibles et non méta. Interdit dans la sortie: nouvelle facette, répéter le même conseil, fonction narrative, changer l'usage, priorité liée à, La journée dynamique, la même priorité revint, Stabiliser Tester limites Agir par gestes courts. Pour chaque daily_timeline, garde le thème principal du daily_plan, mais transforme-le en situation humaine. Le texte principal et le conseil doivent rester alignés avec ce thème principal; si tu utilises un signal secondaire du même jour, garde-le en nuance courte et ne déplace pas l'axe de la journée. Termine toujours chaque phrase: aucune parenthèse ouverte, aucun exemple coupé, aucune fin sur par ex. Explique ce que la personne peut faire de cette journée, ce qu'elle doit éviter d'alourdir, et ce qui la distingue des autres dates de la période. Mentionne les éléments secondaires uniquement s'ils apportent une nuance réelle. key_days, best_days et watch_days doivent rester des repères courts, naturels et non mécaniques. Ne recopie jamais les situations associées sous forme de liste. Sépare strictement best_days et watch_days: best_days doit parler d'opportunité, d'appui, de ressource, de rendez-vous, de preuve ou de tâche pratique; watch_days doit parler de risque, délai, charge, limite ou promesse à vérifier. Interdit dans best_days: Avant de promettre davantage. Interdit dans ces raisons: autour de vérifier, autour de attendre, appuis concrets aide, Appui concret :, est un point d'appui pour, demande de ralentir sur, priorité liée à, ou une construction de type thème + aide à. Transforme la donnée en phrase courte et lisible: date, rôle, puis une seule action concrète. Exemple best_days: Mercredi 10/06 aide à sécuriser une base concrète : ressource, rendez-vous, preuve ou tâche pratique. Exemple watch_days: Jeudi 11/06 demande de vérifier délai et charge avant d'accepter. Quand une date est importante, favorable ou sensible, l'explication complète doit apparaître dans l'entrée daily_timeline correspondante, pas être répétée dans key_days ou watch_days. best_windows et watch_windows sont des plages horaires. Pour chaque fenêtre, indique un usage concret lié à la période: confirmer une ressource, fermer une tâche, demander une preuve, envoyer un message ciblé, cadrer une réponse, différer une promesse, se retirer, reprendre ou terminer. Ne produis jamais une phrase de remplissage interchangeable comme Ce créneau peut servir..., Ce créneau se prête..., ou Ce créneau aide.... domain_sections doit contenir 3 à 5 domaines réellement distincts. Chaque domaine doit apporter un angle transverse que les journées ne répètent pas déjà. N'utilise pas de structure répétée comme Dans ce domaine..., Dans X, Le plus utile..., X donne une direction claire, Cette énergie devient utile..., les repères les plus utiles consistent..., consiste à de, ni Et à choisir le bon niveau d'engagement. Écris chaque domaine comme une mini-lecture naturelle: à quoi sert ce domaine dans la semaine, quelle nuance personnelle il éclaire, et quel geste évite de tout alourdir. Si deux domaines se recoupent, fusionne-les ou choisis le plus utile pour la personne. advice et strategy doivent synthétiser une méthode d'usage riche et pratique. Ils ne doivent pas refaire la liste des dates. Ils doivent expliquer comment utiliser les fenêtres favorables, comment traverser les moments sensibles, comment transformer une promesse vague en preuve concrète, et comment garder une marge de manœuvre. Utilise les libellés français présents dans la requête, mais remplace les taxonomies publiques lourdes par des mots naturels quand nécessaire: relationnel, lien personnel, besoin affectif, cadre, appui concret. N'affiche aucun code interne. Respecte les preuves fournies. Développe les sections publiques afin d'atteindre {} à {} mots publics. Retourne uniquement le JSON conforme au schéma. Requête JSON:\n{compact}",                    limits.target_min, limits.target_max                ),            },        ]);
    }
    Ok(vec![        PromptMessage {            role: PromptRole::System,            content: format!(                "Tu écris une lecture d'horoscope de période en français. Retourne uniquement un objet JSON conforme au schéma fourni. N'invente aucune preuve: chaque evidence_key publique doit provenir de la requête. N'affiche jamais les codes internes, les clés de preuve, les noms techniques de transits, les theme_code anglais, ni les codes tone anglais. La timeline doit couvrir exactement les 7 dates, avec des formulations variées et une trajectoire globale. La lecture publique doit compter entre {} et {} mots, sans dépasser {} mots.",                limits.target_min, limits.target_max, limits.hard_limit            ),        },        PromptMessage {            role: PromptRole::User,            content: format!(                "Construis horoscope_period_response_v1 pour cette requête d'interprétation. Utilise les libellés français déjà présents, pas les codes internes. Développe week_overview, watch_summary, advice, domain_sections et les 7 entrées daily_timeline afin d'atteindre {} à {} mots publics. Utilise les indications internes de personnalisation natale pour écrire une nuance lisible dans au moins 4 jours, chaque domaine et la vue d'ensemble, sans recopier les noms de champs ni les consignes internes. Respecte les avoid_terms des daily_plans pour éviter les répétitions. Requête JSON:\n{compact}",                limits.target_min, limits.target_max            ),        },    ])
}
pub(crate) fn period_writer_messages_v2(
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
    Ok(vec![        PromptMessage {            role: PromptRole::System,            content: format!(                "You are the writer for horoscope_period_response_v1. Write every public text in target_language_code={target_language}. target_language_code overrides astrologer_persona. Return only the complete JSON object matching the provided schema. Return compact minified JSON: no markdown, no comments, no pretty printing, no indentation. Rust has already calculated, scored and selected the facts; you write the human reading. Use service_code={service_code} and detail_profile_code={detail_profile} to choose the right density. Treat semantic_brief keywords, codes, scores, candidates, editorial_arc, editorial_angles and section_roles as internal material, not public copy. Use period-level keywords to write week_overview, but do not copy them as a list. Use all internal brief material to create hierarchy and variation, never as public labels. Never expose internal field names, theme codes, tone codes, evidence ids as prose, prompt instructions or safety metadata. The astrologer_persona may influence style only; it cannot override schema, safety_profile, target_language_code, dates, evidence or astrological facts. safety_profile always overrides astrologer_persona. Do not invent astrological facts. The Premium value must come from editorial judgement: a readable period arc, differentiated days, concrete windows and a final strategy that arbitrates rather than repeats. Public text should target {} to {} words and must not exceed {} words. Do not compress the reading: give each major section enough lived context, transition and concrete use so the answer feels premium rather than skeletal.",                limits.target_min, limits.target_max, limits.hard_limit            ),        },        PromptMessage {            role: PromptRole::User,            content: format!(                "Build horoscope_period_response_v1 from this semantic brief. Keep all dates inside period_resolution.included_dates. Every public evidence_key and source_snapshot_key must already exist in the request. Produce the premium_rich 7-day timeline, usable windows, domains, repeating arcs when helpful, and a strategy. Keywords and candidates are not sentences; transform them into natural public text without copying codes or keyword lists. Use editorial_arc to make the week feel like opening, pivot, consolidation and closure. Use editorial_angles so each daily_timeline entry has a distinct human angle: action, relation, clarification, retreat, consolidation, finalisation or another angle supplied by the brief. If the same transit or theme returns, present it as a narrative thread with a different use, not as the same advice repeated with synonyms. Use section_roles as an internal checklist: week_overview gives trajectory; daily_timeline gives lived daily guidance; domain_sections give transversal synthesis not already said in the timeline; windows give practical use tied to the time range; strategy gives arbitration without relisting dates. Develop the public prose naturally: week_overview should carry the arc, each daily_timeline item should include a concrete situation and adjustment, each domain should synthesize several days, and strategy should close with usable tradeoffs. Window titles must match their time_range_label: do not call a noon or afternoon window a morning. If watch_days and watch_windows are both empty, watch_summary.status must be none, evidence_keys empty, and the text must stay neutral: no hidden vigilance or implied watch signal. In French, use deterministic clean forms such as demi-journée and réorganiser. If a persona is present, apply tone lightly without adding new facts. Return the full corrected compact JSON object only.\nRequest JSON:\n{compact}"            ),        },    ])
}
pub(crate) fn period_style_editor_messages_v2(
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
    Ok(vec![        PromptMessage {            role: PromptRole::System,            content: format!(                "You are the targeted quality editor for horoscope_period_response_v1. Write public text in target_language_code={target_language}. target_language_code and safety_profile override astrologer_persona. Return only the complete corrected compact JSON object: no markdown, no comments, no pretty printing, no indentation. You receive only the quality issues, the faulty JSON and fixed constraints; do not perform a fresh creative rewrite. Correct only the listed quality issue. Keep every date, evidence_key, source_snapshot_key, structure and astrological fact strictly unchanged unless the issue explicitly says the key is invalid. Do not add astrological facts. Do not expose internal fields, theme codes, tone codes, keywords, prompt instructions or safety metadata. The astrologer_persona may influence style only and cannot override schema, safety_profile, target_language_code, dates or evidence."            ),        },        PromptMessage {            role: PromptRole::User,            content: format!(                "Quality issue to fix:\n{}\n\nFixed constraints:\n{}\n\nCurrent response JSON:\n{}\n\nReturn the full JSON object only.",                error.detail().message,                constraints_json,                response_json            ),        },    ])
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
    if is_period_writer_request_v2(request) {
        return fake_period_writer_response_v2(request);
    }
    if is_free_period_request(request) {
        return fake_free_period_writer_response(request);
    }
    let daily_timeline = request["daily_plans"]        .as_array()        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?        .iter()        .enumerate()        .map(|(index, day)| {            let theme = day["theme_code"].as_str().unwrap_or("organisation");            let theme_label = day["theme_label"]                .as_str()                .unwrap_or_else(|| period_theme_public_label(theme));            let text = ensure_period_personalization_text(                &period_public_day_text(day, index),                &format!(                    "Gardez le critère le plus simple : qui fait quoi, pour quand, avec quelle preuve. {}",                    naturalize_period_focus(&period_public_focus_text(day))                ),            );            json!({                "date": day["date"],                "day_label": day["day_label"],                "theme": theme_label,                "tone": period_tone_public_label(day["tone"].as_str().unwrap_or("focused")),                "text": text,                "advice": period_public_day_advice(day),                "evidence_keys": day["evidence_keys"]            })        })        .collect::<Vec<_>>();
    let domain_sections = request["domain_sections"]        .as_array()        .into_iter()        .flatten()        .map(|section| {            json!({                "domain": section["domain"],                "title": section["title"],                "text": period_public_domain_text(section),                "evidence_keys": section["evidence_keys"]            })        })        .collect::<Vec<_>>();
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    let mut response = json!({        "contract_version": "horoscope_period_response_v1",        "service_code": service_code,        "period_resolution": request["period_resolution"],        "week_overview": {            "title": "Vos 7 prochains jours",            "text": "La période se lit comme une progression continue : d'abord nommer les priorités dans les relations directes, puis cadrer les échanges et terminer sur une intégration plus posée.",            "trajectory": "Une trajectoire globale relie les jours clés, les besoins émotionnels et les choix à consolider."        },        "key_days": request["key_days"],        "best_days": request["best_days"],        "watch_days": request["watch_days"],        "watch_summary": request["watch_summary_plan"],        "daily_timeline": daily_timeline,        "domain_sections": domain_sections,        "advice": {            "main": "Avancez par étapes courtes et gardez une trace de ce qui évolue d'un jour à l'autre.",            "best_use": "Planifier, prioriser et consolider les échanges importants.",            "avoid": "Transformer un signal quotidien en certitude définitive."        },        "evidence_summary": request["evidence"].as_array().into_iter().flatten().take(5).map(|item| json!({            "evidence_key": item["evidence_key"],            "date": item["date"],            "label": item["human_label"]        })).collect::<Vec<_>>(),        "quality": {            "daily_timeline_count": 7,            "evidence_guard_passed": true,            "best_watch_overlap_passed": true,            "provider": "fake",            "model": "fake-model",            "fallback_used": false,            "period_contract": "basic_next_7_days"        }    });
    if is_premium_period_service(service_code) {
        response["best_windows"] = request["best_windows"].clone();
        response["watch_windows"] = request["watch_windows"].clone();
        response["strategy"] = json!({            "title": request["strategy"]["title"].as_str().unwrap_or("Stratégie de semaine"),            "text": "Utilisez les meilleurs créneaux pour agir court et les moments de vigilance pour ralentir avant de répondre. La stratégie consiste à alterner décision, mise au net et récupération sans transformer la semaine en suite d'urgences.",            "best_use": request["strategy"]["best_use"].as_str().unwrap_or("Réserver les créneaux soutenants aux échanges utiles."),            "recovery": request["strategy"]["recovery"].as_str().unwrap_or("Préserver des temps de recul après les moments plus réactifs."),            "evidence_keys": request["strategy"]["evidence_keys"]        });
        response["quality"]["period_contract"] = json!("premium_next_7_days");
    }
    repair_period_response_shape(request, &mut response);
    Ok(response)
}
pub fn fake_period_writer_response_v2(request: &Value) -> Result<Value, GenerationError> {
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
        "Jour à retenir",
        "Repère utile à lire dans le mouvement global de la période.",
        primary_date,
        &primary_key,
    );
    if is_free_period_service(service_code) {
        let mut response = json!({            "contract_version": "horoscope_period_response_v1",            "service_code": service_code,            "period_resolution": request["period_resolution"],            "summary": {                "title": "Vos 7 prochains jours",                "text": format!("Cette période donne une boussole générale plutôt qu'un planning détaillé. Le thème {primary_theme} ressort comme fil conducteur : il aide à repérer une priorité, un échange ou une routine à stabiliser sans transformer chaque signal en certitude. Gardez une marge d'observation, puis choisissez une action simple et vérifiable.")            },            "dominant_theme": {                "theme": primary_theme,                "text": "Le thème dominant sert de repère pour hiérarchiser les décisions et éviter la dispersion."            },            "key_days": key_days.into_iter().take(2).collect::<Vec<_>>(),            "advice": "Gardez une seule priorité observable, puis ajustez-la si le même signal revient dans la semaine.",            "watch_summary": {                "status": "low",                "text": "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation.",                "evidence_keys": [primary_key]            },            "evidence_summary": evidence_summary_v2(evidence, 3),            "quality": quality_v2(service_code, request, 0)        });
        repair_period_response_shape_v2(request, &mut response);
        return Ok(response);
    }
    let daily_timeline = request        .pointer("/semantic_brief/daily_signal_summary")        .and_then(Value::as_array)        .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_TIMELINE_MISSING"))?        .iter()        .map(|day| {            let date = day["date"].as_str().unwrap_or(primary_date);            let theme_code = day["theme_codes"]                .as_array()                .and_then(|items| items.first())                .and_then(Value::as_str)                .unwrap_or("organization");            let tone_code = day["tone_codes"]                .as_array()                .and_then(|items| items.first())                .and_then(Value::as_str)                .unwrap_or("focused");            let keys = day["evidence_keys"].as_array().cloned().unwrap_or_else(|| vec![primary_key.clone()]);            json!({                "date": date,                "day_label": public_day_label(date),                "theme": period_theme_public_label(theme_code),                "tone": period_tone_public_label(tone_code),                "text": format!("Le signal du {date} demande de transformer les indices disponibles en action simple pour vous. Vos priorités restent le filtre concret de cette journée, sans en faire une prédiction fermée."),                "advice": "Choisissez un geste court, vérifiable et relié au contexte réel.",                "evidence_keys": keys            })        })        .collect::<Vec<_>>();
    let best_days = day_markers_from_candidates_v2(
        request
            .pointer("/semantic_brief/best_day_candidates")
            .and_then(Value::as_array),
        "Jour favorable",
        "Appui utile pour avancer sur une action concrète.",
        primary_date,
        &primary_key,
    );
    let watch_days = day_markers_from_candidates_v2(
        request
            .pointer("/semantic_brief/watch_day_candidates")
            .and_then(Value::as_array),
        "Jour de vigilance",
        "Repère utile pour vérifier charge, délai ou limite avant d'accepter.",
        primary_date,
        &primary_key,
    );
    let domain_sections = request        .pointer("/semantic_brief/domain_candidates")        .and_then(Value::as_array)        .into_iter()        .flatten()        .take(if is_premium_period_service(service_code) { 5 } else { 4 })        .map(|domain| {            let code = domain["domain_code"].as_str().unwrap_or("organization");            json!({                "domain": period_theme_public_label(code),                "title": period_domain_title(code),                "text": "Ce domaine sert de repère transversal : il aide à relier les journées entre elles et à choisir un geste qui ne surcharge pas la période.",                "evidence_keys": domain["evidence_keys"]            })        })        .collect::<Vec<_>>();
    let mut response = json!({        "contract_version": "horoscope_period_response_v1",        "service_code": service_code,        "period_resolution": request["period_resolution"],        "week_overview": {            "title": "Vos 7 prochains jours",            "text": "La période se lit comme une progression pour vous : observer les premiers signaux, choisir une priorité concrète, puis ajuster le rythme quand une tension ou une opportunité se répète.",            "trajectory": "Le fil conducteur consiste à relier vos priorités à des décisions plus simples et vérifiables."        },        "key_days": key_days,        "best_days": best_days,        "watch_days": watch_days,        "watch_summary": {            "status": "low",            "text": "Les vigilances demandent surtout de vérifier les limites avant de promettre davantage.",            "evidence_keys": [primary_key.clone()]        },        "daily_timeline": daily_timeline,        "domain_sections": domain_sections,        "advice": {            "main": "Avancez par étapes courtes et gardez une trace de ce qui évolue d'un jour à l'autre.",            "best_use": "Utilisez les appuis pour confirmer une décision ou finaliser une tâche concrète.",            "avoid": "Transformer un signal quotidien en certitude définitive."        },        "evidence_summary": evidence_summary_v2(evidence, 5),        "quality": quality_v2(service_code, request, 7)    });
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
        response["strategy"] = json!({            "title": "Stratégie de semaine",            "text": "Utilisez les meilleurs créneaux pour agir court et les moments de vigilance pour ralentir avant de répondre. La stratégie consiste à alterner décision, mise au net et récupération sans transformer la semaine en suite d'urgences.",            "best_use": "Réserver les appuis aux échanges utiles, aux preuves concrètes et aux décisions réversibles.",            "recovery": "Préserver des temps de recul après les moments plus réactifs.",            "evidence_keys": [primary_key]        });
    }
    repair_period_response_shape_v2(request, &mut response);
    Ok(response)
}
pub(crate) fn day_markers_from_candidates_v2(
    candidates: Option<&Vec<Value>>,
    title: &str,
    reason: &str,
    fallback_date: &str,
    fallback_evidence_key: &Value,
) -> Vec<Value> {
    let mut out = candidates        .into_iter()        .flatten()        .take(4)        .map(|candidate| {            json!({                "date": candidate["date"],                "title": title,                "reason": reason,                "evidence_keys": candidate["evidence_keys"],                "fallback_reason": null            })        })        .collect::<Vec<_>>();
    if out.is_empty() {
        out.push(json!({            "date": fallback_date,            "title": title,            "reason": reason,            "evidence_keys": [fallback_evidence_key.clone()],            "fallback_reason": null        }));
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
    windows        .into_iter()        .take(limit)        .enumerate()        .map(|(index, window)| {            if candidate_type == "watch" {                json!({                    "date": window["date"],                    "time_range_label": window["time_range_label"],                    "source_snapshot_keys": window["source_snapshot_keys"],                    "title": "Fenêtre à cadrer",                    "theme": "Vigilance",                    "tone": "Mesuré",                    "watch_point": "Vérifier la limite avant de répondre.",                    "evidence_keys": window["evidence_keys"]                })            } else {                let (title, best_for) = match index {                    0 => (                        "Fenêtre de confirmation",                        vec!["confirmer", "documenter", "terminer"],                    ),                    1 => (                        "Fenêtre de clarification",                        vec!["clarifier", "répondre", "cadrer"],                    ),                    _ => (                        "Fenêtre de mise au net",                        vec!["prioriser", "classer", "finaliser"],                    ),                };                json!({                    "date": window["date"],                    "time_range_label": window["time_range_label"],                    "source_snapshot_keys": window["source_snapshot_keys"],                    "title": title,                    "theme": "Appui concret",                    "tone": "Constructif",                    "reason": "Moment utile pour confirmer une action courte.",                    "best_for": best_for,                    "evidence_keys": window["evidence_keys"]                })            }        })        .collect()
}
pub(crate) fn evidence_summary_v2(evidence: &[Value], limit: usize) -> Vec<Value> {
    evidence        .iter()        .take(limit)        .map(|item| {            json!({                "evidence_key": item["evidence_key"],                "date": item["date"],                "label": format!(                    "{} / {}",                    period_theme_public_label(item["theme_code"].as_str().unwrap_or("organization")),                    period_tone_public_label(                        item["tone_code"]                            .as_str()                            .or_else(|| item["tone"].as_str())                            .unwrap_or("focused")                    )                )            })        })        .collect()
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
        vec![
            json!({            "date": date,            "title": "Jour à retenir",            "reason": format!("Le thème {} ressort plus nettement et donne un repère utile sans en faire un verdict.", theme),            "evidence_keys": [evidence_key.clone()],            "fallback_reason": null        }),
        ]
    } else {
        key_days
    };
    Ok(
        json!({        "contract_version": "horoscope_period_response_v1",        "service_code": HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,        "period_resolution": request["period_resolution"],        "summary": {            "title": "Vos 7 prochains jours",            "text": format!("Les prochains jours donnent surtout une tendance à comprendre plutôt qu'un planning à suivre. Autour du {date}, le climat met l'accent sur {theme} : une priorité simple, un échange à clarifier ou une routine à stabiliser peut devenir le fil conducteur. L'intérêt est de repérer ce qui demande de l'attention sans découper chaque journée ni chercher une fenêtre idéale. Gardez une marge pour ajuster votre rythme, observez les moments où les émotions accélèrent les décisions, puis revenez à une action concrète. Cette lecture reste volontairement compacte : elle sert de boussole générale pour choisir ce qui mérite d'être traité maintenant et ce qui peut attendre.")        },        "dominant_theme": {            "theme": theme,            "text": format!("Le thème dominant est {theme}. Il invite à privilégier une décision simple, reliée à vos priorités concrètes, plutôt qu'une dispersion sur plusieurs sujets.")        },        "key_days": key_days,        "advice": "Choisissez une seule priorité observable et gardez assez de souplesse pour l'ajuster. Notez ce qui se répète avant de conclure.",        "watch_summary": {            "status": "low",            "text": "Une vigilance légère suffit : ralentir si une réaction paraît plus forte que la situation.",            "evidence_keys": [evidence_key]        },        "evidence_summary": evidence.iter().take(3).map(|item| json!({            "evidence_key": item["evidence_key"],            "date": item["date"],            "label": item["human_label"]        })).collect::<Vec<_>>(),        "quality": {            "daily_timeline_count": 0,            "evidence_guard_passed": true,            "best_watch_overlap_passed": true,            "provider": "fake",            "model": "fake-model",            "fallback_used": false,            "period_contract": "free_next_7_days"        }    }),
    )
}

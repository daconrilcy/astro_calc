use super::*;

pub fn repair_period_response_shape(request: &Value, response: &mut Value) {
    repair_period_response_shape_v2(request, response);
}

pub(crate) fn simple_public_word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphabetic))
        .count()
}

pub(crate) fn prune_period_response_variant_fields_v2(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    if is_free_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("week_overview");
            map.remove("best_days");
            map.remove("watch_days");
            map.remove("daily_timeline");
            map.remove("domain_sections");
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
        return;
    }

    response.as_object_mut().map(|map| {
        map.remove("summary");
        map.remove("dominant_theme");
    });

    if !is_premium_period_service(service_code) {
        response.as_object_mut().map(|map| {
            map.remove("best_windows");
            map.remove("watch_windows");
            map.remove("strategy");
        });
    }
}

pub fn repair_period_response_shape_v2(request: &Value, response: &mut Value) {
    let service_code = request["service_code"]
        .as_str()
        .unwrap_or(HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE);
    response["contract_version"] = json!("horoscope_period_response");
    response["service_code"] = json!(service_code);
    response["period_resolution"] = request["period_resolution"].clone();

    if !response.get("quality").is_some_and(Value::is_object) {
        response["quality"] = quality_v2(
            service_code,
            request,
            if is_free_period_service(service_code) {
                0
            } else {
                7
            },
        );
    }

    prune_period_response_variant_fields_v2(request, response);
    restore_period_response_technical_keys_v2(request, response);
}

pub(crate) fn restore_period_response_technical_keys_v2(request: &Value, response: &mut Value) {
    let evidence_by_date = request["semantic_brief"]["daily_signal_summary"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|day| {
            let date = day["date"].as_str()?.to_string();
            let keys = day["evidence_keys"].as_array()?.clone();
            Some((date, keys))
        })
        .collect::<HashMap<_, _>>();

    for field in ["daily_timeline", "key_days", "best_days", "watch_days"] {
        let Some(items) = response.get_mut(field).and_then(Value::as_array_mut) else {
            continue;
        };
        for item in items {
            if item
                .get("evidence_keys")
                .and_then(Value::as_array)
                .is_some_and(|keys| !keys.is_empty())
            {
                continue;
            }
            let Some(date) = item.get("date").and_then(Value::as_str) else {
                continue;
            };
            if let Some(keys) = evidence_by_date.get(date) {
                item["evidence_keys"] = json!(keys);
            }
        }
    }

    if response["watch_summary"]["status"].as_str() == Some("none") {
        response["watch_summary"]["evidence_keys"] = json!([]);
    }
}

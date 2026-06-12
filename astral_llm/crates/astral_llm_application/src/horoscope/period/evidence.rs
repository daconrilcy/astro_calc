use super::*;
pub(crate) fn period_evidence_from_snapshots(
    snapshots: &[Value],
) -> Result<Vec<Value>, GenerationError> {
    let mut out = Vec::new();
    for snapshot in snapshots {
        let date = snapshot["date"]
            .as_str()
            .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"))?;
        for fact in snapshot
            .get("transits_to_natal")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            let key = fact["evidence_key"]
                .as_str()
                .ok_or_else(|| horoscope_error("HOROSCOPE_PERIOD_EVIDENCE_MISSING"))?;
            let fact_type = fact["fact_type"].as_str().unwrap_or("transit_active");
            let object = fact["transiting_object"].as_str().unwrap_or("moon");
            let aspect = fact.get("aspect").and_then(|value| value.as_str());
            let orb_deg = fact.get("orb_deg").and_then(Value::as_f64);
            if let Some(aspect_code) = aspect {
                if is_period_major_aspect(aspect_code)
                    && orb_deg.unwrap_or(f64::INFINITY) > period_max_major_aspect_orb_deg()
                {
                    return Err(horoscope_error("HOROSCOPE_PERIOD_CALCULATION_FAILED"));
                }
            }
            let theme = match object {
                "venus" => "relationship",
                "mars" => "energy",
                "mercury" => "communication",
                "jupiter" => "integration",
                "sun" => "clarity",
                _ => "organization",
            };
            let tone = period_internal_tone(theme, fact_type, aspect);
            let public_orb = if aspect.is_some() {
                fact.get("orb_deg").cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            let natal_focus_code = period_natal_focus_code(fact);
            let natal_focus = period_natal_focus(&natal_focus_code);
            let human_label = format!(
                "{} met en avant le thème {} en touchant {}",
                period_object_public_label(object),
                period_theme_public_label(theme),
                natal_focus.label
            );
            out.push(json!({                "evidence_key": key,                "date": date,                "snapshot_key": snapshot["snapshot_key"].as_str().unwrap_or(""),                "fact_type": fact_type,                "source": fact["source"].as_str().unwrap_or("calculator"),                "transiting_object": object,                "natal_target": fact.get("natal_target").cloned().unwrap_or(Value::Null),                "aspect": fact.get("aspect").cloned().unwrap_or(Value::Null),                "orb_deg": public_orb,                "natal_house": fact.get("natal_house").cloned().unwrap_or(Value::Null),                "theme_code": theme,                "tone": tone,                "natal_focus_code": natal_focus_code,                "natal_focus_label": natal_focus.label,                "natal_focus_hint": natal_focus.hint,                "personalization_hint": natal_focus.hint,                "human_label": human_label            }));
        }
    }
    Ok(out)
}

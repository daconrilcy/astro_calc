use std::collections::HashMap;

pub fn title_case_sign(sign_code: &str) -> String {
    let mut chars = sign_code.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn strength_label(score: f64) -> &'static str {
    if score >= 0.85 {
        "very strong"
    } else if score >= 0.65 {
        "high"
    } else if score >= 0.45 {
        "moderate"
    } else {
        "present"
    }
}

pub fn accidental_overall_label(expression_quality: &str, polarity: &str) -> String {
    match expression_quality {
        "strongly_constrained_expression" => "Strongly weakened".to_string(),
        "constrained_expression" => "Weakened".to_string(),
        "mixed_or_contextual_expression" => "Mixed".to_string(),
        "strong_external_manifestation" => "Fortified".to_string(),
        _ => match polarity {
            "strongly_weakened" => "Strongly weakened".to_string(),
            "weakened" => "Weakened".to_string(),
            "fortified" => "Fortified".to_string(),
            _ => "Mixed".to_string(),
        },
    }
}

pub fn humanize_reason(reason: &str, object_names: &HashMap<String, String>) -> String {
    let object_label = |code: &str| {
        object_names
            .get(code)
            .cloned()
            .unwrap_or_else(|| title_case_sign(code))
    };

    if let Some((obj, rest)) = reason.split_once('_') {
        match (obj, rest) {
            ("sun" | "moon" | "mercury" | "venus" | "mars" | "jupiter" | "saturn" | "uranus" | "neptune" | "pluto", "in_sign") => {
                return format!("{} in sign", object_label(obj));
            }
            (obj, "in_house") => return format!("{} in house", object_label(obj)),
            (obj, "in_sign") if obj.len() > 2 => return format!("{} in sign", object_label(obj)),
            (obj, "domicile") => return format!("{} in domicile", object_label(obj)),
            (sign, "emphasis") => {
                return format!("{} emphasis", title_case_sign(sign));
            }
            _ => {}
        }
    }

    match reason {
        "multiple_objects" => "Multiple chart factors".to_string(),
        "cluster" => "Dominant house cluster".to_string(),
        "sign_house_cluster" => "Sign and house cluster".to_string(),
        "saturn_domicile" => "Saturn in domicile".to_string(),
        "placement" => "Strong placement".to_string(),
        "cluster_participant" => "Participant in dominant theme".to_string(),
        "accidental_context" => "Accidental dignity context".to_string(),
        "ascendant_in_house" => "Ascendant in house".to_string(),
        "dominant_house" => "Dominant house".to_string(),
        "active_signal" => "Active chart signal".to_string(),
        "rulership_context" => "Rulership routing".to_string(),
        "resources_theme" => "Resources theme".to_string(),
        "cross_axis_aspect" => "Cross-axis aspect".to_string(),
        "sun_luminary_in_house" => "Sun as luminary in house".to_string(),
        code => code.replace('_', " "),
    }
}

pub fn humanize_condition_code(code: &str, chart_sect: Option<&str>) -> String {
    match code {
        "angular_house" => "Angular house".to_string(),
        "succedent_house" => "Succedent house".to_string(),
        "cadent_house" => "Cadent house".to_string(),
        "below_horizon" => "Below horizon".to_string(),
        "above_horizon" => "Above horizon".to_string(),
        "on_horizon" => "On horizon".to_string(),
        "retrograde_motion" => "Retrograde motion".to_string(),
        "stationary_motion" => "Stationary motion".to_string(),
        "sect_affinity_match" => match chart_sect {
            Some("day") => "Day sect match".to_string(),
            Some("night") => "Night sect match".to_string(),
            _ => "Sect match".to_string(),
        },
        "sect_affinity_mismatch" => "Sect mismatch".to_string(),
        other => other.replace('_', " "),
    }
}

pub fn push_unique(out: &mut Vec<String>, value: String) {
    if !out.iter().any(|existing| existing == &value) {
        out.push(value);
    }
}

pub fn dignity_effect(dignity_type: &str) -> &'static str {
    match dignity_type {
        "domicile" => "Strong functional expression",
        "exaltation" => "Constructive emphasis",
        "detriment" => "Challenged functional expression",
        "fall" => "Weakened expression",
        _ => "Notable dignity context",
    }
}

pub fn chart_sect_label(sect: &str) -> String {
    match sect {
        "day" => "Day chart".to_string(),
        "night" => "Night chart".to_string(),
        _ => sect.to_string(),
    }
}

pub fn hemisphere_dominant_area(hint: &str, above: i32, below: i32) -> String {
    if hint.contains("private") || hint.contains("interior") || below > above {
        "Below horizon".to_string()
    } else if above > below {
        "Above horizon".to_string()
    } else {
        "Balanced hemispheres".to_string()
    }
}

pub fn reading_slot_section(slot: &str, title: &str) -> String {
    match slot {
        "core_identity" => "Core identity".to_string(),
        "dominant_cluster" => "Dominant repeated theme".to_string(),
        "main_tension_or_support" => "Main dynamic aspect".to_string(),
        "expression_style" => "Expression style".to_string(),
        "background_factors" => "Background factors".to_string(),
        _ => title.to_string(),
    }
}

pub fn axis_balance_label(polarity_balance: &str, primary_house: i32, secondary_house: i32) -> String {
    match polarity_balance {
        "primary_dominant" => format!("Mainly house {primary_house}"),
        "secondary_dominant" => format!("Mainly house {secondary_house}"),
        "balanced" => format!("Balanced houses {primary_house} and {secondary_house}"),
        _ => format!("Mainly house {primary_house}"),
    }
}

pub fn axis_importance(score: f64) -> &'static str {
    if score >= 0.85 {
        "very high"
    } else if score >= 0.6 {
        "high"
    } else {
        "moderate"
    }
}

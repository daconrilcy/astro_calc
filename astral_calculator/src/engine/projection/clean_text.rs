//! Module astral_calculator\src\engine\projection\clean_text.rs du moteur astral_calculator.

use std::collections::HashMap;

/// Fonction title_case_sign.
pub fn title_case_sign(sign_code: &str) -> String {
    let mut chars = sign_code.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Fonction importance_label.
pub fn importance_label(score: f64) -> &'static str {
    if score >= 0.85 {
        "Very high"
    } else if score >= 0.65 {
        "High"
    } else if score >= 0.45 {
        "Moderate"
    } else {
        "Low"
    }
}

/// Fonction accidental_overall_label.
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

/// Fonction humanize_reason.
pub fn humanize_reason(reason: &str, object_names: &HashMap<String, String>) -> String {
    let object_label = |code: &str| {
        object_names
            .get(code)
            .cloned()
            .unwrap_or_else(|| title_case_sign(code))
    };

    if let Some((obj, rest)) = reason.split_once('_') {
        match (obj, rest) {
            (
                "sun" | "moon" | "mercury" | "venus" | "mars" | "jupiter" | "saturn" | "uranus"
                | "neptune" | "pluto",
                "in_sign",
            ) => {
                return format!("{} in sign", object_label(obj));
            }
            (obj, "in_house") => return format!("{} in house", object_label(obj)),
            (obj, "domicile") => return format!("{} in domicile", object_label(obj)),
            (sign, "emphasis") => return format!("{} emphasis", title_case_sign(sign)),
            _ => {}
        }
    }

    match reason {
        "multiple_objects" => {
            "Several chart factors are concentrated in the same sign and house".to_string()
        }
        "cluster" => "Dominant house cluster".to_string(),
        "sign_house_cluster" => {
            "Several chart factors are concentrated in the same sign and house".to_string()
        }
        "saturn_domicile" => "Saturn in domicile".to_string(),
        "placement" => "Strong placement".to_string(),
        "cluster_participant" => "Participant in dominant theme".to_string(),
        "accidental_context" => "Reinforced or modified by accidental conditions".to_string(),
        "ascendant_in_house" => "Ascendant highlights this house".to_string(),
        "dominant_house" => "Dominant house emphasis".to_string(),
        "active_signal" => "Active chart signal".to_string(),
        "rulership_context" => "Supported by rulership links".to_string(),
        "resources_theme" => "Resources theme emphasized".to_string(),
        "shared_resources_theme" => "Shared resources theme emphasized".to_string(),
        "identity_theme" => "Identity theme emphasized".to_string(),
        "relationships_theme" => "Partnership theme emphasized".to_string(),
        "roots_theme" => "Roots theme emphasized".to_string(),
        "career_theme" => "Career theme emphasized".to_string(),
        "ascendant_angle_in_house" => "Ascendant emphasizes this house".to_string(),
        "descendant_angle_in_house" => "Descendant emphasizes this house".to_string(),
        "ic_angle_in_house" => "The IC emphasizes this house".to_string(),
        "mc_angle_in_house" => "The Midheaven emphasizes this house".to_string(),
        "ic_in_house" => "The IC highlights this house".to_string(),
        "mc_in_house" => "The Midheaven highlights this house".to_string(),
        "descendant_in_house" => "Descendant highlights this house".to_string(),
        "jupiter_exaltation" => "Jupiter in exaltation".to_string(),
        "cross_axis_aspect" => "A major aspect connects both sides of this house axis".to_string(),
        "sun_luminary_in_house" => "The Sun highlights this house".to_string(),
        "moon_luminary_in_house" => "The Moon highlights this house".to_string(),
        "strong_aspect_participant" => "Involved in a strong major aspect".to_string(),
        "domicile" => "In domicile".to_string(),
        "exaltation" => "In exaltation".to_string(),
        "detriment" => "In detriment".to_string(),
        "fall" => "In fall".to_string(),
        code if code.chars().all(|c| c.is_ascii_lowercase() || c == '_') => {
            if let Some((obj, "domicile")) = code.split_once('_') {
                return format!("{} in domicile", object_label(obj));
            }
            code.replace('_', " ")
        }
        code => code.to_string(),
    }
}

/// Fonction humanize_condition.
pub fn humanize_condition(code: &str, chart_sect: Option<&str>) -> String {
    match code {
        "angular_house" => "Angular house".to_string(),
        "succedent_house" => "Succedent house".to_string(),
        "cadent_house" => "Cadent house".to_string(),
        "below_horizon" => "Below horizon".to_string(),
        "above_horizon" => "Above horizon".to_string(),
        "on_horizon" => "On horizon".to_string(),
        "retrograde_motion" => "Retrograde motion".to_string(),
        "stationary_motion" => "Stationary motion".to_string(),
        "near_ascendant" => "Close to the Ascendant".to_string(),
        "near_descendant" => "Close to the Descendant".to_string(),
        "near_mc" => "Close to the Midheaven".to_string(),
        "near_ic" => "Close to the IC".to_string(),
        "sect_affinity_match" => match chart_sect {
            Some("day") => "Day sect match".to_string(),
            Some("night") => "Night sect match".to_string(),
            _ => "Sect match".to_string(),
        },
        "sect_affinity_mismatch" => {
            "Sect mismatch: does not match the chart's day/night sect".to_string()
        }
        "sect_affinity_variable_unresolved" => "Variable sect affinity".to_string(),
        other => other.replace('_', " "),
    }
}

/// Fonction humanize_dynamic_quality.
pub fn humanize_dynamic_quality(quality: &str) -> String {
    match quality {
        "tension" => "Tension".to_string(),
        "flow" => "Flow".to_string(),
        "adjustment" => "Adjustment".to_string(),
        "symbolic" => "Symbolic".to_string(),
        "integration" => "Integration".to_string(),
        "intensification" => "Intensification".to_string(),
        "contextual" => "Contextual".to_string(),
        other => title_case_sign(other),
    }
}

/// Fonction humanize_valence.
pub fn humanize_valence(valence: &str) -> String {
    match valence {
        "polarizing" => "Polarizing".to_string(),
        "supportive" => "Supportive".to_string(),
        "harmonious" => "Harmonious".to_string(),
        "dynamic_challenging" => "Dynamic challenging".to_string(),
        "minor_friction" => "Minor friction".to_string(),
        "indirect_tension" => "Indirect tension".to_string(),
        "adjustment" => "Adjustment".to_string(),
        "subtle_adjustment" => "Subtle adjustment".to_string(),
        "creative" | "refined_creative" | "creative_ordering" => "Creative".to_string(),
        "symbolic_fated" => "Symbolic".to_string(),
        "spiritual_integration" => "Integrating".to_string(),
        other => title_case_sign(other),
    }
}

/// Fonction humanize_phase.
pub fn humanize_phase(phase: &str) -> String {
    match phase {
        "separating" => "Separating".to_string(),
        "applying" => "Applying".to_string(),
        "exact" => "Exact".to_string(),
        other => title_case_sign(other),
    }
}

/// Fonction dignity_meaning.
pub fn dignity_meaning(dignity_type: &str) -> &'static str {
    match dignity_type {
        "domicile" => "Strong functional expression",
        "exaltation" => "Constructive emphasis",
        "detriment" => "Challenged functional expression",
        "fall" => "Weakened expression",
        _ => "Notable dignity context",
    }
}

/// Fonction chart_sect_label.
pub fn chart_sect_label(sect: &str) -> String {
    match sect {
        "day" => "Day chart".to_string(),
        "night" => "Night chart".to_string(),
        _ => sect.to_string(),
    }
}

/// Fonction hemisphere_dominant_area.
pub fn hemisphere_dominant_area(hint: &str, above: i32, below: i32) -> String {
    if hint.contains("private") || hint.contains("interior") || below > above {
        "Below horizon".to_string()
    } else if above > below {
        "Above horizon".to_string()
    } else {
        "Balanced hemispheres".to_string()
    }
}

/// Fonction reading_slot_section.
pub fn reading_slot_section(slot: &str, title: &str) -> String {
    match slot {
        "core_identity" => "Core identity".to_string(),
        "dominant_cluster" => "Dominant theme".to_string(),
        "main_tension_or_support" => "Main dynamic".to_string(),
        "expression_style" => "Expression style".to_string(),
        "background_factors" => "Background factors".to_string(),
        _ => title.to_string(),
    }
}

/// Fonction axis_balance_label.
pub fn axis_balance_label(
    polarity_balance: &str,
    primary_house: i32,
    secondary_house: i32,
) -> String {
    match polarity_balance {
        "primary_dominant" => format!("Mainly house {primary_house}"),
        "secondary_dominant" => format!("Mainly house {secondary_house}"),
        "balanced" => format!("Balanced houses {primary_house} and {secondary_house}"),
        _ => format!("Mainly house {primary_house}"),
    }
}

/// Fonction axis_importance.
pub fn axis_importance(score: f64) -> &'static str {
    importance_label(score)
}

/// Fonction limit_keywords.
pub fn limit_keywords(keywords: &[String], limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    for kw in keywords {
        let normalized = kw.trim().to_string();
        if normalized.is_empty() {
            continue;
        }
        if out
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&normalized))
        {
            continue;
        }
        out.push(normalized);
        if out.len() >= limit {
            break;
        }
    }
    out
}

/// Fonction clean_semantic_tags.
pub fn clean_semantic_tags(tags: &[String], limit: usize) -> Vec<String> {
    let filtered: Vec<String> = tags
        .iter()
        .filter(|tag| !is_technical_keyword(tag))
        .map(|tag| tag.replace('_', " "))
        .collect();
    limit_keywords(&filtered, limit)
}

/// Fonction is_technical_keyword.
pub fn is_technical_keyword(tag: &str) -> bool {
    matches!(
        tag,
        "aspect"
            | "major"
            | "minor"
            | "opposition"
            | "conjunction"
            | "trine"
            | "square"
            | "sextile"
            | "tension"
            | "flow"
            | "polarizing"
            | "high_strength"
            | "low_strength"
            | "lunar phase"
            | "sun moon cycle"
    ) || tag.contains("waxing")
        || tag.contains("waning")
        || tag.ends_with("_code")
}

/// Fonction push_unique.
pub fn push_unique(out: &mut Vec<String>, value: String) {
    if !out
        .iter()
        .any(|existing: &String| existing.eq_ignore_ascii_case(&value))
    {
        out.push(value);
    }
}

/// Fonction humanize_theme_code.
pub fn humanize_theme_code(theme_code: &str) -> String {
    match theme_code {
        "shared_resources" => "Shared resources".to_string(),
        "resources" => "Resources".to_string(),
        "identity" => "Identity".to_string(),
        "relationships" => "Partnership".to_string(),
        "roots" => "Roots".to_string(),
        "career" => "Career".to_string(),
        "communication" => "Communication".to_string(),
        "transformation" => "Transformation".to_string(),
        code => {
            if code.contains('_') {
                code.split('_')
                    .map(title_case_sign)
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                title_case_sign(code)
            }
        }
    }
}

/// Rewrites engine `interpretive_hint` parenthetical theme codes for LLM-facing text.
pub fn humanize_axis_summary(hint: &str, theme_in_parens: &[(String, String)]) -> String {
    let mut summary = hint.to_string();
    for (code, label) in theme_in_parens {
        summary = summary.replace(&format!("({code})"), &format!("({label})"));
    }
    summary = humanize_residual_snake_case(&summary);
    summary
}

/// Fonction humanize_residual_snake_case.
fn humanize_residual_snake_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            out.push(ch);
            let mut token = String::new();
            while let Some(&next) = chars.peek() {
                if next == ')' {
                    break;
                }
                token.push(chars.next().expect("peeked"));
            }
            if token.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !token.is_empty() {
                out.push_str(&humanize_theme_code(&token));
            } else {
                out.push_str(&token);
            }
            if chars.peek() == Some(&')') {
                out.push(chars.next().expect(")"));
            }
            continue;
        }
        out.push(ch);
    }
    out
}

/// Fonction is_unremarkable_motion_condition.
pub fn is_unremarkable_motion_condition(label: &str, motion: Option<&str>) -> bool {
    let lower = label.to_ascii_lowercase();
    if lower == "direct motion" || lower == "direct" {
        return true;
    }
    if let Some(motion_label) = motion {
        if motion_label.eq_ignore_ascii_case(label) {
            return true;
        }
    }
    false
}

/// Fonction humanize_motion_label.
pub fn humanize_motion_label(label: &str) -> String {
    match label {
        "Direct" | "direct" => "Direct motion".to_string(),
        "Retrograde" | "retrograde" => "Retrograde motion".to_string(),
        "Stationary" | "stationary" => "Stationary motion".to_string(),
        other => other.to_string(),
    }
}

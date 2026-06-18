//! Module astral_calculator\src\engine\projection\clean_text.rs du moteur astral_calculator.

use std::collections::HashMap;

use crate::domain::{
    AccidentalDignityConditionReference, AnglePointReference, BasicProjectionReason,
    EssentialDignityRuleReference, HouseReference, MotionStateReference, ProjectionLabelDefinition,
    ProjectionReasonDefinition,
};
use crate::shared::error::RuntimeError;

/// Structure ProjectionTextCatalog.
pub struct ProjectionTextCatalog<'a> {
    reason_definitions: HashMap<&'a str, &'a ProjectionReasonDefinition>,
    projection_labels: HashMap<(&'a str, &'a str), &'a ProjectionLabelDefinition>,
    houses_by_number: HashMap<i32, &'a HouseReference>,
    houses_by_theme_code: HashMap<&'a str, &'a HouseReference>,
    angle_points: HashMap<String, &'a AnglePointReference>,
    motion_states_by_code: HashMap<&'a str, &'a MotionStateReference>,
    motion_states_by_label: HashMap<String, &'a MotionStateReference>,
    accidental_conditions: HashMap<&'a str, &'a AccidentalDignityConditionReference>,
    dignity_labels: HashMap<&'a str, &'a str>,
}

impl<'a> ProjectionTextCatalog<'a> {
    /// Fonction build.
    pub fn build(
        projection_reason_definitions: &'a [ProjectionReasonDefinition],
        projection_label_definitions: &'a [ProjectionLabelDefinition],
        house_references: &'a [HouseReference],
        angle_points: &'a [AnglePointReference],
        motion_states: &'a [MotionStateReference],
        accidental_condition_definitions: &'a [AccidentalDignityConditionReference],
        essential_dignity_rules: &'a [EssentialDignityRuleReference],
    ) -> Self {
        let mut indexed_angle_points = HashMap::new();
        for angle in angle_points {
            for key in [
                angle.code.to_ascii_lowercase(),
                angle.chart_object_code.to_ascii_lowercase(),
                angle.full_name.to_ascii_lowercase(),
            ] {
                indexed_angle_points.entry(key).or_insert(angle);
            }
        }

        Self {
            reason_definitions: projection_reason_definitions
                .iter()
                .map(|definition| (definition.reason_code.as_str(), definition))
                .collect(),
            projection_labels: projection_label_definitions
                .iter()
                .map(|definition| {
                    (
                        (
                            definition.label_family.as_str(),
                            definition.label_code.as_str(),
                        ),
                        definition,
                    )
                })
                .collect(),
            houses_by_number: house_references
                .iter()
                .map(|reference| (reference.number, reference))
                .collect(),
            houses_by_theme_code: house_references
                .iter()
                .map(|reference| (reference.theme_code.as_str(), reference))
                .collect(),
            angle_points: indexed_angle_points,
            motion_states_by_code: motion_states
                .iter()
                .map(|reference| (reference.code.as_str(), reference))
                .collect(),
            motion_states_by_label: motion_states
                .iter()
                .map(|reference| (reference.label.to_ascii_lowercase(), reference))
                .collect(),
            accidental_conditions: accidental_condition_definitions
                .iter()
                .map(|definition| (definition.condition_code.as_str(), definition))
                .collect(),
            dignity_labels: essential_dignity_rules
                .iter()
                .map(|rule| (rule.dignity_type.as_str(), rule.dignity_label.as_str()))
                .collect(),
        }
    }

    /// Fonction reason_definition.
    pub fn reason_definition(
        &self,
        reason_code: &str,
    ) -> Result<&ProjectionReasonDefinition, RuntimeError> {
        self.reason_definitions
            .get(reason_code)
            .copied()
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionReasonDefinition(format!(
                    "missing projection reason definition for reason_code '{reason_code}'"
                ))
            })
    }

    /// Fonction projection_label.
    pub fn projection_label(
        &self,
        label_family: &str,
        label_code: &str,
    ) -> Result<&str, RuntimeError> {
        self.projection_labels
            .get(&(label_family, label_code))
            .map(|definition| definition.label_template_en.as_str())
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing projection label definition for family '{label_family}' and code '{label_code}'"
                ))
            })
    }

    /// Fonction house_by_number.
    pub fn house_by_number(&self, house_number: i32) -> Result<&HouseReference, RuntimeError> {
        self.houses_by_number
            .get(&house_number)
            .copied()
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing house reference for house_number '{house_number}'"
                ))
            })
    }

    /// Fonction house_by_theme_code.
    pub fn house_by_theme_code(&self, theme_code: &str) -> Result<&HouseReference, RuntimeError> {
        self.houses_by_theme_code
            .get(theme_code)
            .copied()
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing house reference for theme_code '{theme_code}'"
                ))
            })
    }

    /// Fonction house_label.
    pub fn house_label(&self, house_number: i32, theme_code: &str) -> Result<String, RuntimeError> {
        let house = self.house_by_number(house_number)?;
        if house.theme_code != theme_code {
            return Err(RuntimeError::InvalidProjectionLabelDefinition(format!(
                "house reference mismatch for house_number '{house_number}' and theme_code '{theme_code}'"
            )));
        }
        Ok(house.name.clone())
    }

    /// Fonction angle_point.
    pub fn angle_point(&self, angle_code: &str) -> Result<&AnglePointReference, RuntimeError> {
        self.angle_points
            .get(&angle_code.to_ascii_lowercase())
            .copied()
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing angle point reference for code '{angle_code}'"
                ))
            })
    }

    /// Fonction angle_display_label.
    pub fn angle_display_label(&self, angle_code: &str) -> Result<String, RuntimeError> {
        let angle = self.angle_point(angle_code)?;
        let projection_code = match angle.chart_object_code.as_str() {
            "ascendant" | "descendant" | "mc" | "ic" => angle.chart_object_code.as_str(),
            _ => match angle.code.as_str() {
                "asc" => "ascendant",
                "dsc" => "descendant",
                code => code,
            },
        };
        Ok(self
            .projection_label("angle_display", projection_code)?
            .to_string())
    }

    /// Fonction motion_state.
    pub fn motion_state(&self, label_or_code: &str) -> Result<&MotionStateReference, RuntimeError> {
        self.motion_states_by_code
            .get(label_or_code)
            .copied()
            .or_else(|| {
                self.motion_states_by_label
                    .get(&label_or_code.to_ascii_lowercase())
                    .copied()
            })
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing motion state reference for '{label_or_code}'"
                ))
            })
    }

    /// Fonction accidental_condition.
    pub fn accidental_condition(
        &self,
        condition_code: &str,
    ) -> Result<&AccidentalDignityConditionReference, RuntimeError> {
        self.accidental_conditions
            .get(condition_code)
            .copied()
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing accidental condition reference for condition_code '{condition_code}'"
                ))
            })
    }

    /// Fonction dignity_label.
    pub fn dignity_label(&self, dignity_type: &str) -> Result<&str, RuntimeError> {
        self.dignity_labels
            .get(dignity_type)
            .copied()
            .ok_or_else(|| {
                RuntimeError::InvalidProjectionLabelDefinition(format!(
                    "missing essential dignity label for dignity_type '{dignity_type}'"
                ))
            })
    }
}

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

/// Fonction render_projection_reason.
pub fn render_projection_reason(
    reason: &BasicProjectionReason,
    resolver: &ProjectionTextCatalog<'_>,
    object_names: &HashMap<String, String>,
) -> Result<String, RuntimeError> {
    let definition = resolver.reason_definition(&reason.reason_code)?;

    let object_value = reason
        .object_code
        .as_deref()
        .and_then(|code| object_names.get(code))
        .cloned()
        .unwrap_or_else(|| {
            reason
                .object_code
                .as_deref()
                .map(title_case_sign)
                .unwrap_or_default()
        });
    let dignity_value = reason
        .dignity_type
        .as_deref()
        .map(|code| {
            resolver
                .dignity_label(code)
                .map(|label| label.to_ascii_lowercase())
        })
        .transpose()?
        .unwrap_or_default();
    let sign_value = reason
        .sign_code
        .as_deref()
        .map(title_case_sign)
        .unwrap_or_default();
    let theme_value = reason
        .theme_code
        .as_deref()
        .map(|code| {
            resolver
                .house_by_theme_code(code)
                .map(|reference| reference.name.clone())
        })
        .transpose()?
        .unwrap_or_default();
    let angle_value = reason
        .angle_code
        .as_deref()
        .map(|code| resolver.angle_display_label(code))
        .transpose()?
        .unwrap_or_default();

    let mut rendered = definition
        .label_template_en
        .replace("{object}", &object_value)
        .replace("{dignity}", &dignity_value)
        .replace("{sign}", &sign_value)
        .replace("{theme}", &theme_value)
        .replace("{angle}", &angle_value);

    if definition.requires_object
        && !object_value.is_empty()
        && !rendered
            .to_ascii_lowercase()
            .contains(&object_value.to_ascii_lowercase())
    {
        rendered = format!("{object_value} {}", rendered.trim_start());
    }

    Ok(rendered)
}

/// Fonction humanize_condition.
pub fn humanize_condition(
    code: &str,
    chart_sect: Option<&str>,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    if code == "sect_affinity_match" {
        let variant = match chart_sect {
            Some("day") => "sect_affinity_match_day",
            Some("night") => "sect_affinity_match_night",
            _ => "sect_affinity_match_default",
        };
        return Ok(resolver
            .projection_label("condition_variant", variant)?
            .to_string());
    }
    Ok(resolver.accidental_condition(code)?.label.clone())
}

/// Fonction humanize_dynamic_quality.
pub fn humanize_dynamic_quality(
    quality: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    Ok(resolver
        .projection_label("dynamic_quality", quality)?
        .to_string())
}

/// Fonction humanize_valence.
pub fn humanize_valence(
    valence: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    Ok(resolver.projection_label("valence", valence)?.to_string())
}

/// Fonction humanize_phase.
pub fn humanize_phase(
    phase: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    Ok(resolver.projection_label("phase", phase)?.to_string())
}

/// Fonction dignity_meaning.
pub fn dignity_meaning(
    dignity_type: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    let code = if resolver.dignity_labels.contains_key(dignity_type) {
        dignity_type
    } else {
        "default"
    };
    Ok(resolver
        .projection_label("dignity_meaning", code)?
        .to_string())
}

/// Fonction chart_sect_label.
pub fn chart_sect_label(
    sect: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    Ok(resolver.projection_label("chart_sect", sect)?.to_string())
}

/// Fonction hemisphere_dominant_area.
pub fn hemisphere_dominant_area(
    hint: &str,
    above: i32,
    below: i32,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    let code = if hint.contains("private") || hint.contains("interior") || below > above {
        "below_horizon"
    } else if above > below {
        "above_horizon"
    } else {
        "balanced"
    };
    Ok(resolver
        .projection_label("hemisphere_area", code)?
        .to_string())
}

/// Fonction reading_slot_section.
pub fn reading_slot_section(
    slot: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    Ok(resolver.projection_label("reading_slot", slot)?.to_string())
}

/// Fonction axis_balance_label.
pub fn axis_balance_label(
    polarity_balance: &str,
    primary_house: i32,
    secondary_house: i32,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    Ok(resolver
        .projection_label("axis_balance", polarity_balance)?
        .replace("{primary_house}", &primary_house.to_string())
        .replace("{secondary_house}", &secondary_house.to_string()))
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

#[allow(dead_code)]
/// Fonction humanize_theme_code.
pub fn humanize_theme_code(theme_code: &str) -> String {
    if theme_code.contains('_') {
        theme_code
            .split('_')
            .map(title_case_sign)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        title_case_sign(theme_code)
    }
}

/// Rewrites engine `interpretive_hint` parenthetical theme codes for LLM-facing text.
pub fn humanize_axis_summary(
    hint: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    let mut out = String::with_capacity(hint.len());
    let mut chars = hint.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '(' {
            out.push(ch);
            continue;
        }
        out.push(ch);
        let mut token = String::new();
        while let Some(&next) = chars.peek() {
            if next == ')' {
                break;
            }
            token.push(chars.next().expect("peeked"));
        }
        if token.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !token.is_empty() {
            let label = resolver
                .house_by_theme_code(&token)
                .map(|reference| reference.name.clone())
                .map_err(|_| {
                    RuntimeError::InvalidProjectionLabelDefinition(format!(
                        "unresolved canonical theme token '{token}' in axis summary"
                    ))
                })?;
            out.push_str(&label);
        } else {
            out.push_str(&token);
        }
        if chars.peek() == Some(&')') {
            out.push(chars.next().expect(")"));
        }
    }
    Ok(out)
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
pub fn humanize_motion_label(
    label_or_code: &str,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<String, RuntimeError> {
    let motion = resolver.motion_state(label_or_code)?;
    Ok(resolver
        .projection_label("motion_display", &motion.code)?
        .to_string())
}

use super::*;
use crate::shared::error::RuntimeError;

pub(super) fn build_reading_order(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    dynamics: &LlmDynamics,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<Vec<LlmReadingOrderItem>, RuntimeError> {
    payload
        .reading_plan
        .iter()
        .map(|item| {
            let focus = match item.slot.as_str() {
                "dominant_cluster" => {
                    dominant_cluster_reading_focus(payload, profile, dynamics, resolver)
                }
                "main_tension_or_support" => Ok(main_dynamic_reading_focus(dynamics, payload)),
                _ => Ok(item
                    .primary_signal_keys
                    .iter()
                    .filter_map(|key| reading_focus_from_signal(payload, key))
                    .collect()),
            }?;
            let focus = limit_keywords(&focus, profile.max_keywords_per_item);
            Ok(LlmReadingOrderItem {
                section: super::super::humanize::reading_slot_section(&item.slot, resolver)?,
                focus,
            })
        })
        .collect()
}

fn dominant_cluster_reading_focus(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    _dynamics: &LlmDynamics,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<Vec<String>, RuntimeError> {
    let mut focus = Vec::new();
    if let Some(sign) = payload.chart_emphasis.dominant_signs.first() {
        push_unique(
            &mut focus,
            format!(
                "{} emphasis",
                super::super::humanize::title_case_sign(&sign.sign_code)
            ),
        );
    }
    if let Some(house) = payload.chart_emphasis.dominant_houses.first() {
        let theme = house_ref_from_payload(house.house_number, &house.theme_code, resolver)?;
        push_unique(
            &mut focus,
            format!(
                "House {} {} emphasis",
                theme.number,
                theme.theme.to_lowercase()
            ),
        );
    }
    if let Some(object) = payload.chart_emphasis.dominant_objects.first() {
        let name = payload
            .positions
            .iter()
            .find(|p| p.object_code == object.object_code)
            .map(|p| p.object_name.clone())
            .unwrap_or_else(|| super::super::humanize::title_case_sign(&object.object_code));
        push_unique(&mut focus, format!("{name} strength"));
    }
    if focus.is_empty() {
        for key in payload
            .reading_plan
            .iter()
            .find(|item| item.slot == "dominant_cluster")
            .into_iter()
            .flat_map(|item| item.primary_signal_keys.iter())
        {
            if let Some(label) = reading_focus_from_signal(payload, key) {
                push_unique(&mut focus, label);
            }
            if focus.len() >= profile.max_keywords_per_item {
                break;
            }
        }
    }
    Ok(focus)
}

fn main_dynamic_reading_focus(dynamics: &LlmDynamics, payload: &BasicPayload) -> Vec<String> {
    if let Some(aspect) = dynamics.major_aspects.first() {
        return vec![aspect.aspect.clone()];
    }
    payload
        .reading_plan
        .iter()
        .find(|item| item.slot == "main_tension_or_support")
        .into_iter()
        .flat_map(|item| item.primary_signal_keys.iter())
        .filter_map(|key| reading_focus_from_signal(payload, key))
        .collect()
}

fn reading_focus_from_signal(payload: &BasicPayload, signal_key: &str) -> Option<String> {
    if let Some(signal) = payload.signals.iter().find(|s| s.signal_key == signal_key) {
        return Some(signal.title.clone());
    }
    if signal_key.starts_with("object_position:") {
        let code = signal_key.trim_start_matches("object_position:");
        return payload
            .positions
            .iter()
            .find(|p| p.object_code == code)
            .map(|p| p.object_name.clone());
    }
    if let Some(rest) = signal_key.strip_prefix("angle:") {
        let parts: Vec<_> = rest.split(':').collect();
        if parts.len() >= 2 {
            let angle = super::super::humanize::title_case_sign(parts[0]);
            let sign = super::super::humanize::title_case_sign(parts[2]);
            return Some(format!("{angle} in {sign}"));
        }
    }
    if let Some(rest) = signal_key.strip_prefix("aspect:") {
        let parts: Vec<_> = rest.split(':').collect();
        if parts.len() >= 3 {
            let a = super::super::humanize::title_case_sign(parts[0]);
            let b = super::super::humanize::title_case_sign(parts[1]);
            let aspect = super::super::humanize::title_case_sign(parts[2]);
            return Some(format!("{a} {aspect} {b}"));
        }
    }
    if signal_key.starts_with("cluster:") {
        return Some("Dominant cluster".to_string());
    }
    if signal_key.starts_with("dignity:") {
        let parts: Vec<_> = signal_key.split(':').collect();
        if parts.len() >= 4 {
            return Some(format!(
                "{} {}",
                super::super::humanize::title_case_sign(parts[1]),
                super::super::humanize::title_case_sign(parts[2])
            ));
        }
    }
    None
}

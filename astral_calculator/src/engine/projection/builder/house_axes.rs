use super::*;
use crate::domain::BasicHouseAxisEmphasis;
use crate::shared::error::RuntimeError;

pub(super) fn build_house_axes(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    axis_refs: &[HouseAxisReference],
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<Vec<LlmHouseAxis>, RuntimeError> {
    let object_names = object_name_map(payload);
    payload
        .house_axis_emphasis
        .iter()
        .take(profile.max_house_axes)
        .map(|axis| house_axis_to_llm(axis, axis_refs, profile, &object_names, resolver))
        .collect()
}

fn house_axis_to_llm(
    axis: &BasicHouseAxisEmphasis,
    axis_refs: &[HouseAxisReference],
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmHouseAxis, RuntimeError> {
    let axis_title = super::super::axis_labels::house_axis_label(&axis.axis_code, axis_refs);
    let houses: Vec<LlmHouseRef> = axis
        .house_scores
        .iter()
        .map(|score| house_ref_from_payload(score.house_number, &score.theme_code, resolver))
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    let supporting_factors = dedupe_rendered_reasons(
        &axis.reason_details,
        resolver,
        object_names,
        profile.max_keywords_per_item,
    )?;

    let summary = super::super::humanize::humanize_axis_summary(&axis.interpretive_hint, resolver)?;

    Ok(LlmHouseAxis {
        axis: axis_title,
        houses,
        balance: super::super::humanize::axis_balance_label(
            &axis.polarity_balance,
            axis.primary_house,
            axis.secondary_house,
            resolver,
        )?,
        importance: super::super::humanize::axis_importance(axis.axis_score).to_string(),
        summary,
        supporting_factors,
    })
}

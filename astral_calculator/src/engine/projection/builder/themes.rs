use super::*;
use crate::shared::error::RuntimeError;

pub(super) fn build_dominant_themes(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmDominantThemes, RuntimeError> {
    let signs = payload
        .chart_emphasis
        .dominant_signs
        .iter()
        .take(profile.max_dominant_signs)
        .map(|entry| {
            let sign = super::super::humanize::title_case_sign(&entry.sign_code);
            Ok(LlmDominantSign {
                name: sign.clone(),
                importance: super::super::humanize::importance_label(entry.score).to_string(),
                supporting_factors: dedupe_rendered_reasons(
                    &entry.reason_details,
                    resolver,
                    object_names,
                    profile.max_keywords_per_item,
                )?,
                keywords: sign_keywords_from_positions(
                    payload,
                    &entry.sign_code,
                    profile.max_keywords_per_item,
                ),
                score: profile.include_scores.then_some(entry.score),
            })
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    let houses = payload
        .chart_emphasis
        .dominant_houses
        .iter()
        .take(profile.max_dominant_houses)
        .map(|entry| {
            let house_ref =
                house_ref_from_payload(entry.house_number, &entry.theme_code, resolver)?;
            Ok(LlmDominantHouse {
                number: house_ref.number,
                theme: house_ref.theme,
                importance: super::super::humanize::importance_label(entry.score).to_string(),
                supporting_factors: dedupe_rendered_reasons(
                    &entry.reason_details,
                    resolver,
                    object_names,
                    profile.max_keywords_per_item,
                )?,
                score: profile.include_scores.then_some(entry.score),
            })
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    let objects = payload
        .chart_emphasis
        .dominant_objects
        .iter()
        .take(profile.max_dominant_objects)
        .map(|entry| {
            Ok(LlmDominantObject {
                name: object_names
                    .get(&entry.object_code)
                    .cloned()
                    .unwrap_or_else(|| super::super::humanize::title_case_sign(&entry.object_code)),
                importance: super::super::humanize::importance_label(entry.score).to_string(),
                supporting_factors: dedupe_rendered_reasons(
                    &entry.reason_details,
                    resolver,
                    object_names,
                    profile.max_keywords_per_item,
                )?,
                score: profile.include_scores.then_some(entry.score),
            })
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

    Ok(LlmDominantThemes {
        signs,
        houses,
        objects,
    })
}

fn sign_keywords_from_positions(
    payload: &BasicPayload,
    sign_code: &str,
    limit: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    for position in payload
        .positions
        .iter()
        .filter(|p| p.sign_code == sign_code)
    {
        for kw in limited_keywords(position, limit) {
            push_unique(&mut out, kw);
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

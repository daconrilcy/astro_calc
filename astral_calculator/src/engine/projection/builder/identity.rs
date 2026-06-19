use super::*;
use crate::shared::error::RuntimeError;

pub(super) fn build_core_identity(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmCoreIdentity, RuntimeError> {
    Ok(LlmCoreIdentity {
        sun: mobile_body(payload, "sun", profile, resolver)?,
        moon: mobile_body(payload, "moon", profile, resolver)?,
        ascendant: build_ascendant_core(payload, profile, object_names),
    })
}

fn mobile_body(
    payload: &BasicPayload,
    code: &str,
    profile: &LlmProjectionProfile,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<Option<LlmCoreBody>, RuntimeError> {
    let Some(position) = payload.positions.iter().find(|p| p.object_code == code) else {
        return Ok(None);
    };
    Ok(Some(LlmCoreBody {
        placement: placement_from_position(position, profile.include_degrees, resolver)?,
        keywords: limited_keywords(position, profile.max_keywords_per_item),
        conditions: position_conditions(position, chart_sect(payload), profile, resolver)?,
        importance: "high".to_string(),
    }))
}

fn build_ascendant_core(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> Option<LlmAscendantBody> {
    let asc = payload
        .angles
        .iter()
        .find(|a| a.angle_code == "ascendant")?;
    let sign_keywords = payload
        .positions
        .iter()
        .find(|p| p.object_code == "ascendant")
        .map(|p| limited_keywords(p, profile.max_keywords_per_item))
        .unwrap_or_default();

    let ruler = if profile.include_rulership_details {
        payload.rulership_context.ascendant_ruler.as_ref().map(|r| {
            let mut rulers = LlmAscendantRulers {
                traditional: None,
                modern: None,
            };
            for source in &r.ruler_sources {
                if source.astral_system_code == "traditional" {
                    rulers.traditional = Some(
                        object_names
                            .get(&source.object_code)
                            .cloned()
                            .unwrap_or_else(|| {
                                super::super::humanize::title_case_sign(&source.object_code)
                            }),
                    );
                }
                if source.astral_system_code == "modern" {
                    rulers.modern = Some(
                        object_names
                            .get(&source.object_code)
                            .cloned()
                            .unwrap_or_else(|| {
                                super::super::humanize::title_case_sign(&source.object_code)
                            }),
                    );
                }
            }
            if rulers.traditional.is_none() {
                rulers.traditional = Some(
                    object_names
                        .get(&r.ruler_object_code)
                        .cloned()
                        .unwrap_or_else(|| {
                            super::super::humanize::title_case_sign(&r.ruler_object_code)
                        }),
                );
            }
            rulers
        })
    } else {
        None
    };

    Some(LlmAscendantBody {
        sign: asc.sign_name.clone(),
        keywords: sign_keywords,
        ruler,
        importance: "high".to_string(),
    })
}

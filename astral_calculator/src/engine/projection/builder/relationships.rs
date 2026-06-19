use super::*;

pub(super) fn build_relationship_network(
    payload: &BasicPayload,
    profile: &LlmProjectionProfile,
    object_names: &HashMap<String, String>,
) -> LlmRelationshipNetwork {
    if !profile.include_rulership_details {
        return LlmRelationshipNetwork::default();
    }

    let ascendant_ruler = payload.rulership_context.ascendant_ruler.as_ref().map(|r| {
        let mut traditional = None;
        let mut modern = None;
        for source in &r.ruler_sources {
            if source.astral_system_code == "traditional" {
                traditional = Some(
                    object_names
                        .get(&source.object_code)
                        .cloned()
                        .unwrap_or_else(|| {
                            super::super::humanize::title_case_sign(&source.object_code)
                        }),
                );
            }
            if source.astral_system_code == "modern" {
                modern = Some(
                    object_names
                        .get(&source.object_code)
                        .cloned()
                        .unwrap_or_else(|| {
                            super::super::humanize::title_case_sign(&source.object_code)
                        }),
                );
            }
        }
        if traditional.is_none() {
            traditional = Some(
                object_names
                    .get(&r.ruler_object_code)
                    .cloned()
                    .unwrap_or_else(|| {
                        super::super::humanize::title_case_sign(&r.ruler_object_code)
                    }),
            );
        }
        LlmAscendantRulerNetwork {
            ascendant_sign: super::super::humanize::title_case_sign(&r.sign_code),
            traditional_ruler: traditional,
            modern_ruler: modern,
            main_ruler_placement: ruler_placement_text(payload, &r.ruler_object_code),
        }
    });

    let midheaven_ruler = payload
        .rulership_context
        .mc_ruler
        .as_ref()
        .map(|r| LlmMcRulerNetwork {
            midheaven_sign: super::super::humanize::title_case_sign(&r.sign_code),
            ruler: object_names
                .get(&r.ruler_object_code)
                .cloned()
                .unwrap_or_else(|| super::super::humanize::title_case_sign(&r.ruler_object_code)),
            ruler_placement: ruler_placement_text(payload, &r.ruler_object_code),
        });

    let final_dispositors = payload
        .rulership_context
        .final_dispositors
        .iter()
        .map(|d| LlmFinalDispositor {
            object: object_names
                .get(&d.object_code)
                .cloned()
                .unwrap_or_else(|| super::super::humanize::title_case_sign(&d.object_code)),
            source_objects: d
                .source_objects
                .iter()
                .map(|code| {
                    object_names
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| super::super::humanize::title_case_sign(code))
                })
                .collect(),
        })
        .collect();

    let mutual_receptions = payload
        .rulership_context
        .mutual_receptions
        .iter()
        .map(|m| {
            let objects: Vec<String> = m
                .object_codes
                .iter()
                .map(|code| {
                    object_names
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| super::super::humanize::title_case_sign(code))
                })
                .collect();
            let source_objects = m
                .source_objects
                .iter()
                .map(|code| {
                    object_names
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| super::super::humanize::title_case_sign(code))
                })
                .collect();
            LlmMutualReception {
                objects,
                source_objects,
            }
        })
        .collect();

    LlmRelationshipNetwork {
        ascendant_ruler,
        midheaven_ruler,
        final_dispositors,
        mutual_receptions,
    }
}

fn ruler_placement_text(payload: &BasicPayload, ruler_code: &str) -> String {
    let Some(position) = payload
        .positions
        .iter()
        .find(|p| p.object_code == ruler_code)
    else {
        return super::super::humanize::title_case_sign(ruler_code);
    };
    format!(
        "{} in {}, house {}",
        position.object_name,
        position.sign_name,
        position.house_number.unwrap_or_default()
    )
}

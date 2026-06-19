use super::*;
use crate::shared::error::RuntimeError;

pub(super) fn build_chart(
    payload: &BasicPayload,
    ctx: &LlmProjectionBuildContext<'_>,
    resolver: &ProjectionTextCatalog<'_>,
) -> Result<LlmChart, RuntimeError> {
    let sect = payload
        .chart_context
        .sect
        .chart_sect
        .as_deref()
        .map(|sect| super::super::humanize::chart_sect_label(sect, resolver))
        .transpose()?;
    let hemisphere = payload
        .chart_context
        .hemisphere_emphasis
        .interpretive_hint
        .as_ref()
        .map(|hint| -> Result<LlmHemisphereEmphasis, RuntimeError> {
            Ok(LlmHemisphereEmphasis {
                dominant_area: super::super::humanize::hemisphere_dominant_area(
                    hint,
                    payload
                        .chart_context
                        .hemisphere_emphasis
                        .above_horizon_count,
                    payload
                        .chart_context
                        .hemisphere_emphasis
                        .below_horizon_count,
                    resolver,
                )?,
                summary: hint.clone(),
            })
        })
        .transpose()?;

    Ok(LlmChart {
        chart_type: "Natal chart".to_string(),
        birth: LlmChartBirth {
            datetime_utc: payload.birth_datetime_utc.to_rfc3339(),
            location: ctx.birth_location_label.to_string(),
        },
        calculation: LlmChartCalculation {
            zodiac: ctx.zodiac_label.to_string(),
            coordinates: ctx.coordinate_label.to_string(),
            house_system: ctx.house_system_label.to_string(),
        },
        sect,
        hemisphere_emphasis: hemisphere,
    })
}

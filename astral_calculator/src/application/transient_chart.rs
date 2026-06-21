use chrono::{DateTime, Utc};

use crate::astrology::ephemeris::EphemerisEngine;
use crate::domain::{CalculatedChartFacts, NatalChartInput};
use crate::shared::error::RuntimeError;

use super::chart_context::ChartContextData;

pub fn calculate_transient_chart_facts<E>(
    ephemeris: &E,
    natal_input: &NatalChartInput,
    reference_datetime_utc: DateTime<Utc>,
    product_code: &str,
    chart_context: &ChartContextData,
) -> Result<CalculatedChartFacts, RuntimeError>
where
    E: EphemerisEngine + ?Sized,
{
    let mut transit_input = natal_input.clone();
    transit_input.birth_datetime_utc = reference_datetime_utc;
    transit_input.product_code = Some(product_code.to_string());
    ephemeris.calculate_chart(
        &transit_input,
        &chart_context.chart_objects,
        &chart_context.aspect_definitions,
        &chart_context.house_system,
        &chart_context.references,
    )
}

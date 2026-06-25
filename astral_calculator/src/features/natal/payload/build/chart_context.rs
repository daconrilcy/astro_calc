//! Module astral_calculator\src\features\natal\payload\build\chart_context.rs du moteur astral_calculator.

use serde_json::Value;

use crate::domain::{BasicChartContext, NatalChartInput, ObjectPositionFact};
use crate::features::natal::catalog::BasicPayloadCatalog;

pub(super) fn build_chart_context(
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    contract_version: &str,
    catalog: Option<&BasicPayloadCatalog>,
    locale: &str,
) -> BasicChartContext {
    crate::features::natal::payload::rules::chart_context::build_chart_context(
        input,
        positions,
        contract_version,
        catalog,
        locale,
    )
}

pub(super) fn visibility_context(position: &ObjectPositionFact) -> Value {
    crate::features::natal::payload::rules::chart_context::visibility_context(position)
}

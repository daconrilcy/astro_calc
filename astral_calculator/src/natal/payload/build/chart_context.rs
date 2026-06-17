use serde_json::Value;

use crate::domain::{BasicChartContext, NatalChartInput, ObjectPositionFact};
use crate::natal::catalog::BasicPayloadCatalog;

pub(super) fn build_chart_context(
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    contract_version: &str,
    catalog: Option<&BasicPayloadCatalog>,
) -> BasicChartContext {
    crate::natal::payload::rules::chart_context::build_chart_context(
        input,
        positions,
        contract_version,
        catalog,
    )
}

pub(super) fn visibility_context(position: &ObjectPositionFact) -> Value {
    crate::natal::payload::rules::chart_context::visibility_context(position)
}

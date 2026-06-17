use serde_json::Value;

use crate::catalog::BasicPayloadCatalog;
use crate::domain::{BasicChartContext, NatalChartInput, ObjectPositionFact};

pub(super) fn build_chart_context(
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    contract_version: &str,
    catalog: Option<&BasicPayloadCatalog>,
) -> BasicChartContext {
    crate::features::payload_rules::chart_context::build_chart_context(
        input,
        positions,
        contract_version,
        catalog,
    )
}

pub(super) fn visibility_context(position: &ObjectPositionFact) -> Value {
    crate::features::payload_rules::chart_context::visibility_context(position)
}

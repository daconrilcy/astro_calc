use crate::domain::{
    BasicChartEmphasis, BasicRulershipContext, BasicSignal, DomicileRulerReference,
    ObjectPositionFact,
};

pub(super) fn build_rulership_context(
    positions: &[ObjectPositionFact],
    chart_emphasis: &BasicChartEmphasis,
    rulers: &[DomicileRulerReference],
    signals: &[BasicSignal],
) -> BasicRulershipContext {
    crate::payload_rules::rulership::build_rulership_context(
        positions,
        chart_emphasis,
        rulers,
        signals,
    )
}

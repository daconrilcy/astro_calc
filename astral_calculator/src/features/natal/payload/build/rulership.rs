//! Module astral_calculator\src\features\natal\payload\build\rulership.rs du moteur astral_calculator.

use crate::domain::{
    BasicChartEmphasis, BasicRulershipContext, BasicSignal, DomicileRulerReference,
    ObjectPositionFact,
};

pub(super) fn build_rulership_context(
    positions: &[ObjectPositionFact],
    chart_emphasis: &BasicChartEmphasis,
    rulers: &[DomicileRulerReference],
    signals: &[BasicSignal],
    locale: &str,
) -> BasicRulershipContext {
    crate::features::natal::payload::rules::rulership::build_rulership_context(
        positions,
        chart_emphasis,
        rulers,
        signals,
        locale,
    )
}

//! Module astral_calculator\src\features\natal\payload\build\lunar_phase.rs du moteur astral_calculator.

use crate::domain::{
    BasicLunarPhaseContext, BasicReadingPlanItem, BasicSignal, LunarPhaseReference,
    ObjectPositionFact,
};

pub(super) fn build_lunar_phase_context(
    references: &[LunarPhaseReference],
    positions: &[ObjectPositionFact],
    signals: &[BasicSignal],
    reading_plan: &[BasicReadingPlanItem],
) -> Option<BasicLunarPhaseContext> {
    crate::features::natal::payload::rules::lunar_phase::build_lunar_phase_context(
        references,
        positions,
        signals,
        reading_plan,
    )
}

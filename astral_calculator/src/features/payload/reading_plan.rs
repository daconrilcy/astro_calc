use crate::domain::{BasicReadingPlanItem, BasicSignal};

use super::signal_filters::{
    is_interpretive_aspect_signal, is_interpretive_support_aspect, is_interpretive_tension_aspect,
};
pub(super) fn build_reading_plan(signals: &[BasicSignal]) -> Vec<BasicReadingPlanItem> {
    crate::features::payload_rules::reading_plan::build_reading_plan(
        signals,
        is_interpretive_aspect_signal,
        is_interpretive_support_aspect,
        is_interpretive_tension_aspect,
    )
}

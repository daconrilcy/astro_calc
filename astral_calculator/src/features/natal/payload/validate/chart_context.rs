//! Module astral_calculator\src\features\natal\payload\validate\chart_context.rs du moteur astral_calculator.

use crate::domain::BasicPayload;

pub(super) fn has_current_chart_context(payload: &BasicPayload) -> bool {
    crate::features::natal::payload::rules::chart_context::has_current_chart_context(payload)
}

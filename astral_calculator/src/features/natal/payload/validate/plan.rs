//! Module astral_calculator\src\features\natal\payload\validate\plan.rs du moteur astral_calculator.

use crate::domain::BasicPayload;

pub(super) fn has_current_reading_plan(payload: &BasicPayload) -> bool {
    crate::features::natal::payload::rules::reading_plan::has_current_reading_plan(payload)
}

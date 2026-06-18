//! Module astral_calculator\src\features\natal\payload\validate\lunar_phase.rs du moteur astral_calculator.

use crate::domain::BasicPayload;

pub(super) fn has_current_lunar_phase_context(payload: &BasicPayload) -> bool {
    crate::features::natal::payload::rules::lunar_phase::has_current_lunar_phase_context(payload)
}

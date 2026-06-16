use crate::domain::BasicPayload;

pub(super) fn has_current_lunar_phase_context(payload: &BasicPayload) -> bool {
    crate::payload_rules::lunar_phase::has_current_lunar_phase_context(payload)
}

use crate::domain::BasicPayload;

pub(super) fn has_current_reading_plan(payload: &BasicPayload) -> bool {
    crate::features::payload_rules::reading_plan::has_current_reading_plan(payload)
}

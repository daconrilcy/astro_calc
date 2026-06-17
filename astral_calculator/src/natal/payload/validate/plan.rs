use crate::domain::BasicPayload;

pub(super) fn has_current_reading_plan(payload: &BasicPayload) -> bool {
    crate::natal::payload::rules::reading_plan::has_current_reading_plan(payload)
}

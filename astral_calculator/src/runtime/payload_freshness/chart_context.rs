use crate::domain::BasicPayload;

pub(super) fn has_current_chart_context(payload: &BasicPayload) -> bool {
    crate::features::payload_rules::chart_context::has_current_chart_context(payload)
}

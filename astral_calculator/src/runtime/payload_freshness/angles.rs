use std::collections::HashSet;

use crate::domain::{BasicPayload, BasicSignal};

pub(super) fn has_current_angles(payload: &BasicPayload) -> bool {
    crate::payload_rules::angles::has_current_angles(payload)
}

pub(super) fn has_current_angle_evidence(payload: &BasicPayload, signal: &BasicSignal) -> bool {
    crate::payload_rules::angles::has_current_angle_evidence(payload, signal)
}

pub(super) fn structural_axis_pairs_from_payload(
    payload: &BasicPayload,
) -> HashSet<(String, String)> {
    crate::payload_rules::angles::structural_axis_pairs_from_payload(payload)
}

pub(super) fn angle_object_codes_from_payload(payload: &BasicPayload) -> HashSet<String> {
    crate::payload_rules::angles::angle_object_codes_from_payload(payload)
}

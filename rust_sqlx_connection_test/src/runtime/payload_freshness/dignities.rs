use crate::domain::{BasicDignity, BasicPayload, BasicSignal};

pub(super) fn has_current_dignities(payload: &BasicPayload) -> bool {
    let all_dignities_are_valid = payload.dignities.iter().all(|dignity| {
        !dignity.object_code.trim().is_empty()
            && !dignity.object_name.trim().is_empty()
            && !dignity.sign_code.trim().is_empty()
            && !dignity.sign_name.trim().is_empty()
            && !dignity.dignity_type.trim().is_empty()
            && !dignity.dignity_label.trim().is_empty()
            && matches!(dignity.polarity.as_str(), "dignity" | "debility")
            && dignity.strength_score > 0.0
            && dignity.signal_key.as_deref().is_none_or(|signal_key| {
                payload.signals.iter().any(|signal| {
                    signal.signal_key == signal_key
                        && signal_matches_structured_dignity(signal, dignity)
                })
            })
    });

    all_dignities_are_valid
        && payload
            .signals
            .iter()
            .filter(|signal| signal.signal_key.starts_with("dignity:"))
            .all(|signal| {
                payload.dignities.iter().any(|dignity| {
                    dignity.signal_key.as_deref() == Some(&signal.signal_key)
                        && signal_matches_structured_dignity(signal, dignity)
                }) && signal
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.get("fact_type"))
                    .and_then(|value| value.as_str())
                    == Some("essential_dignity")
            })
}

fn signal_matches_structured_dignity(signal: &BasicSignal, dignity: &BasicDignity) -> bool {
    let Some(evidence) = signal.evidence.as_ref() else {
        return false;
    };

    evidence
        .get("chart_object")
        .and_then(|value| value.as_str())
        == Some(dignity.object_code.as_str())
        && evidence.get("sign_code").and_then(|value| value.as_str())
            == Some(dignity.sign_code.as_str())
        && evidence
            .get("dignity_type")
            .and_then(|value| value.as_str())
            == Some(dignity.dignity_type.as_str())
}

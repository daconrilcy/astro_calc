use crate::dignities::{
    dignity_is_signal_worthy, essential_dignities_for_position, essential_dignities_for_positions,
    EssentialDignityFact,
};
use crate::domain::{BasicDignity, BasicSignal, ObjectPositionFact};
pub(super) fn position_dignity_context(position: &ObjectPositionFact) -> serde_json::Value {
    let dignities = essential_dignities_for_position(position);
    serde_json::Value::Array(
        dignities
            .into_iter()
            .map(|dignity| {
                serde_json::json!({
                    "fact_type": "essential_dignity",
                    "dignity_type": dignity.dignity_type,
                    "dignity_label": dignity.dignity_label,
                    "polarity": dignity.polarity,
                    "strength_score": dignity.strength_score,
                })
            })
            .collect(),
    )
}

pub(super) fn build_payload_dignities(
    positions: &[ObjectPositionFact],
    signals: &[BasicSignal],
) -> Vec<BasicDignity> {
    essential_dignities_for_positions(positions)
        .into_iter()
        .map(|dignity| {
            let signal_key = dignity_signal_key(&dignity);
            let signal_key = signals
                .iter()
                .any(|signal| signal.signal_key == signal_key)
                .then_some(signal_key);

            BasicDignity {
                object_code: dignity.object_code,
                object_name: dignity.object_name,
                sign_id: dignity.sign_id,
                sign_code: dignity.sign_code,
                sign_name: dignity.sign_name,
                dignity_type: dignity.dignity_type,
                dignity_label: dignity.dignity_label,
                polarity: dignity.polarity,
                strength_score: dignity.strength_score,
                signal_key,
            }
        })
        .collect()
}

fn dignity_signal_key(dignity: &EssentialDignityFact) -> String {
    if dignity_is_signal_worthy(dignity) {
        format!(
            "dignity:{}:{}:{}",
            dignity.object_code, dignity.dignity_type, dignity.sign_code
        )
    } else {
        String::new()
    }
}

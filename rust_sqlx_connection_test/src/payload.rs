use crate::domain::{
    BasicObjectPosition, BasicPayload, BasicSignal, InterpretationSignalRow, NatalChartInput,
    ObjectPositionFact,
};

pub fn build_basic_payload(
    chart_calculation_id: i32,
    input: &NatalChartInput,
    positions: &[ObjectPositionFact],
    signals: &[InterpretationSignalRow],
) -> BasicPayload {
    BasicPayload {
        product_code: input.product_code().to_string(),
        chart_calculation_id,
        reference_version_id: input.reference_version_id,
        subject_label: input.subject_label.clone(),
        birth_datetime_utc: input.birth_datetime_utc,
        positions: positions
            .iter()
            .map(|position| BasicObjectPosition {
                object_code: position.object_code.clone(),
                object_name: position.object_name.clone(),
                longitude_deg: position.longitude_deg,
                sign_id: position.sign_id,
                house_id: position.house_id,
                motion_state_id: position.motion_state_id,
            })
            .collect(),
        signals: signals
            .iter()
            .map(|signal| BasicSignal {
                signal_key: signal.signal_key.clone(),
                title: signal.title.clone(),
                summary: signal.summary.clone(),
                priority_score: signal.priority_score,
                confidence_score: signal.confidence_score,
            })
            .collect(),
    }
}

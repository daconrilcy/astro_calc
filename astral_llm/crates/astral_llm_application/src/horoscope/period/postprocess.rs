use super::*;

pub(crate) fn reprocess_horoscope_daily_payload(response: Value) -> Value {
    reprocess_horoscope_daily("fr", response, None).payload
}

#[doc(hidden)]
pub fn reprocess_horoscope_period_payload(response: Value) -> Value {
    reprocess_horoscope_period("fr", response, None).payload
}

#[doc(hidden)]
pub fn postprocess_period_provider_response(request: &Value, response: Value) -> Value {
    let mut response = response;
    repair_period_response_shape(request, &mut response);
    prune_period_v2_overlapping_watch_windows(&mut response);
    response
}

pub(crate) fn prune_period_v2_overlapping_watch_windows(response: &mut Value) {
    let best_identities = response["best_windows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(period_window_identity)
        .collect::<HashSet<_>>();
    if best_identities.is_empty() {
        return;
    }
    if let Some(watch_windows) = response
        .get_mut("watch_windows")
        .and_then(Value::as_array_mut)
    {
        watch_windows.retain(|window| {
            period_window_identity(window)
                .map(|identity| !best_identities.contains(&identity))
                .unwrap_or(true)
        });
    }
}

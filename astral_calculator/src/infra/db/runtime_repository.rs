//! Helpers runtime DB conservés pour compatibilité.

use serde_json::Value;

use crate::domain::BasicPayload;
use crate::shared::error::RuntimeError;

/// Fonction parse_existing_basic_payload_value.
pub fn parse_existing_basic_payload_value(
    value: Value,
) -> Result<Option<BasicPayload>, RuntimeError> {
    match serde_json::from_value(value) {
        Ok(payload) => Ok(Some(payload)),
        Err(error) if is_stale_basic_payload_shape(&error) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Fonction is_stale_basic_payload_shape.
fn is_stale_basic_payload_shape(error: &serde_json::Error) -> bool {
    error.is_data() && error.to_string().contains("missing field")
}

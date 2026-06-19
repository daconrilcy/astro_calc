#[cfg(feature = "swisseph-engine")]
use std::sync::{Mutex, OnceLock};

use crate::shared::error::RuntimeError;

#[cfg(feature = "swisseph-engine")]
pub(crate) fn with_swiss_ephemeris_lock<T>(
    f: impl FnOnce() -> Result<T, RuntimeError>,
) -> Result<T, RuntimeError> {
    let _guard = swiss_ephemeris_lock()
        .lock()
        .map_err(|_| RuntimeError::Ephemeris("Swiss Ephemeris lock poisoned".to_string()))?;
    f()
}

#[cfg(not(feature = "swisseph-engine"))]
#[allow(dead_code)]
pub(crate) fn with_swiss_ephemeris_lock<T>(
    f: impl FnOnce() -> Result<T, RuntimeError>,
) -> Result<T, RuntimeError> {
    f()
}

#[cfg(feature = "swisseph-engine")]
fn swiss_ephemeris_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

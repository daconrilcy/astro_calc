//! Module astral_calculator\src\features\mod.rs du moteur astral_calculator.

pub mod horoscope;
/// Module natal.
pub mod natal;
/// Module simplified.
pub mod simplified;

pub use crate::engine::projection as llm_projection;

/// Module payload.
pub mod payload {
    pub use crate::features::natal::payload::build::*;
}

/// Module signals.
pub mod signals {
    pub use crate::features::natal::signals::*;
}

//! Module astral_calculator/src/lib.rs du moteur astral_calculator.

pub mod astrology;
/// Module bootstrap.
pub mod bootstrap;
/// Module domain.
pub mod domain;
/// Module engine.
pub mod engine;
/// Module features.
pub mod features;
/// Module infra.
pub mod infra;
/// Module runtime.
pub mod runtime;
/// Module shared.
pub mod shared;

pub use engine::engine_request_from_env;

/// Module aspects.
pub mod aspects {
    pub use crate::astrology::aspects::*;
}

/// Module catalog.
pub mod catalog {
    pub use crate::features::natal::catalog::*;
}

/// Module cli.
pub mod cli {
    pub use crate::bootstrap::cli::*;
}

/// Module config.
pub mod config {
    pub use crate::bootstrap::env::*;
}

/// Module db.
pub mod db {
    pub use crate::bootstrap::db::*;
}

/// Module dignities.
pub mod dignities {
    pub use crate::features::natal::dignities::*;
}

/// Module ephemeris.
pub mod ephemeris {
    pub use crate::astrology::ephemeris::*;
}

/// Module facts.
pub mod facts {
    pub use crate::shared::astro_math::*;
}

/// Module idempotency.
pub mod idempotency {
    pub use crate::shared::idempotency::*;
}

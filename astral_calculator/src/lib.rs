pub mod astrology;
pub mod bootstrap;
pub mod domain;
pub mod engine;
pub mod horoscope;
pub mod infra;
pub mod natal;
pub mod runtime;
pub mod shared;
pub mod simplified;

pub use engine::engine_request_from_env;

pub mod aspects {
    pub use crate::astrology::aspects::*;
}

pub mod catalog {
    pub use crate::natal::catalog::*;
}

pub mod cli {
    pub use crate::bootstrap::cli::*;
}

pub mod config {
    pub use crate::bootstrap::env::*;
}

pub mod db {
    pub use crate::bootstrap::db::*;
}

pub mod dignities {
    pub use crate::natal::dignities::*;
}

pub mod ephemeris {
    pub use crate::astrology::ephemeris::*;
}

pub mod facts {
    pub use crate::shared::astro_math::*;
}

pub mod features {
    pub use crate::engine::projection as llm_projection;
    pub use crate::horoscope;
    pub use crate::simplified;

    pub mod payload {
        pub use crate::natal::payload::build::*;
    }

    pub mod signals {
        pub use crate::natal::signals::*;
    }
}

pub mod idempotency {
    pub use crate::shared::idempotency::*;
}

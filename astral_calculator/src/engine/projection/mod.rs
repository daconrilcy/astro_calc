//! Module astral_calculator\src\engine\projection\mod.rs du moteur astral_calculator.

mod axis_labels;
/// Module builder.
mod builder;
/// Module clean_text.
mod clean_text;
/// Module dynamics.
mod dynamics;
/// Module humanize.
mod humanize;
/// Module profiles.
mod profiles;
/// Module types.
mod types;

pub use profiles::{
    default_max_accidental_conditions, default_max_background_placements,
    resolve_projection_profile,
};

pub use builder::{build_llm_projection_natal_v1, LlmProjectionBuildContext};
pub use dynamics::is_active_major_aspect_signal;
pub use profiles::{limits_envelope, profile_from_level};
pub use types::*;

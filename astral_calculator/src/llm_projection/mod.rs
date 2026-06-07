mod axis_labels;
mod builder;
mod clean_text;
mod dynamics;
mod humanize;
mod profiles;
mod types;

pub use profiles::{
    default_max_accidental_conditions, default_max_background_placements,
    resolve_projection_profile,
};

pub use builder::{build_llm_projection_natal_v1, LlmProjectionBuildContext};
pub use dynamics::is_active_major_aspect_signal;
pub use profiles::{all_profiles_from_seed, limits_envelope, profile_from_level};
pub use types::*;

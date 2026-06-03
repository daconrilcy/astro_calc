mod axis_labels;
mod builder;
mod humanize;
mod profiles;
mod types;

pub use profiles::resolve_projection_profile;

pub use builder::{build_llm_projection_natal_v1, LlmProjectionBuildContext};
pub use profiles::{all_profiles_from_seed, limits_envelope, profile_from_level};
pub use types::*;

pub mod application;
pub(crate) mod catalog;
mod ephemeris_calc;
mod facts;
mod payload;
mod repository;
mod request;
mod resolve;
mod response;
mod service;
mod uncertainty_window;

pub use catalog::SimplifiedCatalog;
pub use request::{
    AstroSimplifiedNatalRequest, SimplifiedLocationRequest, SIMPLIFIED_REQUEST_CONTRACT_VERSION,
};
pub use response::{
    AstroSimplifiedNatalResponse, LlmPayloadControls, SIMPLIFIED_RESPONSE_CONTRACT_VERSION,
};

#[cfg(any(test, feature = "test-utils"))]
pub use catalog::{
    CalculationScope, InputPrecisionLevel, LimitationCode, ProfileFeatureExclusion,
    ReliabilityLevel, SimplifiedPolicy,
};
#[cfg(any(test, feature = "test-utils"))]
pub use ephemeris_calc::dedupe_preserve_order;
#[cfg(any(test, feature = "test-utils"))]
pub use facts::{
    collect_window_sign_facts, CollectedSignFacts, RELIABILITY_AMBIGUOUS, RELIABILITY_STABLE,
};
#[cfg(any(test, feature = "test-utils"))]
pub use payload::build_response;
#[cfg(any(test, feature = "test-utils"))]
pub use resolve::{build_uncertainty_window, validate_and_resolve};
#[cfg(any(test, feature = "test-utils"))]
pub use response::{AmbiguousSignFactResponse, SignFactResponse};
pub use service::calculate_simplified_natal;
#[cfg(any(test, feature = "test-utils"))]
pub use uncertainty_window::sample_points_utc;

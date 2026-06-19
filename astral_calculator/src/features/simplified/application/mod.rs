//! Module astral_calculator\src\features\simplified\application\mod.rs du moteur astral_calculator.

use async_trait::async_trait;

use crate::features::simplified::{AstroSimplifiedNatalRequest, AstroSimplifiedNatalResponse};
use crate::shared::error::RuntimeError;

pub mod simplified_natal_service;

pub use simplified_natal_service::SimplifiedNatalService;

#[async_trait]
pub trait SimplifiedNatalCapability: Send + Sync {
    async fn calculate_simplified(
        &self,
        request: AstroSimplifiedNatalRequest,
        ephemeris_path: &std::path::Path,
    ) -> Result<AstroSimplifiedNatalResponse, RuntimeError>;
}

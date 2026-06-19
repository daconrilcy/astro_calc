//! Module astral_calculator\src\features\natal\application\mod.rs du moteur astral_calculator.

use async_trait::async_trait;

use crate::domain::{BasicPayload, BasicPayloadCatalog, NatalChartInput, RuntimeOptions};
use crate::shared::error::RuntimeError;

pub mod natal_calculation_service;

pub use natal_calculation_service::NatalCalculationService;

#[async_trait]
pub trait NatalCalculationCapability: Send + Sync {
    fn options(&self) -> &RuntimeOptions;
    async fn calculate_basic(&self, input: NatalChartInput) -> Result<BasicPayload, RuntimeError>;
    async fn calculate_basic_with_catalog(
        &self,
        input: NatalChartInput,
    ) -> Result<(BasicPayload, BasicPayloadCatalog), RuntimeError>;
}

use std::sync::Arc;

use crate::{
    natal::GenerateNatalReadingUseCase,
    ports::{CalculatorPort, LlmPort},
};

#[derive(Clone)]
pub struct AppState {
    pub calculator: Arc<dyn CalculatorPort>,
    pub llm: Arc<dyn LlmPort>,
}

impl AppState {
    pub fn natal_use_case(&self) -> GenerateNatalReadingUseCase {
        GenerateNatalReadingUseCase::new(self.calculator.clone(), self.llm.clone())
    }
}

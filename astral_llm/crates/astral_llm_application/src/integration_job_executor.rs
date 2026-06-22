use astral_contracts::horoscope_service_descriptor;
use astral_llm_domain::{
    generation_request::AudienceLevel,
    integration::{CalculationMode, IntegrationService},
    GenerateReadingResponse, GenerationError, GenerationErrorCode,
};
use serde_json::Value;
use uuid::Uuid;

use crate::core::calculator::CalculatorPort;
use crate::generate_reading_use_case::{GenerateReadingUseCase, UseCaseOutput};
use crate::horoscope::{HoroscopeDailyNatalOrchestrator, HoroscopePeriodNatalOrchestrator};
use crate::integration_job_validator::ValidatedIntegrationJob;
use crate::simplified_reading::build_reading_request;

#[derive(Debug, Clone)]
pub struct UnifiedReadingResult {
    pub run_id: String,
    pub outcome: UnifiedReadingOutcome,
}

#[derive(Debug, Clone)]
pub enum UnifiedReadingOutcome {
    Reading {
        calculation: Option<Value>,
        reading: GenerateReadingResponse,
        reading_completeness: Option<String>,
    },
    Json(Value),
}

pub struct IntegrationJobExecutor<'a, C>
where
    C: CalculatorPort + ?Sized,
{
    calculator: &'a C,
    use_case: &'a GenerateReadingUseCase,
}

impl<'a, C> IntegrationJobExecutor<'a, C>
where
    C: CalculatorPort + ?Sized,
{
    pub fn new(calculator: &'a C, use_case: &'a GenerateReadingUseCase) -> Self {
        Self {
            calculator,
            use_case,
        }
    }

    pub async fn execute(
        &self,
        service: &IntegrationService,
        job: &ValidatedIntegrationJob,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        if !supports_integration_service(service) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!(
                    "orchestration not implemented for service: {}",
                    service.service_code
                ),
                Value::Null,
            ));
        }
        if let Some(descriptor) = horoscope_service_descriptor(&service.service_code) {
            return self
                .run_horoscope_service(
                    job,
                    descriptor.contracts.public_request_contract,
                    public_run_id,
                )
                .await;
        }

        if service.is_from_payload() {
            return self.run_from_payload(job).await;
        }

        match service.calculation_mode {
            CalculationMode::SimplifiedNatal => self.run_simplified(job).await,
            CalculationMode::FullNatal => self.run_full_natal(job).await,
            CalculationMode::None => Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!(
                    "orchestration not implemented for service: {}",
                    service.service_code
                ),
                Value::Null,
            )),
        }
    }

    async fn run_horoscope_service(
        &self,
        job: &ValidatedIntegrationJob,
        public_request_contract: &str,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let run_id = public_run_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let result = match public_request_contract {
            "horoscope_daily_request_v2" => {
                HoroscopeDailyNatalOrchestrator::execute(
                    &job.service_code,
                    self.calculator,
                    self.use_case,
                    &job.payload,
                    Some(&run_id),
                )
                .await?
            }
            "horoscope_period_request_v2" => {
                HoroscopePeriodNatalOrchestrator::execute(
                    &job.service_code,
                    self.calculator,
                    self.use_case,
                    &job.payload,
                    Some(&run_id),
                )
                .await?
            }
            contract => {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::InvalidInput,
                    format!("unsupported horoscope contract: {contract}"),
                    Value::Null,
                ));
            }
        };
        Ok(UnifiedReadingResult {
            run_id,
            outcome: UnifiedReadingOutcome::Json(result),
        })
    }

    async fn run_simplified(
        &self,
        job: &ValidatedIntegrationJob,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        crate::validate_simplified_calculation_request(&job.payload)?;
        let calculation = self
            .calculator
            .calculate_simplified_natal(&job.payload)
            .await?;

        let audience = parse_audience_level(&job.audience_level);
        let mut reading_request =
            build_reading_request(&calculation, &job.user_language, audience)?;
        self.use_case.prepare_request(&mut reading_request)?;

        let run_id = Uuid::new_v4().to_string();
        let output = self
            .use_case
            .execute_with_audit(reading_request, run_id.clone())
            .await;

        let reading_completeness = calculation
            .pointer("/reading_hint/reading_completeness")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        Ok(build_unified_result(
            run_id,
            Some(calculation),
            output,
            reading_completeness,
        ))
    }

    async fn run_from_payload(
        &self,
        job: &ValidatedIntegrationJob,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let mut reading_request: astral_llm_domain::GenerateReadingRequest =
            serde_json::from_value(job.payload.clone()).map_err(|err| {
                GenerationError::with_details(
                    GenerationErrorCode::InvalidInput,
                    format!("invalid generate_reading_request payload: {err}"),
                    Value::Null,
                )
            })?;
        self.use_case.prepare_request(&mut reading_request)?;

        let run_id = Uuid::new_v4().to_string();
        let output = self
            .use_case
            .execute_with_audit(reading_request, run_id.clone())
            .await;

        Ok(build_unified_result(run_id, None, output, None))
    }

    async fn run_full_natal(
        &self,
        job: &ValidatedIntegrationJob,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let calculation = self.calculator.calculate_natal(&job.payload).await?;
        crate::engine_reading::validate_engine_response(&calculation)?;

        let audience = parse_audience_level(&job.audience_level);
        let mut reading_request = crate::engine_reading::build_reading_request_from_engine(
            &calculation,
            &job.profile_code,
            &job.user_language,
            audience,
            None,
            None,
        )?;
        self.use_case.prepare_request(&mut reading_request)?;

        let run_id = Uuid::new_v4().to_string();
        let output = self
            .use_case
            .execute_with_audit(reading_request, run_id.clone())
            .await;

        Ok(build_unified_result(
            run_id,
            Some(calculation),
            output,
            Some("full".into()),
        ))
    }
}

pub fn supports_integration_service(service: &IntegrationService) -> bool {
    if horoscope_service_descriptor(&service.service_code).is_some() {
        return true;
    }
    if service.is_from_payload() {
        return service.payload_contract == "generate_reading_request_v1"
            && matches_orchestration(service, &["interpretation_only", "llm_only"]);
    }
    match service.calculation_mode {
        CalculationMode::SimplifiedNatal => {
            matches_orchestration(
                service,
                &[
                    "unified_from_birth",
                    "calculator_then_llm",
                    "legacy_unified",
                ],
            ) && service.payload_contract == "astro_simplified_natal_request_v1"
        }
        CalculationMode::FullNatal => {
            matches_orchestration(
                service,
                &[
                    "unified_from_birth",
                    "calculator_then_llm",
                    "legacy_unified",
                ],
            ) && service.payload_contract == "astro_engine_request_v1"
        }
        CalculationMode::None => false,
    }
}

fn matches_orchestration(service: &IntegrationService, accepted_raw_modes: &[&str]) -> bool {
    accepted_raw_modes.contains(&service.orchestration_mode.as_str())
}

fn build_unified_result(
    run_id: String,
    calculation: Option<Value>,
    output: UseCaseOutput,
    reading_completeness: Option<String>,
) -> UnifiedReadingResult {
    UnifiedReadingResult {
        run_id,
        outcome: UnifiedReadingOutcome::Reading {
            calculation,
            reading: output.response,
            reading_completeness,
        },
    }
}

fn parse_audience_level(raw: &str) -> AudienceLevel {
    match raw {
        "intermediate" => AudienceLevel::Intermediate,
        "expert" => AudienceLevel::Expert,
        _ => AudienceLevel::Beginner,
    }
}

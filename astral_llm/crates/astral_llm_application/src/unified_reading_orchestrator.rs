use astral_llm_domain::{
    generation_request::AudienceLevel, GenerateReadingResponse, GenerationError,
    GenerationErrorCode,
};
use astral_llm_infra::CalculatorClient;
use serde_json::Value;
use uuid::Uuid;

use crate::generate_reading_use_case::{GenerateReadingUseCase, UseCaseOutput};
use crate::horoscope::{
    HoroscopeBasicDailyNatalOrchestrator, HoroscopeFreeDailyOrchestrator,
    HoroscopePeriodNatalOrchestrator, HoroscopePremiumDailyLocalOrchestrator,
    HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_FREE_DAILY_SERVICE_CODE,
    HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE, HOROSCOPE_SERVICE_CODE,
};
use crate::integration_job_validator::ValidatedIntegrationJob;
use crate::simplified_reading::{
    build_reading_request, validate_simplified_calculation_request, SIMPLIFIED_PROFILE,
};

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

pub struct UnifiedReadingOrchestrator<'a> {
    calculator: &'a CalculatorClient,
    use_case: &'a GenerateReadingUseCase,
}

impl<'a> UnifiedReadingOrchestrator<'a> {
    pub fn new(calculator: &'a CalculatorClient, use_case: &'a GenerateReadingUseCase) -> Self {
        Self {
            calculator,
            use_case,
        }
    }

    pub async fn execute(
        &self,
        job: &ValidatedIntegrationJob,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        match job.service_code.as_str() {
            HOROSCOPE_SERVICE_CODE => self.run_horoscope(job, public_run_id).await,
            HOROSCOPE_FREE_DAILY_SERVICE_CODE => self.run_free_horoscope(job, public_run_id).await,
            HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE => {
                self.run_premium_horoscope(job, public_run_id).await
            }
            HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE => {
                self.run_period_horoscope(job, public_run_id).await
            }
            SIMPLIFIED_PROFILE => self.run_simplified(job).await,
            other if other.ends_with("_from_payload") => self.run_from_payload(job).await,
            other if other.starts_with("natal_") => self.run_full_natal(job).await,
            code => Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!("orchestration not implemented for service: {code}"),
                Value::Null,
            )),
        }
    }

    async fn run_horoscope(
        &self,
        job: &ValidatedIntegrationJob,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let run_id = public_run_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let result = HoroscopeBasicDailyNatalOrchestrator::execute(
            self.calculator,
            self.use_case,
            &job.payload,
            Some(&run_id),
        )
        .await?;
        Ok(UnifiedReadingResult {
            run_id,
            outcome: UnifiedReadingOutcome::Json(result),
        })
    }

    async fn run_free_horoscope(
        &self,
        job: &ValidatedIntegrationJob,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let run_id = public_run_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let result = HoroscopeFreeDailyOrchestrator::execute(
            self.calculator,
            self.use_case,
            &job.payload,
            Some(&run_id),
        )
        .await?;
        Ok(UnifiedReadingResult {
            run_id,
            outcome: UnifiedReadingOutcome::Json(result),
        })
    }

    async fn run_premium_horoscope(
        &self,
        job: &ValidatedIntegrationJob,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let run_id = public_run_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let result = HoroscopePremiumDailyLocalOrchestrator::execute(
            self.calculator,
            self.use_case,
            &job.payload,
            Some(&run_id),
        )
        .await?;
        Ok(UnifiedReadingResult {
            run_id,
            outcome: UnifiedReadingOutcome::Json(result),
        })
    }

    async fn run_period_horoscope(
        &self,
        job: &ValidatedIntegrationJob,
        public_run_id: Option<&str>,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        let run_id = public_run_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let result = HoroscopePeriodNatalOrchestrator::execute(
            &job.service_code,
            self.calculator,
            self.use_case,
            &job.payload,
            Some(&run_id),
        )
        .await?;
        Ok(UnifiedReadingResult {
            run_id,
            outcome: UnifiedReadingOutcome::Json(result),
        })
    }

    async fn run_simplified(
        &self,
        job: &ValidatedIntegrationJob,
    ) -> Result<UnifiedReadingResult, GenerationError> {
        validate_simplified_calculation_request(&job.payload)?;
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

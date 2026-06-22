use super::*;
use crate::core::calculator::CalculatorPort;
pub struct HoroscopeBasicDailyNatalOrchestrator;
pub struct HoroscopeFreeDailyOrchestrator;
pub struct HoroscopePremiumDailyLocalOrchestrator;
pub struct HoroscopeDailyNatalOrchestrator;
pub struct HoroscopePeriodNatalOrchestrator;
impl HoroscopeBasicDailyNatalOrchestrator {
    pub async fn execute<C: CalculatorPort + ?Sized>(
        calculator: &C,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
        run_id: Option<&str>,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
            calculator,
            use_case,
            payload,
            run_id,
        )
        .await
    }
}
impl HoroscopeFreeDailyOrchestrator {
    pub async fn execute<C: CalculatorPort + ?Sized>(
        calculator: &C,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
        run_id: Option<&str>,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_FREE_DAILY_SERVICE_CODE,
            calculator,
            use_case,
            payload,
            run_id,
        )
        .await
    }
}
impl HoroscopePremiumDailyLocalOrchestrator {
    pub async fn execute<C: CalculatorPort + ?Sized>(
        calculator: &C,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
        run_id: Option<&str>,
    ) -> Result<serde_json::Value, GenerationError> {
        HoroscopeDailyNatalOrchestrator::execute(
            HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
            calculator,
            use_case,
            payload,
            run_id,
        )
        .await
    }
}
impl HoroscopePeriodNatalOrchestrator {
    pub async fn execute<C: CalculatorPort + ?Sized>(
        service_code: &str,
        calculator: &C,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
        run_id: Option<&str>,
    ) -> Result<serde_json::Value, GenerationError> {
        validate_period_service_code(service_code)?;
        let public = validate_period_public_request(payload)?;
        let calculation_request =
            build_period_calculation_request_for_service(service_code, &public)?;
        let calculation = calculator
            .calculate_horoscope_period_natal(&calculation_request)
            .await
            .map_err(|err| {
                GenerationError::with_details(
                    GenerationErrorCode::ProviderUnavailable,
                    format!(
                        "HOROSCOPE_PERIOD_CALCULATION_FAILED: {}",
                        err.detail().message
                    ),
                    Value::Null,
                )
            })?;
        let writer_request = build_period_writer_request(&public, &calculation)?;
        let response =
            period_writer_response_with_quality_loop(use_case, &writer_request, run_id).await?;
        validate_period_response_contract(&writer_request, &response)?;
        let mut result = json!({
            "calculation": calculation,
            "writer_request": writer_request,
            "reading": response
        });
        result["debug"]["period_editorial_audit"] =
            period_editorial_audit(&result["writer_request"], &result["reading"]);
        if let Some(warning) = public.language_compat_warning.clone() {
            result["debug"]["language_compatibility"] = warning;
        }
        Ok(result)
    }
}
impl HoroscopeDailyNatalOrchestrator {
    pub async fn execute<C: CalculatorPort + ?Sized>(
        service_code: &str,
        calculator: &C,
        use_case: &GenerateReadingUseCase,
        payload: &Value,
        run_id: Option<&str>,
    ) -> Result<serde_json::Value, GenerationError> {
        let public = validate_public_request(payload)?;
        let calculation_request = build_calculation_request_for_service(service_code, &public)?;
        let calculation = calculator
            .calculate_horoscope_daily_natal(&calculation_request)
            .await
            .map_err(|err| {
                GenerationError::with_details(
                    GenerationErrorCode::ProviderUnavailable,
                    format!("HOROSCOPE_CALCULATOR_UNAVAILABLE: {}", err.detail().message),
                    Value::Null,
                )
            })?;
        let signals = score_calculation(&calculation)?;
        let interpretation = build_interpretation_request(&public, &calculation, &signals)?;
        let response = daily_writer_response(use_case, &interpretation, run_id).await?;
        validate_horoscope_response_schema(&response)?;
        validate_response_evidence(&interpretation, &response)?;
        Ok(
            json!({            "calculation": calculation,            "interpretation_request": interpretation,            "reading": response        }),
        )
    }
}

use super::*;
pub struct HoroscopeBasicDailyNatalOrchestrator;
pub struct HoroscopeFreeDailyOrchestrator;
pub struct HoroscopePremiumDailyLocalOrchestrator;
pub struct HoroscopeDailyNatalOrchestrator;
pub struct HoroscopePeriodNatalOrchestrator;
impl HoroscopeBasicDailyNatalOrchestrator {
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
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
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
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
    pub async fn execute(
        calculator: &astral_llm_infra::CalculatorClient,
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
    pub async fn execute(
        service_code: &str,
        calculator: &astral_llm_infra::CalculatorClient,
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
        let generation_mode = period_generation_mode(service_code)?;
        let interpretation = match generation_mode {
            PeriodGenerationMode::LegacyV1 => {
                build_period_interpretation_request(&public, &calculation)?
            }
            PeriodGenerationMode::SemanticBriefV2 => {
                build_period_writer_request_v2(&public, &calculation)?
            }
        };
        let mut response = match generation_mode {
            PeriodGenerationMode::LegacyV1 => {
                period_writer_response(use_case, &interpretation, run_id).await?
            }
            PeriodGenerationMode::SemanticBriefV2 => {
                period_writer_response_with_quality_loop(use_case, &interpretation, run_id).await?
            }
        };
        if generation_mode == PeriodGenerationMode::LegacyV1 {
            if is_free_period_request(&interpretation) {
                prune_period_response_variant_fields(&interpretation, &mut response);
            } else {
                enforce_period_public_personalization_from_request(&interpretation, &mut response);
                enforce_premium_period_advice_synthesis(&interpretation, &mut response);
                restore_period_response_evidence_from_request(&interpretation, &mut response);
                normalize_period_public_strings(&mut response);
                enforce_period_public_personalization_from_request(&interpretation, &mut response);
            }
        }
        match generation_mode {
            PeriodGenerationMode::LegacyV1 => {
                if is_free_period_request(&interpretation) {
                    prune_period_response_variant_fields(&interpretation, &mut response);
                }
                validate_period_response_schema(&response)?;
                validate_period_response_evidence(&interpretation, &response)?;
            }
            PeriodGenerationMode::SemanticBriefV2 => {
                validate_period_response_contract_gates_v2(&interpretation, &response)?;
            }
        }
        let mut result = json!({            "calculation": calculation,            "interpretation_request": interpretation,            "reading": response        });
        if generation_mode == PeriodGenerationMode::SemanticBriefV2 {
            result["writer_request"] = result["interpretation_request"].clone();
            result["debug"]["period_v2_editorial_audit"] =
                period_v2_editorial_audit(&result["interpretation_request"], &result["reading"]);
            if let Some(warning) = public.language_compat_warning.clone() {
                result["debug"]["language_compatibility"] = warning;
            }
        }
        Ok(result)
    }
}
impl HoroscopeDailyNatalOrchestrator {
    pub async fn execute(
        service_code: &str,
        calculator: &astral_llm_infra::CalculatorClient,
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

use astral_llm_domain::{
    interpretation_profile::{InterpretationProfile, NATAL_PROMPTER_PRODUCT},
    GenerateReadingRequest, GenerationError, GenerationErrorCode, ProductGenerationPolicy,
};
use astral_llm_infra::SharedCanonicalCatalog;

/// Contexte resolu pour une generation `natal_prompter`.
#[derive(Debug, Clone)]
pub struct ResolvedInterpretationContext {
    pub profile: InterpretationProfile,
    pub effective_policy: ProductGenerationPolicy,
}

/// Resultat de validation produit + profil optionnel.
#[derive(Debug, Clone)]
pub struct ValidatedProductContext {
    pub policy: ProductGenerationPolicy,
    pub interpretation: Option<ResolvedInterpretationContext>,
}

pub struct InterpretationProfileResolver;

impl InterpretationProfileResolver {
    /// Normalise les anciens `product_code` et aligne `generation_mode` sur le profil.
    pub fn normalize_request(
        request: &mut GenerateReadingRequest,
        catalog: &SharedCanonicalCatalog,
    ) -> Result<(), GenerationError> {
        Self::migrate_legacy_product_codes(request);

        if request.product_context.product_code != NATAL_PROMPTER_PRODUCT {
            return Ok(());
        }

        let profile_code = Self::required_profile_code(request)?;
        let profile = catalog.interpretation_profile(profile_code).ok_or_else(|| {
            profile_not_found(profile_code)
        })?;

        profile.validate().map_err(|msg| {
            GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                format!("invalid interpretation profile: {msg}"),
                serde_json::json!({ "profile_code": profile_code }),
            )
        })?;

        request.response_contract.generation_mode = profile.document.generation_mode;
        Ok(())
    }

    fn migrate_legacy_product_codes(request: &mut GenerateReadingRequest) {
        match request.product_context.product_code.as_str() {
            "natal_premium" => {
                request.product_context.product_code = NATAL_PROMPTER_PRODUCT.into();
                if request
                    .product_context
                    .interpretation_profile_code
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
                {
                    request.product_context.interpretation_profile_code =
                        Some("natal_premium".into());
                }
            }
            "natal_basic" => {
                request.product_context.product_code = NATAL_PROMPTER_PRODUCT.into();
                if request
                    .product_context
                    .interpretation_profile_code
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
                {
                    request.product_context.interpretation_profile_code =
                        Some("natal_basic".into());
                }
            }
            _ => {}
        }
    }

    fn required_profile_code<'a>(
        request: &'a GenerateReadingRequest,
    ) -> Result<&'a str, GenerationError> {
        request
            .product_context
            .interpretation_profile_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                GenerationError::with_details(
                    GenerationErrorCode::InvalidInput,
                    "interpretation_profile_code is required for natal_prompter",
                    serde_json::json!({ "product_code": NATAL_PROMPTER_PRODUCT }),
                )
            })
    }

    pub fn resolve(
        request: &GenerateReadingRequest,
        catalog: &SharedCanonicalCatalog,
    ) -> Result<Option<ResolvedInterpretationContext>, GenerationError> {
        if request.product_context.product_code != NATAL_PROMPTER_PRODUCT {
            return Ok(None);
        }

        let profile_code = Self::required_profile_code(request)?;
        let profile = catalog.interpretation_profile(profile_code).ok_or_else(|| {
            profile_not_found(profile_code)
        })?;

        profile.validate().map_err(|msg| {
            GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                format!("invalid interpretation profile: {msg}"),
                serde_json::json!({ "profile_code": profile_code }),
            )
        })?;

        let mut effective_policy = profile.to_product_generation_policy();
        if let Some(base) = catalog.product_policy(NATAL_PROMPTER_PRODUCT) {
            if effective_policy.allowed_providers.is_empty() {
                effective_policy.allowed_providers = base.allowed_providers.clone();
            }
            if effective_policy.allowed_models.is_empty() {
                effective_policy.allowed_models = base.allowed_models.clone();
            }
        }

        Ok(Some(ResolvedInterpretationContext {
            profile: profile.clone(),
            effective_policy,
        }))
    }

    pub fn validate_product(
        request: &GenerateReadingRequest,
        catalog: &SharedCanonicalCatalog,
        resolved_provider: &astral_llm_domain::ProviderKind,
        resolved_model: &str,
    ) -> Result<ValidatedProductContext, GenerationError> {
        let interpretation = Self::resolve(request, catalog)?;

        let policy = if let Some(ctx) = &interpretation {
            ctx.effective_policy.clone()
        } else {
            catalog
                .product_policy(&request.product_context.product_code)
                .cloned()
                .ok_or_else(|| {
                    GenerationError::with_details(
                        GenerationErrorCode::ProductPolicyViolation,
                        format!(
                            "no generation policy for product: {}",
                            request.product_context.product_code
                        ),
                        serde_json::json!({
                            "product_code": request.product_context.product_code
                        }),
                    )
                })?
        };

        crate::product_policy_validator::ProductPolicyValidator::validate_against_policy(
            request,
            &policy,
            resolved_provider,
            resolved_model,
        )?;

        Ok(ValidatedProductContext {
            policy,
            interpretation,
        })
    }
}

fn profile_not_found(profile_code: &str) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::InvalidInput,
        format!("interpretation profile not found or inactive: {profile_code}"),
        serde_json::json!({
            "profile_code": profile_code,
            "error": "PROFILE_NOT_FOUND"
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
        engine_params::EngineParams,
        generation_request::{AudienceLevel, ProductContext},
        output_contract::{GenerationMode, OutputFormat, ResponseContract},
        AstroCalculationPayload, AstrologerProfile,
    };
    use astral_llm_infra::{
        bootstrap_interpretation_profiles, bootstrap_product_policies, CanonicalCatalog,
    };

    fn catalog_with_profiles() -> SharedCanonicalCatalog {
        std::sync::Arc::new(CanonicalCatalog {
            product_generation_policies: bootstrap_product_policies(),
            interpretation_profiles: bootstrap_interpretation_profiles(),
            ..Default::default()
        })
    }

    fn base_request(product_code: &str, profile: Option<&str>, mode: GenerationMode) -> GenerateReadingRequest {
        GenerateReadingRequest {
            request_id: None,
            idempotency_key: None,
            product_context: ProductContext {
                product_code: product_code.into(),
                interpretation_profile_code: profile.map(str::to_string),
                user_language: "fr".into(),
                audience_level: AudienceLevel::Beginner,
            },
            astro_result: AstroCalculationPayload {
                contract_version: "natal_structured_v13".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({}),
            },
            astrologer_profile: AstrologerProfile {
                profile_id: None,
                name: None,
                tone: ToneProfile::Warm,
                jargon_level: JargonLevel::Beginner,
                wording_style: WordingStyle::Clear,
                preferred_domains: vec![],
                forbidden_wording: vec![],
                custom_instructions: None,
            },
            engine: EngineParams::default(),
            response_contract: ResponseContract {
                output_schema_version: "natal_reading_v1".into(),
                generation_mode: mode,
                format: OutputFormat::StructuredJson,
                chapters: vec![],
                global_max_tokens: None,
                include_astro_sources: true,
                include_legal_disclaimer: true,
            },
            safety_policy: None,
        }
    }

    #[test]
    fn migrates_legacy_natal_premium_product_code() {
        let catalog = catalog_with_profiles();
        let mut request = base_request("natal_premium", None, GenerationMode::SinglePass);
        InterpretationProfileResolver::normalize_request(&mut request, &catalog).unwrap();
        assert_eq!(request.product_context.product_code, NATAL_PROMPTER_PRODUCT);
        assert_eq!(
            request.product_context.interpretation_profile_code.as_deref(),
            Some("natal_premium")
        );
        assert_eq!(
            request.response_contract.generation_mode,
            GenerationMode::ChapterOrchestrated
        );
    }

    #[test]
    fn aligns_generation_mode_from_profile() {
        let catalog = catalog_with_profiles();
        let mut request = base_request(
            NATAL_PROMPTER_PRODUCT,
            Some("natal_light"),
            GenerationMode::ChapterOrchestrated,
        );
        InterpretationProfileResolver::normalize_request(&mut request, &catalog).unwrap();
        assert_eq!(
            request.response_contract.generation_mode,
            GenerationMode::SinglePass
        );
    }
}

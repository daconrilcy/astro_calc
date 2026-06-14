use astral_llm_domain::{
    interpretation_profile::{
        InterpretationProfile, LEGACY_PRODUCT_NATAL_BASIC, LEGACY_PRODUCT_NATAL_PREMIUM,
        NATAL_PROMPTER_PRODUCT, PROFILE_NATAL_BASIC, PROFILE_NATAL_PREMIUM,
    },
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
        legacy_product_code_shim_available: bool,
    ) -> Result<(), GenerationError> {
        Self::migrate_legacy_product_codes(request, legacy_product_code_shim_available)?;

        if request.product_context.product_code != NATAL_PROMPTER_PRODUCT {
            return Ok(());
        }

        let profile_code = Self::required_profile_code(request)?;
        let profile = catalog
            .interpretation_profile(profile_code)
            .ok_or_else(|| profile_not_found(profile_code))?;

        profile.validate().map_err(|msg| {
            GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                format!("invalid interpretation profile: {msg}"),
                serde_json::json!({ "profile_code": profile_code }),
            )
        })?;

        request.response_contract.generation_mode = profile.document.generation_mode;
        if request.engine.domain_count.is_none() {
            request.engine.domain_count = Some(profile.default_domain_count());
        }
        Ok(())
    }

    /// Rate limit « premium » : profils avec evidence ou gate qualite bloquante (ex. `natal_premium`).
    pub fn requires_premium_rate_limit(
        request: &GenerateReadingRequest,
        catalog: &SharedCanonicalCatalog,
    ) -> bool {
        Self::profile_for_request(request, catalog)
            .map(|p| p.evidence_enabled() || p.blocking_quality_gate())
            .unwrap_or(false)
    }

    fn profile_for_request<'a>(
        request: &'a GenerateReadingRequest,
        catalog: &'a SharedCanonicalCatalog,
    ) -> Option<&'a InterpretationProfile> {
        if request.product_context.product_code != NATAL_PROMPTER_PRODUCT {
            return None;
        }
        let code = request
            .product_context
            .interpretation_profile_code
            .as_deref()?
            .trim();
        if code.is_empty() {
            return None;
        }
        catalog.interpretation_profile(code)
    }

    fn migrate_legacy_product_codes(
        request: &mut GenerateReadingRequest,
        legacy_product_code_shim_available: bool,
    ) -> Result<(), GenerationError> {
        let implied_profile = match request.product_context.product_code.as_str() {
            LEGACY_PRODUCT_NATAL_PREMIUM => Some(PROFILE_NATAL_PREMIUM),
            LEGACY_PRODUCT_NATAL_BASIC => Some(PROFILE_NATAL_BASIC),
            _ => return Ok(()),
        };

        if !legacy_product_code_shim_available {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                "legacy product_code shim is disabled; use natal_prompter + interpretation_profile_code",
                serde_json::json!({
                    "legacy_product_code": request.product_context.product_code,
                    "expected_product_code": NATAL_PROMPTER_PRODUCT,
                    "expected_profile_code": implied_profile,
                }),
            ));
        }

        let legacy_code = request.product_context.product_code.clone();
        if let Some(existing) = request
            .product_context
            .interpretation_profile_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if existing != implied_profile.unwrap() {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::ProductPolicyViolation,
                    "legacy product_code conflicts with interpretation_profile_code",
                    serde_json::json!({
                        "legacy_product_code": legacy_code,
                        "interpretation_profile_code": existing,
                        "expected_profile_code": implied_profile,
                    }),
                ));
            }
        }

        tracing::warn!(
            legacy_product_code = %legacy_code,
            implied_profile_code = implied_profile,
            "legacy product_code; use natal_prompter + interpretation_profile_code"
        );
        request.product_context.product_code = NATAL_PROMPTER_PRODUCT.into();
        request.product_context.interpretation_profile_code = Some(implied_profile.unwrap().into());
        Ok(())
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
        let profile = catalog
            .interpretation_profile(profile_code)
            .ok_or_else(|| profile_not_found(profile_code))?;

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

        if let Some(ctx) = &interpretation {
            let planned = ctx
                .profile
                .planned_chapter_count(request.engine.domain_count);
            if planned > policy.max_chapters {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::ProductPolicyViolation,
                    format!(
                        "planned chapter count exceeds profile maximum ({})",
                        policy.max_chapters
                    ),
                    serde_json::json!({
                        "planned_chapters": planned,
                        "max_chapters": policy.max_chapters,
                        "profile_code": ctx.profile.profile_code,
                    }),
                ));
            }
        }

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

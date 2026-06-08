//! Regles de validation redactionnelle (fixtures et profils bloquants).

use std::sync::Arc;

use astral_llm_domain::{
    generation_request::{AudienceLevel, GenerateReadingRequest},
    generation_response::NatalReadingResponse,
    output_contract::GenerationMode,
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::{bootstrap_interpretation_profiles, CanonicalCatalog};

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::interpretation_profile_resolver::InterpretationProfileResolver;
use crate::reading_quality_validator::{
    requires_blocking_quality_gate, thresholds_for_request, ReadingQualityReport,
    ReadingQualityValidator,
};
use crate::text_reprocessing_service_adapter::reprocess_natal_theme_with_context;
use astral_llm_domain::TextRetreatmentRequestContext;

fn editorial_catalog() -> Arc<CanonicalCatalog> {
    Arc::new(CanonicalCatalog {
        interpretation_profiles: bootstrap_interpretation_profiles(),
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
pub struct EditorialFixtureSpec {
    pub fixture_id: String,
    pub product_code: String,
    pub audience_level: AudienceLevel,
    pub user_language: String,
    pub generation_mode: GenerationMode,
}

pub struct EditorialValidator;

impl EditorialValidator {
    pub fn validate_fixture(
        spec: &EditorialFixtureSpec,
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
    ) -> Result<ReadingQualityReport, GenerationError> {
        if request.product_context.product_code != spec.product_code {
            return Err(GenerationError::new(
                GenerationErrorCode::InvalidInput,
                "fixture product_code mismatch",
            ));
        }
        Self::validate_reading(spec, request, reading)
    }

    pub fn validate_reading(
        spec: &EditorialFixtureSpec,
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
    ) -> Result<ReadingQualityReport, GenerationError> {
        let catalog = editorial_catalog();
        let mut request = request.clone();
        InterpretationProfileResolver::normalize_request(&mut request, &catalog)?;
        let interpretation = InterpretationProfileResolver::resolve(&request, &catalog)?;

        let mut violations = Vec::new();

        if reading.language != spec.user_language {
            violations.push(format!(
                "expected language {}, got {}",
                spec.user_language, reading.language
            ));
        }

        if has_fatalistic_wording(&corpus(reading)) {
            violations.push("fatalistic wording detected".into());
        }

        if has_forbidden_advice(&corpus(reading)) {
            violations.push("forbidden medical/legal/financial advice detected".into());
        }

        let interpretation_ref = interpretation.as_ref();
        let mut reading_for_quality = reading.clone();
        let mut thresholds = thresholds_for_request(&request, interpretation_ref);
        let fixture_min_astro_basis = interpretation_ref
            .map(|ctx| {
                ctx.profile
                    .document
                    .quality
                    .min_astro_basis_refs_per_chapter as usize
            })
            .unwrap_or(thresholds.min_astro_basis_per_chapter as usize);
        thresholds.min_astro_basis_per_chapter = fixture_min_astro_basis as u8;
        let _ = reprocess_natal_theme_with_context(
            &mut reading_for_quality,
            &spec.user_language,
            TextRetreatmentRequestContext {
                profile_code: request.product_context.interpretation_profile_code.clone(),
                product_code: Some(request.product_context.product_code.clone()),
                audience_level: Some(format!("{:?}", request.product_context.audience_level)),
                min_astro_basis_per_chapter: Some(fixture_min_astro_basis),
                ..TextRetreatmentRequestContext::default()
            },
        )
        .map_err(|err| {
            GenerationError::new(
                GenerationErrorCode::ReadingQualityFailed,
                format!("text reprocessing editorial quality failed: {err}"),
            )
        })?;
        let blocking = requires_blocking_quality_gate(&request, interpretation_ref);
        let locale = AstroLabelHumanizer::locale_key(&request.product_context.user_language);
        let synthesis_min_astro =
            interpretation_ref.map(|c| c.profile.synthesis_min_astro_basis_refs());
        let synthesis_min_words =
            interpretation_ref.map(|c| c.profile.synthesis_word_targets().0 as usize);
        let quality = ReadingQualityValidator::assess_with_thresholds(
            &request,
            &reading_for_quality,
            &thresholds,
            locale,
            blocking,
            synthesis_min_astro,
            synthesis_min_words,
        );

        if blocking && !quality.is_acceptable() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ReadingQualityFailed,
                "reading quality below profile threshold",
                serde_json::json!({
                    "fixture_id": spec.fixture_id,
                    "profile_code": request
                        .product_context
                        .interpretation_profile_code,
                    "warnings": quality.warnings,
                }),
            ));
        }

        if !blocking && !quality.is_acceptable() {
            violations.extend(quality.warnings.clone());
        }

        if matches!(spec.audience_level, AudienceLevel::Beginner)
            && has_excessive_jargon(&corpus(reading))
        {
            violations.push("excessive jargon for beginner audience".into());
        }

        if !violations.is_empty() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ReadingQualityFailed,
                "editorial fixture validation failed",
                serde_json::json!({
                    "fixture_id": spec.fixture_id,
                    "violations": violations,
                    "quality_warnings": quality.warnings,
                }),
            ));
        }

        Ok(quality)
    }
}

fn corpus(reading: &NatalReadingResponse) -> String {
    reading
        .chapters
        .iter()
        .map(|c| c.body.to_lowercase())
        .collect::<Vec<_>>()
        .join("\n")
}

fn has_fatalistic_wording(corpus: &str) -> bool {
    [
        "destin inevitable",
        "tu vas mourir",
        "certitude absolue",
        "fatalité",
        "inevitably die",
        "certain death",
    ]
    .iter()
    .any(|p| corpus.contains(p))
}

fn has_forbidden_advice(corpus: &str) -> bool {
    [
        "consultez un medecin",
        "consult a doctor",
        "investissez dans",
        "buy this stock",
        "demandez a votre avocat",
        "sue them",
    ]
    .iter()
    .any(|p| corpus.contains(p))
}

fn has_excessive_jargon(corpus: &str) -> bool {
    ["quincunx", "pars fortunae", "maison xii", "biquintile"]
        .iter()
        .any(|j| corpus.contains(j))
}

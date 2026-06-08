//! Pipeline central de retraitement des textes LLM.
//!
//! Le branchement applicatif passe par `text_reprocessing_service_adapter` afin de garder les
//! contrats publics des services stables.

use std::collections::{HashMap, HashSet};

use astral_llm_domain::{
    TextChapterEvidenceKeys, TextLanguage, TextRetreatmentAuditAction, TextRetreatmentAuditItem,
    TextRetreatmentOperation, TextRetreatmentRequest, TextRetreatmentResponse,
    TextRetreatmentViolation, TextService, TextTarget, TextWordLimits, LANG_DE, LANG_EN, LANG_ES,
    LANG_FR, SERVICE_CALCULATOR_PROJECTION, SERVICE_HOROSCOPE_DAILY, SERVICE_HOROSCOPE_PERIOD,
    SERVICE_NATAL_SIMPLIFIED, SERVICE_NATAL_THEME, SERVICE_PROMPT_TRACE, SERVICE_SHARED,
};
use serde_json::{json, Value};

use crate::french_typography::restore_french_elisions;
use crate::reading_script_guard::sanitize_text_for_french_script;
use crate::summary_ux_rules::{count_words, split_sentences_fr};

const ALL_KNOWN_SERVICES: &[&str] = &[
    SERVICE_HOROSCOPE_DAILY,
    SERVICE_HOROSCOPE_PERIOD,
    SERVICE_NATAL_THEME,
    SERVICE_NATAL_SIMPLIFIED,
    SERVICE_CALCULATOR_PROJECTION,
    SERVICE_PROMPT_TRACE,
    SERVICE_SHARED,
];

#[derive(Debug, Clone)]
pub struct LanguageRuleSet {
    pub code: String,
    pub sentence_prefix: String,
    pub default_summary_title: String,
    pub fallback_sentence: String,
    pub fallback_summary_text: String,
    pub fallback_advice: String,
    pub repetitive_replacements: Vec<(String, String)>,
    pub humanized_labels: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ServiceRuleSet {
    pub code: String,
    pub default_operations: Vec<TextRetreatmentOperation>,
    pub word_limits: TextWordLimits,
    pub fallback_summary_title: String,
}

#[derive(Debug, Clone)]
pub struct LanguageRegistry {
    rules: HashMap<String, LanguageRuleSet>,
}

impl LanguageRegistry {
    pub fn default_rules() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
        };
        registry.insert(LanguageRuleSet {
            code: LANG_FR.into(),
            sentence_prefix: "Dans cette perspective, ".into(),
            default_summary_title: "Lecture indicative".into(),
            fallback_sentence: "Cette formulation reste symbolique et doit rester lisible.".into(),
            fallback_summary_text:
                "Cette lecture reste indicative et reformule les points utiles sans ajouter de certitude."
                    .into(),
            fallback_advice: "Gardez une priorité simple et vérifiable.".into(),
            repetitive_replacements: vec![
                ("restez concret".into(), "gardez une prise directe".into()),
                (
                    "gardez une marge".into(),
                    "préservez un espace de recul".into(),
                ),
                ("clarifier".into(), "rendre lisible".into()),
            ],
            humanized_labels: vec![
                ("sun".into(), "Soleil".into()),
                ("moon".into(), "Lune".into()),
                ("ascendant".into(), "Ascendant".into()),
                ("relationship".into(), "relations".into()),
                ("organization".into(), "organisation".into()),
                ("shared_resources".into(), "ressources partagées".into()),
                ("private_public".into(), "privé / public".into()),
                ("career".into(), "carrière".into()),
            ],
        });
        registry.insert(LanguageRuleSet {
            code: LANG_EN.into(),
            sentence_prefix: "In this perspective, ".into(),
            default_summary_title: "Indicative reading".into(),
            fallback_sentence: "This wording remains symbolic and should stay readable.".into(),
            fallback_summary_text:
                "This reading remains indicative and keeps the useful points clear.".into(),
            fallback_advice: "Keep one simple and verifiable priority.".into(),
            repetitive_replacements: vec![("stay concrete".into(), "keep a direct handle".into())],
            humanized_labels: vec![
                ("sun".into(), "Sun".into()),
                ("moon".into(), "Moon".into()),
                ("ascendant".into(), "Ascendant".into()),
                ("relationship".into(), "Relationship".into()),
                ("organization".into(), "Organization".into()),
                ("shared_resources".into(), "Shared resources".into()),
                ("private_public".into(), "Private / public".into()),
                ("career".into(), "Career".into()),
            ],
        });
        registry.insert(LanguageRuleSet {
            code: LANG_ES.into(),
            sentence_prefix: "En esta perspectiva, ".into(),
            default_summary_title: "Lectura indicativa".into(),
            fallback_sentence: "Esta formulación sigue siendo simbólica y legible.".into(),
            fallback_summary_text:
                "Esta lectura sigue siendo indicativa y conserva una formulación clara.".into(),
            fallback_advice: "Mantenga una prioridad simple y verificable.".into(),
            repetitive_replacements: vec![],
            humanized_labels: vec![
                ("sun".into(), "Sol".into()),
                ("moon".into(), "Luna".into()),
                ("ascendant".into(), "Ascendente".into()),
                ("relationship".into(), "relaciones".into()),
                ("organization".into(), "organización".into()),
                ("shared_resources".into(), "recursos compartidos".into()),
                ("private_public".into(), "privado / público".into()),
                ("career".into(), "carrera".into()),
            ],
        });
        registry.insert(LanguageRuleSet {
            code: LANG_DE.into(),
            sentence_prefix: "In dieser Perspektive ".into(),
            default_summary_title: "Hinweisende Deutung".into(),
            fallback_sentence: "Diese Formulierung bleibt symbolisch und lesbar.".into(),
            fallback_summary_text: "Diese Deutung bleibt hinweisend und klar formuliert.".into(),
            fallback_advice: "Behalten Sie eine einfache und überprüfbare Priorität.".into(),
            repetitive_replacements: vec![],
            humanized_labels: vec![
                ("sun".into(), "Sonne".into()),
                ("moon".into(), "Mond".into()),
                ("ascendant".into(), "Aszendent".into()),
                ("relationship".into(), "Beziehungen".into()),
                ("organization".into(), "Organisation".into()),
                ("shared_resources".into(), "gemeinsame Ressourcen".into()),
                ("private_public".into(), "privat / öffentlich".into()),
                ("career".into(), "Karriere".into()),
            ],
        });
        registry
    }

    pub fn insert(&mut self, rules: LanguageRuleSet) {
        self.rules.insert(rules.code.clone(), rules);
    }

    pub fn get(&self, language: &TextLanguage) -> Option<&LanguageRuleSet> {
        self.rules.get(&language.code)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::default_rules()
    }
}

#[derive(Debug, Clone)]
pub struct ServiceRegistry {
    rules: HashMap<String, ServiceRuleSet>,
}

impl ServiceRegistry {
    pub fn default_rules() -> Self {
        let mut registry = Self {
            rules: HashMap::new(),
        };
        for code in ALL_KNOWN_SERVICES {
            registry.insert(default_service_rules(code));
        }
        registry
    }

    pub fn insert(&mut self, rules: ServiceRuleSet) {
        self.rules.insert(rules.code.clone(), rules);
    }

    pub fn get(&self, service: &TextService) -> Option<&ServiceRuleSet> {
        self.rules.get(&service.code)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::default_rules()
    }
}

fn default_service_rules(code: &str) -> ServiceRuleSet {
    let default_operations = match code {
        SERVICE_PROMPT_TRACE => vec![TextRetreatmentOperation::FormatTrace],
        SERVICE_CALCULATOR_PROJECTION => vec![
            TextRetreatmentOperation::HumanizeLabels,
            TextRetreatmentOperation::Sanitize,
        ],
        SERVICE_SHARED => vec![
            TextRetreatmentOperation::Sanitize,
            TextRetreatmentOperation::Typography,
        ],
        _ => vec![
            TextRetreatmentOperation::Sanitize,
            TextRetreatmentOperation::Typography,
            TextRetreatmentOperation::NormalizeLength,
            TextRetreatmentOperation::ReduceRepetition,
            TextRetreatmentOperation::ValidateQuality,
            TextRetreatmentOperation::BuildFallback,
        ],
    };
    ServiceRuleSet {
        code: code.into(),
        default_operations,
        word_limits: TextWordLimits {
            min_words: None,
            max_words: Some(if code == SERVICE_HOROSCOPE_PERIOD {
                60
            } else {
                40
            }),
            hard_limit_words: Some(if code == SERVICE_HOROSCOPE_PERIOD {
                90
            } else {
                60
            }),
        },
        fallback_summary_title: match code {
            SERVICE_HOROSCOPE_DAILY => "Votre tendance du jour",
            SERVICE_HOROSCOPE_PERIOD => "Vos 7 prochains jours",
            SERVICE_NATAL_SIMPLIFIED => "Lecture indicative",
            SERVICE_NATAL_THEME => "Synthèse astrologique",
            _ => "Texte retraité",
        }
        .into(),
    }
}

pub struct TextRetreatmentContext<'a> {
    pub request: &'a TextRetreatmentRequest,
    pub language_rules: Option<&'a LanguageRuleSet>,
    pub service_rules: Option<&'a ServiceRuleSet>,
}

pub trait TextRetreatmentProcessor: Send + Sync {
    fn id(&self) -> &'static str;
    fn operation(&self) -> TextRetreatmentOperation;
    fn supported_languages(&self) -> &'static [&'static str];
    fn supported_services(&self) -> &'static [&'static str];
    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome;

    fn supports(&self, ctx: &TextRetreatmentContext<'_>) -> bool {
        supports_code(self.supported_languages(), &ctx.request.language.code)
            && supports_code(self.supported_services(), &ctx.request.service.code)
    }
}

fn supports_code(supported: &[&str], code: &str) -> bool {
    supported.is_empty() || supported.iter().any(|item| *item == code)
}

#[derive(Debug, Default)]
pub struct ProcessorOutcome {
    pub changed_paths: Vec<String>,
    pub fallback_paths: Vec<String>,
    pub validated_paths: Vec<String>,
    pub warnings: Vec<String>,
    pub violations: Vec<TextRetreatmentViolation>,
}

pub struct ProcessorRegistry {
    processors: Vec<Box<dyn TextRetreatmentProcessor>>,
}

impl ProcessorRegistry {
    pub fn new(processors: Vec<Box<dyn TextRetreatmentProcessor>>) -> Self {
        Self { processors }
    }

    pub fn default_processors() -> Self {
        Self::new(vec![
            Box::new(ScriptSanitizerProcessor),
            Box::new(TypographyProcessor),
            Box::new(SentenceAndLengthProcessor),
            Box::new(RepetitionProcessor),
            Box::new(AstroLabelHumanizerProcessor),
            Box::new(AstroBasisProcessor),
            Box::new(AstroBasisDensityProcessor),
            Box::new(QualityValidationProcessor),
            Box::new(FallbackTextProcessor),
            Box::new(PromptGuidanceProcessor),
            Box::new(TraceFormattingProcessor),
        ])
    }

    pub fn processors(&self) -> &[Box<dyn TextRetreatmentProcessor>] {
        &self.processors
    }
}

impl Default for ProcessorRegistry {
    fn default() -> Self {
        Self::default_processors()
    }
}

pub struct TextRetreatmentPipeline {
    languages: LanguageRegistry,
    services: ServiceRegistry,
    processors: ProcessorRegistry,
}

impl TextRetreatmentPipeline {
    pub fn new(
        languages: LanguageRegistry,
        services: ServiceRegistry,
        processors: ProcessorRegistry,
    ) -> Self {
        Self {
            languages,
            services,
            processors,
        }
    }

    pub fn process(&self, request: TextRetreatmentRequest) -> TextRetreatmentResponse {
        let mut payload = request.payload.clone();
        let mut audit = Vec::new();
        let mut warnings = Vec::new();
        let mut violations = Vec::new();
        let operations = requested_operations(&request, self.services.get(&request.service));
        let ctx = TextRetreatmentContext {
            language_rules: self.languages.get(&request.language),
            service_rules: self.services.get(&request.service),
            request: &request,
        };
        if ctx.language_rules.is_none() {
            warnings.push(format!("unregistered_language:{}", request.language.code));
        }
        if ctx.service_rules.is_none() {
            warnings.push(format!("unregistered_service:{}", request.service.code));
        }

        for processor in self.processors.processors() {
            let operation = processor.operation();
            if !operations.contains(&operation) {
                continue;
            }
            if !processor.supports(&ctx) {
                audit.push(TextRetreatmentAuditItem {
                    processor_id: processor.id().into(),
                    operation,
                    field_path: None,
                    action: TextRetreatmentAuditAction::Skipped,
                    reason_code: "unsupported_language_or_service".into(),
                });
                continue;
            }

            let outcome = processor.process(&ctx, &mut payload);
            if outcome.changed_paths.is_empty()
                && outcome.fallback_paths.is_empty()
                && outcome.validated_paths.is_empty()
                && outcome.violations.is_empty()
            {
                audit.push(TextRetreatmentAuditItem {
                    processor_id: processor.id().into(),
                    operation: operation.clone(),
                    field_path: None,
                    action: TextRetreatmentAuditAction::Skipped,
                    reason_code: "no_applicable_change".into(),
                });
            }
            if outcome.changed_paths.is_empty()
                && outcome.fallback_paths.is_empty()
                && outcome.validated_paths.is_empty()
                && !outcome.violations.is_empty()
            {
                audit.push(TextRetreatmentAuditItem {
                    processor_id: processor.id().into(),
                    operation: operation.clone(),
                    field_path: None,
                    action: TextRetreatmentAuditAction::Validated,
                    reason_code: "violations_detected".into(),
                });
            }
            for path in outcome.changed_paths {
                audit.push(TextRetreatmentAuditItem {
                    processor_id: processor.id().into(),
                    operation: operation.clone(),
                    field_path: Some(path),
                    action: TextRetreatmentAuditAction::Changed,
                    reason_code: "updated".into(),
                });
            }
            for path in outcome.fallback_paths {
                audit.push(TextRetreatmentAuditItem {
                    processor_id: processor.id().into(),
                    operation: operation.clone(),
                    field_path: Some(path),
                    action: TextRetreatmentAuditAction::FallbackApplied,
                    reason_code: "fallback_applied".into(),
                });
            }
            for path in outcome.validated_paths {
                audit.push(TextRetreatmentAuditItem {
                    processor_id: processor.id().into(),
                    operation: operation.clone(),
                    field_path: Some(path),
                    action: TextRetreatmentAuditAction::Validated,
                    reason_code: "validated".into(),
                });
            }
            warnings.extend(outcome.warnings);
            violations.extend(outcome.violations);
        }

        let changed = payload != request.payload
            || audit.iter().any(|item| {
                matches!(
                    item.action,
                    TextRetreatmentAuditAction::Changed
                        | TextRetreatmentAuditAction::FallbackApplied
                )
            });
        TextRetreatmentResponse {
            payload,
            audit,
            warnings,
            violations,
            changed,
        }
    }
}

impl Default for TextRetreatmentPipeline {
    fn default() -> Self {
        Self::new(
            LanguageRegistry::default(),
            ServiceRegistry::default(),
            ProcessorRegistry::default(),
        )
    }
}

fn requested_operations(
    request: &TextRetreatmentRequest,
    service_rules: Option<&ServiceRuleSet>,
) -> HashSet<TextRetreatmentOperation> {
    if request.operations.is_empty() {
        return service_rules
            .map(|rules| rules.default_operations.iter().cloned().collect())
            .unwrap_or_default();
    }
    request.operations.iter().cloned().collect()
}

pub struct ScriptSanitizerProcessor;

impl TextRetreatmentProcessor for ScriptSanitizerProcessor {
    fn id(&self) -> &'static str {
        "script_sanitizer"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::Sanitize
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        mutate_strings(payload, "$", &mut |path, text| {
            if contains_prompt_injection(text) {
                outcome.violations.push(TextRetreatmentViolation {
                    code: "prompt_injection_like_text".into(),
                    field_path: Some(path.to_string()),
                    message: "text contains disallowed instruction-like content".into(),
                });
            }
            if is_technical_string_path(path) {
                return None;
            }

            let mut updated = text.to_string();
            if ctx.request.service.code == SERVICE_HOROSCOPE_PERIOD {
                updated = sanitize_period_public_text(&updated);
            }
            if ctx.request.language.code == LANG_FR {
                let (clean, _) = sanitize_text_for_french_script(&updated);
                updated = clean;
            }
            if updated != text {
                outcome.changed_paths.push(path.to_string());
                return Some(updated);
            }
            None
        });
        outcome
    }
}

pub struct TypographyProcessor;

impl TextRetreatmentProcessor for TypographyProcessor {
    fn id(&self) -> &'static str {
        "typography"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::Typography
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[LANG_FR]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, _ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        mutate_public_text_strings(payload, "$", &mut |path, text| {
            let (fixed, changed) = restore_french_elisions(text);
            let fixed = normalize_french_colon_spacing(&fixed);
            if changed || fixed != text {
                outcome.changed_paths.push(path.to_string());
                Some(fixed)
            } else {
                None
            }
        });
        outcome
    }
}

pub struct SentenceAndLengthProcessor;

impl TextRetreatmentProcessor for SentenceAndLengthProcessor {
    fn id(&self) -> &'static str {
        "sentence_and_length"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::NormalizeLength
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let limits = ctx
            .request
            .context
            .word_limits
            .as_ref()
            .or_else(|| ctx.service_rules.map(|rules| &rules.word_limits));
        let max_words = limits.and_then(|limits| limits.hard_limit_words.or(limits.max_words));
        let min_words = limits.and_then(|limits| limits.min_words);
        let fallback = ctx
            .language_rules
            .map(|rules| rules.fallback_sentence.as_str())
            .unwrap_or("This text remains symbolic and readable.");
        let mut outcome = ProcessorOutcome::default();

        mutate_public_text_strings(payload, "$", &mut |path, text| {
            if !is_length_controlled_path(path) {
                return None;
            }
            let mut updated = text.trim().to_string();
            if let Some(max) = max_words {
                if count_words(&updated) > max {
                    updated = truncate_complete(&updated, max);
                }
            }
            if let Some(min) = min_words {
                if count_words(&updated) < min && !updated.contains(fallback) {
                    if !updated.is_empty() && !updated.ends_with(' ') {
                        updated.push(' ');
                    }
                    updated.push_str(fallback);
                }
            }
            if updated != text {
                outcome.changed_paths.push(path.to_string());
                Some(updated)
            } else {
                None
            }
        });
        outcome
    }
}

pub struct RepetitionProcessor;

impl TextRetreatmentProcessor for RepetitionProcessor {
    fn id(&self) -> &'static str {
        "repetition"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::ReduceRepetition
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let replacements = ctx
            .language_rules
            .map(|rules| rules.repetitive_replacements.clone())
            .unwrap_or_default();
        let mut outcome = ProcessorOutcome::default();
        mutate_public_text_strings(payload, "$", &mut |path, text| {
            let mut updated = text.to_string();
            for (phrase, replacement) in &replacements {
                updated = replace_after_first_case_insensitive(&updated, phrase, replacement);
            }
            if updated != text {
                outcome.changed_paths.push(path.to_string());
                Some(updated)
            } else {
                None
            }
        });
        outcome
    }
}

pub struct AstroLabelHumanizerProcessor;

impl TextRetreatmentProcessor for AstroLabelHumanizerProcessor {
    fn id(&self) -> &'static str {
        "astro_label_humanizer"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::HumanizeLabels
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        humanize_label_fields(payload, "$", ctx.language_rules, &mut outcome);
        outcome
    }
}

pub struct AstroBasisProcessor;

impl TextRetreatmentProcessor for AstroBasisProcessor {
    fn id(&self) -> &'static str {
        "astro_basis"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::HumanizeLabels
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, _ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        normalize_astro_basis(payload, "$", &mut outcome);
        outcome
    }
}

pub struct AstroBasisDensityProcessor;

impl TextRetreatmentProcessor for AstroBasisDensityProcessor {
    fn id(&self) -> &'static str {
        "astro_basis_density"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::ValidateQuality
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[SERVICE_NATAL_THEME, SERVICE_NATAL_SIMPLIFIED]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        let Some(min_basis) = ctx.request.context.min_astro_basis_per_chapter else {
            return outcome;
        };
        if !matches!(ctx.request.target, TextTarget::NatalReading) {
            return outcome;
        }
        complete_astro_basis_density(
            payload,
            min_basis,
            &ctx.request.context.allowed_evidence_keys,
            &ctx.request.context.allowed_evidence_by_chapter,
            &mut outcome,
        );
        outcome
    }
}

pub struct QualityValidationProcessor;

impl TextRetreatmentProcessor for QualityValidationProcessor {
    fn id(&self) -> &'static str {
        "quality_validation"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::ValidateQuality
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        let text = collect_public_text(payload);
        if text.trim().is_empty() {
            outcome.violations.push(TextRetreatmentViolation {
                code: "empty_public_text".into(),
                field_path: None,
                message: "payload contains no public text".into(),
            });
        }
        let lower = text.to_lowercase();
        for forbidden in ["destin inevitable", "certain death", "ignore previous"] {
            if lower.contains(forbidden) {
                outcome.violations.push(TextRetreatmentViolation {
                    code: "forbidden_wording".into(),
                    field_path: None,
                    message: format!("forbidden wording detected: {forbidden}"),
                });
            }
        }
        if matches!(
            ctx.request.target,
            TextTarget::NatalReading | TextTarget::HoroscopeDailyResponse
        ) {
            outcome.validated_paths.push("$".into());
        }
        outcome
    }
}

pub struct FallbackTextProcessor;

impl TextRetreatmentProcessor for FallbackTextProcessor {
    fn id(&self) -> &'static str {
        "fallback_text"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::BuildFallback
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        if !payload.is_object() {
            outcome
                .warnings
                .push("fallback_requires_object_payload".into());
            return outcome;
        }
        let title = ctx
            .service_rules
            .map(|rules| rules.fallback_summary_title.as_str())
            .or_else(|| {
                ctx.language_rules
                    .map(|rules| rules.default_summary_title.as_str())
            })
            .unwrap_or("Texte retraité");
        if payload.get("summary").is_none() {
            payload["summary"] = json!({ "title": title, "text": fallback_summary_text(ctx) });
            outcome.fallback_paths.push("$.summary".into());
        }
        if ctx.request.service.code == SERVICE_HOROSCOPE_DAILY && payload.get("advice").is_none() {
            payload["advice"] = json!(ctx
                .language_rules
                .map(|rules| rules.fallback_advice.as_str())
                .unwrap_or("Gardez une priorité simple."));
            outcome.fallback_paths.push("$.advice".into());
        }
        outcome
    }
}

pub struct PromptGuidanceProcessor;

impl TextRetreatmentProcessor for PromptGuidanceProcessor {
    fn id(&self) -> &'static str {
        "prompt_guidance"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::BuildPromptGuidance
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        if !payload.is_object() {
            outcome
                .warnings
                .push("prompt_guidance_requires_object_payload".into());
            return outcome;
        }
        if payload.get("prompt_guidance").is_none() {
            payload["prompt_guidance"] = json!(format!(
                "OUTPUT_LANGUAGE: {}. Keep fact_ids unchanged and avoid repeated openings.",
                ctx.request.language.code
            ));
            outcome.changed_paths.push("$.prompt_guidance".into());
        }
        outcome
    }
}

pub struct TraceFormattingProcessor;

impl TextRetreatmentProcessor for TraceFormattingProcessor {
    fn id(&self) -> &'static str {
        "trace_formatting"
    }

    fn operation(&self) -> TextRetreatmentOperation {
        TextRetreatmentOperation::FormatTrace
    }

    fn supported_languages(&self) -> &'static [&'static str] {
        &[]
    }

    fn supported_services(&self) -> &'static [&'static str] {
        &[]
    }

    fn process(&self, _ctx: &TextRetreatmentContext<'_>, payload: &mut Value) -> ProcessorOutcome {
        let mut outcome = ProcessorOutcome::default();
        let Some(messages) = payload.get("messages").and_then(Value::as_array) else {
            return outcome;
        };
        let formatted = messages
            .iter()
            .filter_map(|message| {
                Some(format!(
                    "<<< {} >>>\n{}\n",
                    message.get("role")?.as_str()?,
                    message.get("content")?.as_str()?
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !formatted.is_empty() {
            payload["formatted_trace"] = json!(formatted);
            outcome.changed_paths.push("$.formatted_trace".into());
        }
        outcome
    }
}

fn mutate_strings(value: &mut Value, path: &str, f: &mut impl FnMut(&str, &str) -> Option<String>) {
    match value {
        Value::String(text) => {
            if let Some(updated) = f(path, text) {
                *text = updated;
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter_mut().enumerate() {
                mutate_strings(item, &format!("{path}[{idx}]"), f);
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                mutate_strings(child, &format!("{path}.{key}"), f);
            }
        }
        _ => {}
    }
}

fn mutate_public_text_strings(
    value: &mut Value,
    path: &str,
    f: &mut impl FnMut(&str, &str) -> Option<String>,
) {
    mutate_strings(value, path, &mut |path, text| {
        if is_technical_string_path(path) {
            None
        } else {
            f(path, text)
        }
    });
}

fn is_technical_string_path(path: &str) -> bool {
    let Some(key) = leaf_key(path) else {
        return false;
    };
    key == "code"
        || key == "id"
        || key == "role"
        || key == "interpretive_role"
        || key == "formatted_trace"
        || key.ends_with("_code")
        || key.ends_with("_id")
}

fn is_length_controlled_path(path: &str) -> bool {
    let Some(key) = leaf_key(path) else {
        return true;
    };
    matches!(
        key,
        "text" | "body" | "content" | "advice" | "watch_point" | "main"
    )
}

fn leaf_key(path: &str) -> Option<&str> {
    if path == "$" {
        return None;
    }
    path.rsplit('.')
        .next()
        .map(|segment| segment.split('[').next().unwrap_or(segment))
}

fn contains_prompt_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "ignore previous",
        "ignore safety",
        "override system",
        "ignore les instructions",
        "oublie les regles",
        "jailbreak",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

fn normalize_french_colon_spacing(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars = text.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if *ch == ':' {
            if should_preserve_colon(&chars, index) {
                out.push(':');
                continue;
            }
            while out.ends_with(' ') {
                out.pop();
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push(':');
            if chars
                .get(index + 1)
                .is_some_and(|next| !next.is_whitespace())
            {
                out.push(' ');
            }
            continue;
        }
        out.push(*ch);
    }
    out
}

fn should_preserve_colon(chars: &[char], index: usize) -> bool {
    let previous = index.checked_sub(1).and_then(|idx| chars.get(idx));
    let next = chars.get(index + 1);
    if previous.is_some_and(|ch| ch.is_ascii_digit()) && next.is_some_and(|ch| ch.is_ascii_digit())
    {
        return true;
    }
    if chars.get(index + 1) == Some(&'/') && chars.get(index + 2) == Some(&'/') {
        return true;
    }
    let token_start = chars[..index]
        .iter()
        .rposition(|ch| ch.is_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let token_end = chars[index + 1..]
        .iter()
        .position(|ch| ch.is_whitespace())
        .map(|offset| index + 1 + offset)
        .unwrap_or(chars.len());
    let token = chars[token_start..token_end].iter().collect::<String>();
    if token.contains("://") {
        return true;
    }
    false
}

fn sanitize_period_public_text(text: &str) -> String {
    let mut sanitized = text.to_string();
    for (from, to) in [
        ("natal_focus_hint", "nuance natale"),
        ("personalization_hint", "nuance personnelle"),
        ("theme_code", "thème"),
        ("evidence_key", "preuve"),
        ("moon_house_by_day", "passage lunaire"),
        ("transit_exact", "signal précis"),
        ("transit_active", "signal actif"),
        ("fake_period_calculator_v1", "calculateur de période"),
        ("fake_period_writer_v1", "rédaction de période"),
        ("natal_moon", "sensibilité natale"),
        ("natal_mercury", "pensée natale"),
        ("natal_venus", "attachement natal"),
        ("natal_mars", "élan natal"),
        ("natal_saturn", "responsabilité natale"),
        ("natal_", "natal "),
        ("period:", "signal de période "),
        ("slot:", "moment "),
        ("slot_", "moment "),
        ("snapshot", "repère"),
        ("raw_transits", "signaux astrologiques"),
    ] {
        sanitized = replace_ascii_case_insensitive(&sanitized, from, to);
    }
    for (from, to) in [
        ("organization", "organisation"),
        ("relationship", "relations"),
        ("energy", "énergie"),
        ("clarity", "clarté"),
        ("integration", "intégration"),
        ("focused", "concentré"),
        ("focus", "attention"),
        ("supportive", "soutenant"),
        ("careful", "vigilant"),
        ("active", "dynamique"),
        ("mixed", "nuancé"),
        ("fluid", "fluide"),
        ("tense", "sous tension"),
    ] {
        sanitized = replace_ascii_token_case_insensitive(&sanitized, from, to);
    }
    sanitize_period_french_fragments(&sanitize_period_sentence_boundaries(
        &normalize_french_colon_spacing(&sanitize_period_broken_sentences(&sanitized)),
    ))
}

fn sanitize_period_french_fragments(text: &str) -> String {
    text.replace("tout s’dynamique", "tout s'accélère")
        .replace("tout s'dynamique", "tout s'accélère")
        .replace("s’dynamique", "s'accélère")
        .replace("s'dynamique", "s'accélère")
        .replace("d’accélère", "s'accélère")
        .replace("d'accélère", "s'accélère")
        .replace("rédynamique", "dynamisante")
        .replace("redynamique", "dynamisante")
}

fn sanitize_period_broken_sentences(text: &str) -> String {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            sentences.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current);
    }
    if sentences.len() <= 1 {
        let trimmed = text.trim().trim_end_matches(['.', '!', '?']);
        if period_is_broken_sentence_tail(trimmed) {
            return "Vos repères personnels gardent un appui concret pour avancer avec mesure."
                .to_string();
        }
        return text.to_string();
    }
    let filtered = sentences
        .iter()
        .filter(|sentence| {
            let trimmed = sentence.trim().trim_end_matches(['.', '!', '?']);
            !period_is_broken_sentence_tail(trimmed)
        })
        .map(|sentence| sentence.trim())
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if filtered.trim().is_empty() {
        text.to_string()
    } else {
        filtered
    }
}

fn sanitize_period_sentence_boundaries(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = false;
    for ch in text.chars() {
        if capitalize_next {
            if ch.is_whitespace() {
                out.push(ch);
                continue;
            }
            for upper in ch.to_uppercase() {
                out.push(upper);
            }
            capitalize_next = false;
            continue;
        }
        out.push(ch);
        if matches!(ch, '.' | '!') || ch == '?' {
            capitalize_next = true;
        }
    }
    out
}

fn period_is_broken_sentence_tail(tail: &str) -> bool {
    let normalized = tail
        .trim()
        .trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '\'' | '’' | '“' | '”' | '"'))
        .to_lowercase();
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        [] => false,
        [last] => period_is_weak_sentence_ending(last),
        [.., "à", "la" | "l" | "l'"] => true,
        [.., "de", "la" | "l" | "l'"] => true,
        [.., last] => period_is_weak_sentence_ending(last),
    }
}

fn period_is_weak_sentence_ending(word: &str) -> bool {
    matches!(
        word,
        "à" | "a"
            | "de"
            | "d"
            | "d'"
            | "l"
            | "l'"
            | "le"
            | "la"
            | "les"
            | "un"
            | "une"
            | "des"
            | "du"
            | "et"
            | "ou"
            | "mais"
            | "pour"
            | "avec"
            | "sans"
            | "dans"
            | "sur"
            | "plutôt"
            | "que"
    )
}

fn replace_ascii_case_insensitive(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_from = from.to_lowercase();
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(relative) = lower_text[cursor..].find(&lower_from) {
        let start = cursor + relative;
        output.push_str(&text[cursor..start]);
        output.push_str(to);
        cursor = start + from.len();
    }
    output.push_str(&text[cursor..]);
    output
}

fn replace_ascii_token_case_insensitive(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_from = from.to_lowercase();
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    while let Some(relative) = lower_text[cursor..].find(&lower_from) {
        let start = cursor + relative;
        let end = start + from.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        let is_token = before
            .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
            .unwrap_or(true)
            && after
                .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(true);
        output.push_str(&text[cursor..start]);
        if is_token {
            output.push_str(to);
        } else {
            output.push_str(&text[start..end]);
        }
        cursor = end;
    }
    output.push_str(&text[cursor..]);
    output
}

fn truncate_complete(text: &str, max_words: usize) -> String {
    if count_words(text) <= max_words {
        return text.trim().to_string();
    }
    let mut out = String::new();
    for sentence in split_sentences_fr(text) {
        let candidate = if out.is_empty() {
            sentence
        } else {
            format!("{out} {sentence}")
        };
        if count_words(&candidate) > max_words {
            break;
        }
        out = candidate;
    }
    if !out.trim().is_empty() {
        return out;
    }
    let mut words = text.split_whitespace().take(max_words).collect::<Vec<_>>();
    while words
        .last()
        .map(|word| is_weak_ending(word))
        .unwrap_or(false)
    {
        words.pop();
    }
    let mut compact = words.join(" ");
    compact = compact.trim_end_matches([',', ';', ':']).to_string();
    if !compact.ends_with(['.', '!', '?']) {
        compact.push('.');
    }
    compact
}

fn is_weak_ending(word: &str) -> bool {
    matches!(
        word.trim_matches(|ch: char| !ch.is_alphabetic())
            .to_lowercase()
            .as_str(),
        "et" | "à"
            | "a"
            | "de"
            | "pour"
            | "avec"
            | "sans"
            | "dans"
            | "sur"
            | "the"
            | "and"
            | "of"
            | "to"
            | "with"
    )
}

fn replace_after_first_case_insensitive(text: &str, phrase: &str, replacement: &str) -> String {
    let lower = text.to_lowercase();
    let phrase_lower = phrase.to_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    let mut seen = false;
    for (index, _) in lower.match_indices(&phrase_lower) {
        out.push_str(&text[cursor..index]);
        let end = index + phrase.len();
        if seen {
            out.push_str(replacement);
        } else {
            out.push_str(&text[index..end]);
            seen = true;
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn humanize_label_fields(
    value: &mut Value,
    path: &str,
    language_rules: Option<&LanguageRuleSet>,
    outcome: &mut ProcessorOutcome,
) {
    match value {
        Value::Object(map) => {
            let keys = map.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    let child_path = format!("{path}.{key}");
                    if should_humanize_key(&key) {
                        if let Some(raw) = child.as_str() {
                            let label = humanize_code(raw, language_rules);
                            if label != raw {
                                *child = json!(label);
                                outcome.changed_paths.push(child_path);
                                continue;
                            }
                        }
                    }
                    humanize_label_fields(child, &child_path, language_rules, outcome);
                }
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter_mut().enumerate() {
                humanize_label_fields(item, &format!("{path}[{idx}]"), language_rules, outcome);
            }
        }
        _ => {}
    }
}

fn should_humanize_key(key: &str) -> bool {
    matches!(
        key,
        "theme_code" | "tone_code" | "object_code" | "sign_code" | "axis_code" | "label"
    )
}

fn humanize_code(raw: &str, language_rules: Option<&LanguageRuleSet>) -> String {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return cleaned.into();
    }
    if let Some(label) = language_rules.and_then(|rules| {
        rules
            .humanized_labels
            .iter()
            .find(|(code, _)| code == cleaned)
            .map(|(_, label)| label.clone())
    }) {
        label
    } else if cleaned.contains('_') {
        cleaned
            .split('_')
            .filter(|part| !part.is_empty())
            .map(title_case)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        title_case(cleaned)
    }
}

fn title_case(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn normalize_astro_basis(value: &mut Value, path: &str, outcome: &mut ProcessorOutcome) {
    match value {
        Value::Object(map) => {
            if let Some(items) = map.get_mut("astro_basis").and_then(Value::as_array_mut) {
                for (idx, item) in items.iter_mut().enumerate() {
                    if let Some(role) = item.get_mut("interpretive_role") {
                        if let Some(raw) = role.as_str() {
                            let normalized = normalize_role(raw);
                            if normalized != raw {
                                *role = json!(normalized);
                                outcome
                                    .changed_paths
                                    .push(format!("{path}.astro_basis[{idx}].interpretive_role"));
                            }
                        }
                    }
                }
            }
            for (key, child) in map {
                normalize_astro_basis(child, &format!("{path}.{key}"), outcome);
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter_mut().enumerate() {
                normalize_astro_basis(item, &format!("{path}[{idx}]"), outcome);
            }
        }
        _ => {}
    }
}

fn normalize_role(raw: &str) -> &'static str {
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        "core" | "principal" | "fondement" | "fondement principal" => "core",
        "supporting" | "soutien" | "support" => "supporting",
        "nuance" => "nuance",
        "domain_score" | "signal de selection du domaine" => "domain_score",
        _ if lower.contains("nuance") => "nuance",
        _ if lower.contains("support") || lower.contains("soutien") => "supporting",
        _ if lower.contains("core") || lower.contains("principal") => "core",
        _ => "supporting",
    }
}

fn complete_astro_basis_density(
    value: &mut Value,
    min_basis: usize,
    allowed_evidence_keys: &[String],
    allowed_evidence_by_chapter: &[TextChapterEvidenceKeys],
    outcome: &mut ProcessorOutcome,
) {
    let Some(chapters) = value.get_mut("chapters").and_then(Value::as_array_mut) else {
        return;
    };
    let chapter_count = chapters.len();
    for (chapter_idx, chapter) in chapters.iter_mut().enumerate() {
        let chapter_code = chapter
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("chapter")
            .to_string();
        let Some(basis) = chapter.get_mut("astro_basis").and_then(Value::as_array_mut) else {
            continue;
        };
        let valid_count = basis
            .iter()
            .filter(|item| {
                item.get("factor")
                    .and_then(Value::as_str)
                    .map(|factor| !factor.trim().is_empty())
                    .unwrap_or(false)
            })
            .count();
        if valid_count >= min_basis || basis.is_empty() {
            continue;
        }
        let mut cited = basis
            .iter()
            .filter_map(|item| item.get("fact_id").and_then(Value::as_str))
            .map(str::to_string)
            .collect::<HashSet<_>>();
        let Some(source_keys) = evidence_keys_for_chapter(
            &chapter_code,
            chapter_count,
            allowed_evidence_keys,
            allowed_evidence_by_chapter,
            outcome,
        ) else {
            continue;
        };
        let mut candidates = source_keys
            .iter()
            .filter(|key| !key.trim().is_empty())
            .filter(|key| !cited.contains(key.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            outcome.warnings.push(format!(
                "astro_basis_density_insufficient_allowed_evidence:{chapter_code}"
            ));
            continue;
        }
        candidates.reverse();
        while basis
            .iter()
            .filter(|item| {
                item.get("factor")
                    .and_then(Value::as_str)
                    .map(|factor| !factor.trim().is_empty())
                    .unwrap_or(false)
            })
            .count()
            < min_basis
        {
            let Some(fact_id) = candidates.pop() else {
                outcome.warnings.push(format!(
                    "astro_basis_density_insufficient_allowed_evidence:{chapter_code}"
                ));
                break;
            };
            let factor = factor_from_fact_id(&fact_id);
            basis.push(json!({
                "fact_id": fact_id.clone(),
                "label": title_case(&factor),
                "factor": factor,
                "interpretive_role": supplemental_role(&fact_id),
            }));
            cited.insert(fact_id);
            outcome
                .changed_paths
                .push(format!("$.chapters[{chapter_idx}].astro_basis"));
        }
    }
}

fn evidence_keys_for_chapter<'a>(
    chapter_code: &str,
    chapter_count: usize,
    allowed_evidence_keys: &'a [String],
    allowed_evidence_by_chapter: &'a [TextChapterEvidenceKeys],
    outcome: &mut ProcessorOutcome,
) -> Option<&'a [String]> {
    if let Some(scoped) = allowed_evidence_by_chapter
        .iter()
        .find(|entry| entry.chapter_code == chapter_code)
    {
        return Some(&scoped.fact_ids);
    }
    if chapter_count <= 1 {
        return Some(allowed_evidence_keys);
    }
    outcome.warnings.push(format!(
        "astro_basis_density_requires_chapter_scoped_evidence:{chapter_code}"
    ));
    None
}

fn factor_from_fact_id(fact_id: &str) -> String {
    let parts = fact_id.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        ["domain_score", domain] => clean_factor_token(domain),
        ["placement", object, ..] => clean_factor_token(object),
        ["angle", angle, ..] => clean_factor_token(angle),
        ["ruler", "angle", _, object, ..] => clean_factor_token(object),
        ["aspect", first, second, ..] => {
            format!(
                "{} {}",
                clean_factor_token(first),
                clean_factor_token(second)
            )
        }
        ["signal", "object_position", object, ..] => clean_factor_token(object),
        ["dominant_planet", object, ..] => clean_factor_token(object),
        ["element_balance", element, ..] => clean_factor_token(element),
        ["modality_balance", modality, ..] => clean_factor_token(modality),
        ["house_axis", axis, ..] => clean_factor_token(axis),
        ["house_emphasis", house, ..] => clean_factor_token(house),
        _ => parts
            .iter()
            .rev()
            .find(|part| !part.trim().is_empty())
            .map(|part| clean_factor_token(part))
            .unwrap_or_else(|| fact_id.replace('_', " ")),
    }
}

fn clean_factor_token(token: &str) -> String {
    token.replace('_', " ")
}

fn supplemental_role(fact_id: &str) -> &'static str {
    if fact_id.starts_with("domain_score:") {
        "domain_score"
    } else if fact_id.contains("nuance") {
        "nuance"
    } else {
        "supporting"
    }
}

fn fallback_summary_text(ctx: &TextRetreatmentContext<'_>) -> String {
    ctx.language_rules
        .map(|rules| rules.fallback_summary_text.clone())
        .unwrap_or_else(|| {
            "This reading remains indicative and keeps the useful points clear.".into()
        })
}

fn collect_public_text(value: &Value) -> String {
    let mut out = String::new();
    collect_public_strings(value, "$", &mut out);
    out
}

fn collect_public_strings(value: &Value, path: &str, out: &mut String) {
    match value {
        Value::String(text) => {
            if !is_technical_string_path(path) {
                out.push_str(text);
                out.push('\n');
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                collect_public_strings(item, &format!("{path}[{idx}]"), out);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                collect_public_strings(value, &format!("{path}.{key}"), out);
            }
        }
        _ => {}
    }
}

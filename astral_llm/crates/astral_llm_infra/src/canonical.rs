use std::sync::Arc;

use std::collections::HashMap;

use astral_llm_domain::{
    interpretation_profile::{InterpretationProfile, InterpretationProfileDocument},
    model_capability::ProviderModelRef,
    ProductGenerationPolicy, ProviderKind, ServiceLimits,
};

use crate::evidence_canonical::{bootstrap_evidence_catalog, EvidenceCanonicalCatalog};
use crate::i18n_canonical::{
    bootstrap_aspect_type_labels, bootstrap_astro_basis_roles, bootstrap_element_balance_labels,
    bootstrap_extra_object_sign_labels, bootstrap_house_theme_labels, bootstrap_modality_balance_labels,
    bootstrap_sect_labels, bootstrap_writing_locales, I18nLabelPair, WritingLocale,
};

/// Referentiel canonique charge depuis PostgreSQL (tables `llm_*`).
#[derive(Debug, Clone, Default)]
pub struct CanonicalCatalog {
    pub astrological_domains: Vec<String>,
    pub safety_patterns: Vec<SafetyPattern>,
    pub product_prompt_families: Vec<ProductPromptFamily>,
    pub product_generation_policies: Vec<ProductGenerationPolicy>,
    /// (locale, object_code) -> libelle affichable
    pub astro_object_labels: HashMap<(String, String), String>,
    /// (locale, sign_code) -> libelle affichable
    pub zodiac_sign_labels: HashMap<(String, String), String>,
    pub evidence: EvidenceCanonicalCatalog,
    pub writing_locales: Vec<WritingLocale>,
    pub astro_basis_roles: std::collections::HashSet<String>,
    pub aspect_type_labels: HashMap<(String, String), String>,
    pub element_balance_labels: HashMap<(String, String), I18nLabelPair>,
    pub modality_balance_labels: HashMap<(String, String), I18nLabelPair>,
    pub sect_labels: HashMap<(String, String), I18nLabelPair>,
    pub house_theme_labels: HashMap<(String, u8), I18nLabelPair>,
    /// profile_code -> profil actif
    pub interpretation_profiles: HashMap<String, InterpretationProfile>,
}

#[derive(Debug, Clone)]
pub struct SafetyPattern {
    pub pattern_type: String,
    pub locale: String,
    pub pattern: String,
}

#[derive(Debug, Clone)]
pub struct ProductPromptFamily {
    pub product_code: String,
    pub prompt_family: String,
    pub prompt_version: String,
}

pub type SharedCanonicalCatalog = Arc<CanonicalCatalog>;

/// Charge le referentiel depuis PostgreSQL. Retourne defaults vides si tables absentes.
pub async fn load_canonical_catalog(pool: &sqlx::PgPool) -> CanonicalCatalog {
    let mut catalog = CanonicalCatalog::default();

    if let Ok(rows) = sqlx::query_as::<_, (String,)>(
        "SELECT domain_code FROM llm_astrological_domains WHERE is_active = true ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await
    {
        catalog.astrological_domains = rows.into_iter().map(|(c,)| c).collect();
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT pattern_type, locale, pattern FROM llm_safety_content_patterns WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        catalog.safety_patterns = rows
            .into_iter()
            .map(|(pattern_type, locale, pattern)| SafetyPattern {
                pattern_type,
                locale,
                pattern,
            })
            .collect();
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT product_code, prompt_family, prompt_version FROM llm_product_prompt_profiles WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        catalog.product_prompt_families = rows
            .into_iter()
            .map(|(product_code, prompt_family, prompt_version)| ProductPromptFamily {
                product_code,
                prompt_family,
                prompt_version,
            })
            .collect();
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, i32, i32, i32, String, bool)>(
        "SELECT product_code, max_domains, max_chapters, max_output_tokens, max_reasoning_effort, allow_chapter_orchestrated \
         FROM llm_product_generation_policies WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        catalog.product_generation_policies = rows
            .into_iter()
            .map(
                |(product_code, max_domains, max_chapters, max_output_tokens, max_reasoning, allow_chapter)| {
                    ProductGenerationPolicy {
                        product_code,
                        allowed_providers: vec![],
                        allowed_models: vec![],
                        max_domains: max_domains as u8,
                        max_chapters: max_chapters as u8,
                        max_output_tokens: max_output_tokens as u32,
                        max_reasoning_effort: parse_reasoning_effort(&max_reasoning),
                        allow_chapter_orchestrated: allow_chapter,
                        min_astro_basis_refs_per_chapter: if allow_chapter { 1 } else { 0 },
                        min_interpretive_astro_basis_refs_per_chapter: if allow_chapter {
                            1
                        } else {
                            0
                        },
                        default_provider: None,
                        default_model: None,
                        economic_model: None,
                    }
                },
            )
            .collect();
        attach_product_allowed_models(pool, &mut catalog).await;
        attach_product_default_engine(pool, &mut catalog).await;
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT object_code, locale, label FROM llm_astro_object_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (object_code, locale, label) in rows {
            catalog
                .astro_object_labels
                .insert((locale, object_code), label);
        }
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT sign_code, locale, label FROM llm_zodiac_sign_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (sign_code, locale, label) in rows {
            catalog
                .zodiac_sign_labels
                .insert((locale, sign_code), label);
        }
    }

    load_interpretation_profiles_from_db(pool, &mut catalog).await;
    load_evidence_from_db(pool, &mut catalog).await;
    load_chapter_exclusions_from_db(pool, &mut catalog).await;
    apply_profile_evidence_to_catalog(&mut catalog);
    load_i18n_from_db(pool, &mut catalog).await;
    enrich_catalog_from_bootstrap(&mut catalog);
    if catalog.evidence.chapter_slots.is_empty() {
        catalog.evidence = bootstrap_evidence_catalog();
    } else if catalog.evidence.chapter_exclusions.is_empty() {
        catalog.evidence.chapter_exclusions =
            crate::evidence_canonical::bootstrap_evidence_catalog().chapter_exclusions;
    }
    catalog
}

async fn load_chapter_exclusions_from_db(pool: &sqlx::PgPool, catalog: &mut CanonicalCatalog) {
    let Ok(rows) = sqlx::query_as::<_, (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
        Vec<String>,
    )>(
        "SELECT rule_code, chapter_code, kind_code, object_code, fact_id_contains, \
         global_filler_only, global_filler_allow_contains \
         FROM llm_chapter_evidence_exclusions WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    else {
        return;
    };
    if rows.is_empty() {
        return;
    }
    catalog.evidence.chapter_exclusions = rows
        .into_iter()
        .map(
            |(rule_code, chapter_code, kind_code, object_code, fact_id_contains, global_filler_only, allow)| {
                astral_llm_domain::ChapterEvidenceExclusion {
                    rule_code,
                    chapter_code,
                    kind_code,
                    object_code,
                    fact_id_contains,
                    global_filler_only,
                    global_filler_allow_contains: allow,
                }
            },
        )
        .collect();
}

async fn load_evidence_from_db(pool: &sqlx::PgPool, catalog: &mut CanonicalCatalog) {
    if catalog.interpretation_profiles.contains_key("natal_premium") {
        return;
    }
    if let Ok(row) = sqlx::query_as::<_, (
        String,
        i32,
        i32,
        i32,
        f32,
        bool,
        i32,
        i32,
        i32,
        i32,
        i32,
    )>(
        "SELECT product_code, min_evidence_per_chapter, min_distinct_kind_families, \
         min_non_placement_if_available, max_core_overlap_ratio, domain_score_counts_in_minimum, \
         max_core_evidence, max_supporting_evidence, max_nuance_evidence, max_avoid_repeating, \
         COALESCE(max_supporting_semantic_chapters, 3) \
         FROM llm_premium_evidence_policies \
         WHERE is_active = true AND product_code = 'natal_premium'",
    )
    .fetch_optional(pool)
    .await
    {
        if let Some(r) = row {
            catalog.evidence.premium_policy = astral_llm_domain::PremiumEvidencePolicy {
                product_code: r.0,
                min_evidence_per_chapter: r.1 as u8,
                min_distinct_kind_families: r.2 as u8,
                min_non_placement_if_available: r.3 as u8,
                max_core_overlap_ratio: r.4,
                domain_score_counts_in_minimum: r.5,
                max_core_evidence: r.6 as u8,
                max_supporting_evidence: r.7 as u8,
                max_nuance_evidence: r.8 as u8,
                max_avoid_repeating: r.9 as u8,
                max_supporting_semantic_chapters: r.10 as u8,
            };
        }
    }
}

/// Complete les trous laisses par le schema SQL (listes provider/model, patterns safety).
pub fn enrich_catalog_from_bootstrap(catalog: &mut CanonicalCatalog) {
    if catalog.astrological_domains.is_empty() {
        catalog.astrological_domains = bootstrap_domains();
    }
    if catalog.safety_patterns.is_empty() {
        catalog.safety_patterns = bootstrap_safety_patterns();
    }

    for bootstrap in bootstrap_product_policies() {
        if let Some(existing) = catalog
            .product_generation_policies
            .iter_mut()
            .find(|p| p.product_code == bootstrap.product_code)
        {
            if existing.allowed_providers.is_empty() {
                existing.allowed_providers = bootstrap.allowed_providers.clone();
            }
            // allowed_models : uniquement depuis llm_product_allowed_models (pas de restriction bootstrap)
            if existing.min_astro_basis_refs_per_chapter == 0
                && bootstrap.min_astro_basis_refs_per_chapter > 0
            {
                existing.min_astro_basis_refs_per_chapter =
                    bootstrap.min_astro_basis_refs_per_chapter;
            }
            if existing.min_interpretive_astro_basis_refs_per_chapter == 0
                && bootstrap.min_interpretive_astro_basis_refs_per_chapter > 0
            {
                existing.min_interpretive_astro_basis_refs_per_chapter =
                    bootstrap.min_interpretive_astro_basis_refs_per_chapter;
            }
        } else {
            catalog.product_generation_policies.push(bootstrap);
        }
    }

    if catalog.product_generation_policies.is_empty() {
        catalog.product_generation_policies = bootstrap_product_policies();
    }
    if catalog.astro_object_labels.is_empty() {
        catalog.astro_object_labels = bootstrap_astro_object_labels();
    }
    if catalog.zodiac_sign_labels.is_empty() {
        catalog.zodiac_sign_labels = bootstrap_zodiac_sign_labels();
    }
    if catalog.evidence.chapter_slots.is_empty() {
        catalog.evidence = bootstrap_evidence_catalog();
    } else if catalog.evidence.chapter_exclusions.is_empty() {
        catalog.evidence.chapter_exclusions =
            crate::evidence_canonical::bootstrap_evidence_catalog().chapter_exclusions;
    }
    if catalog.writing_locales.is_empty() {
        catalog.writing_locales = bootstrap_writing_locales();
    }
    if catalog.astro_basis_roles.is_empty() {
        catalog.astro_basis_roles = bootstrap_astro_basis_roles();
    }
    if catalog.aspect_type_labels.is_empty() {
        catalog.aspect_type_labels = bootstrap_aspect_type_labels();
    }
    bootstrap_extra_object_sign_labels(&mut catalog.astro_object_labels, &mut catalog.zodiac_sign_labels);
    if catalog.element_balance_labels.is_empty() {
        catalog.element_balance_labels = bootstrap_element_balance_labels();
    }
    if catalog.modality_balance_labels.is_empty() {
        catalog.modality_balance_labels = bootstrap_modality_balance_labels();
    }
    if catalog.sect_labels.is_empty() {
        catalog.sect_labels = bootstrap_sect_labels();
    }
    if catalog.house_theme_labels.is_empty() {
        catalog.house_theme_labels = bootstrap_house_theme_labels();
    }
    if catalog.interpretation_profiles.is_empty() {
        catalog.interpretation_profiles = bootstrap_interpretation_profiles();
        apply_profile_evidence_to_catalog(catalog);
    }
}

fn apply_profile_evidence_to_catalog(catalog: &mut CanonicalCatalog) {
    for profile_code in ["natal_premium_plus", "natal_premium"] {
        if let Some(profile) = catalog.interpretation_profiles.get(profile_code) {
            if let Some(policy) = profile.to_premium_evidence_policy() {
                catalog.evidence.premium_policy = policy;
                return;
            }
        }
    }
}

async fn load_interpretation_profiles_from_db(
    pool: &sqlx::PgPool,
    catalog: &mut CanonicalCatalog,
) {
    let Ok(rows) = sqlx::query_as::<_, (String, String, String, serde_json::Value)>(
        "SELECT profile_code, product_code, schema_version, profile_json \
         FROM llm_interpretation_profiles WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    else {
        return;
    };

    for (profile_code, _product_code, _schema_version, profile_json) in rows {
        if let Ok(doc) = serde_json::from_value::<InterpretationProfileDocument>(profile_json) {
            if doc.profile_code != profile_code {
                tracing::warn!(
                    profile_code = %profile_code,
                    json_profile_code = %doc.profile_code,
                    "skipping interpretation profile: profile_code column mismatch"
                );
                continue;
            }
            let profile = InterpretationProfile::from_document(doc);
            if profile.validate().is_ok() {
                catalog
                    .interpretation_profiles
                    .insert(profile_code, profile);
            }
        }
    }
}

async fn load_i18n_from_db(pool: &sqlx::PgPool, catalog: &mut CanonicalCatalog) {
    if let Ok(rows) = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT locale_code, iso_639_1, display_name, prompt_instruction \
         FROM llm_writing_locales WHERE is_active = true ORDER BY locale_code",
    )
    .fetch_all(pool)
    .await
    {
        catalog.writing_locales = rows
            .into_iter()
            .map(|(locale_code, iso_639_1, display_name, prompt_instruction)| WritingLocale {
                locale_code,
                iso_639_1,
                display_name,
                prompt_instruction,
            })
            .collect();
    }

    if let Ok(rows) = sqlx::query_as::<_, (String,)>(
        "SELECT role_code FROM llm_astro_basis_roles WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        catalog.astro_basis_roles = rows.into_iter().map(|(c,)| c).collect();
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT aspect_code, locale, label FROM llm_aspect_type_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (aspect_code, locale, label) in rows {
            catalog
                .aspect_type_labels
                .insert((locale, aspect_code), label);
        }
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT element_code, locale, display_label, interpretive_label \
         FROM llm_element_balance_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (code, locale, display, interpretive) in rows {
            catalog.element_balance_labels.insert(
                (locale, code),
                I18nLabelPair {
                    display_label: display,
                    interpretive_label: interpretive,
                },
            );
        }
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT modality_code, locale, display_label, interpretive_label \
         FROM llm_modality_balance_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (code, locale, display, interpretive) in rows {
            catalog.modality_balance_labels.insert(
                (locale, code),
                I18nLabelPair {
                    display_label: display,
                    interpretive_label: interpretive,
                },
            );
        }
    }

    if let Ok(rows) = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT sect_code, locale, display_label, interpretive_label \
         FROM llm_sect_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (code, locale, display, interpretive) in rows {
            catalog.sect_labels.insert(
                (locale, code),
                I18nLabelPair {
                    display_label: display,
                    interpretive_label: interpretive,
                },
            );
        }
    }

    if let Ok(rows) = sqlx::query_as::<_, (i16, String, String, String)>(
        "SELECT house_number, locale, display_label, interpretive_label \
         FROM llm_house_theme_labels WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    {
        for (house, locale, display, interpretive) in rows {
            catalog.house_theme_labels.insert(
                (locale, house as u8),
                I18nLabelPair {
                    display_label: display,
                    interpretive_label: interpretive,
                },
            );
        }
    }
}

fn parse_reasoning_effort(raw: &str) -> astral_llm_domain::ReasoningEffort {
    match raw.trim().to_lowercase().as_str() {
        "minimal" => astral_llm_domain::ReasoningEffort::Minimal,
        "low" => astral_llm_domain::ReasoningEffort::Low,
        "medium" => astral_llm_domain::ReasoningEffort::Medium,
        "high" => astral_llm_domain::ReasoningEffort::High,
        _ => astral_llm_domain::ReasoningEffort::None,
    }
}

impl CanonicalCatalog {
    pub fn domains_or_fallback<'a>(&'a self, fallback: &'a [&str]) -> Vec<String> {
        if self.astrological_domains.is_empty() {
            fallback.iter().map(|d| d.to_string()).collect()
        } else {
            self.astrological_domains.clone()
        }
    }

    pub fn prompt_for_product(&self, product_code: &str) -> Option<&ProductPromptFamily> {
        self.product_prompt_families
            .iter()
            .find(|p| p.product_code == product_code)
    }

    pub fn patterns_for_type(&self, pattern_type: &str) -> Vec<&str> {
        self.safety_patterns
            .iter()
            .filter(|p| p.pattern_type == pattern_type)
            .map(|p| p.pattern.as_str())
            .collect()
    }

    pub fn product_policy(&self, product_code: &str) -> Option<&ProductGenerationPolicy> {
        self.product_generation_policies
            .iter()
            .find(|p| p.product_code == product_code)
    }

    pub fn object_label(&self, locale: &str, object_code: &str) -> Option<&str> {
        self.astro_object_labels
            .get(&(locale.to_string(), object_code.to_string()))
            .map(String::as_str)
    }

    pub fn sign_label(&self, locale: &str, sign_code: &str) -> Option<&str> {
        self.zodiac_sign_labels
            .get(&(locale.to_string(), sign_code.to_string()))
            .map(String::as_str)
    }

    pub fn element_balance_label(&self, locale: &str, element_code: &str) -> Option<&I18nLabelPair> {
        self.element_balance_labels
            .get(&(locale.to_string(), element_code.to_string()))
    }

    pub fn modality_balance_label(&self, locale: &str, modality_code: &str) -> Option<&I18nLabelPair> {
        self.modality_balance_labels
            .get(&(locale.to_string(), modality_code.to_string()))
    }

    pub fn sect_label(&self, locale: &str, sect_code: &str) -> Option<&I18nLabelPair> {
        self.sect_labels
            .get(&(locale.to_string(), sect_code.to_string()))
    }

    pub fn house_theme_label(&self, locale: &str, house_number: u8) -> Option<&I18nLabelPair> {
        self.house_theme_labels
            .get(&(locale.to_string(), house_number))
    }

    pub fn writing_locale(&self, user_language: &str) -> Option<&WritingLocale> {
        let code = user_language.trim().to_lowercase();
        self.writing_locales
            .iter()
            .find(|l| l.locale_code == code || l.iso_639_1 == code)
    }

    pub fn aspect_label(&self, locale: &str, aspect_code: &str) -> Option<&str> {
        self.aspect_type_labels
            .get(&(locale.to_string(), aspect_code.to_string()))
            .map(String::as_str)
    }

    pub fn is_allowed_basis_role(&self, role: &str) -> bool {
        self.astro_basis_roles.contains(role)
    }

    pub fn interpretation_profile(&self, profile_code: &str) -> Option<&InterpretationProfile> {
        self.interpretation_profiles.get(profile_code)
    }
}

fn insert_label(map: &mut HashMap<(String, String), String>, locale: &str, code: &str, label: &str) {
    map.insert((locale.into(), code.into()), label.into());
}

pub fn bootstrap_astro_object_labels() -> HashMap<(String, String), String> {
    let mut m = HashMap::new();
    for (code, fr, en) in [
        ("sun", "Soleil", "Sun"),
        ("moon", "Lune", "Moon"),
        ("mercury", "Mercure", "Mercury"),
        ("venus", "Vénus", "Venus"),
        ("mars", "Mars", "Mars"),
        ("jupiter", "Jupiter", "Jupiter"),
        ("saturn", "Saturne", "Saturn"),
        ("uranus", "Uranus", "Uranus"),
        ("neptune", "Neptune", "Neptune"),
        ("pluto", "Pluton", "Pluto"),
        ("ascendant", "Ascendant", "Ascendant"),
        ("descendant", "Descendant", "Descendant"),
        ("mc", "Milieu du Ciel", "Midheaven"),
        ("ic", "Fond du Ciel", "Imum Coeli"),
    ] {
        insert_label(&mut m, "fr", code, fr);
        insert_label(&mut m, "en", code, en);
    }
    m
}

pub fn bootstrap_zodiac_sign_labels() -> HashMap<(String, String), String> {
    let mut m = HashMap::new();
    for (code, fr, en) in [
        ("aries", "Bélier", "Aries"),
        ("taurus", "Taureau", "Taurus"),
        ("gemini", "Gémeaux", "Gemini"),
        ("cancer", "Cancer", "Cancer"),
        ("leo", "Lion", "Leo"),
        ("virgo", "Vierge", "Virgo"),
        ("libra", "Balance", "Libra"),
        ("scorpio", "Scorpion", "Scorpio"),
        ("sagittarius", "Sagittaire", "Sagittarius"),
        ("capricorn", "Capricorne", "Capricorn"),
        ("aquarius", "Verseau", "Aquarius"),
        ("pisces", "Poissons", "Pisces"),
    ] {
        insert_label(&mut m, "fr", code, fr);
        insert_label(&mut m, "en", code, en);
    }
    m
}

async fn attach_product_default_engine(pool: &sqlx::PgPool, catalog: &mut CanonicalCatalog) {
    let Ok(rows) = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT product_code, default_provider, default_model, economic_model \
         FROM llm_product_default_engine WHERE is_active = true",
    )
    .fetch_all(pool)
    .await
    else {
        return;
    };

    for (product_code, provider, model, economic) in rows {
        let Some(provider_kind) = parse_catalog_provider(&provider) else {
            continue;
        };
        let Some(policy) = catalog
            .product_generation_policies
            .iter_mut()
            .find(|p| p.product_code == product_code)
        else {
            continue;
        };
        policy.default_provider = Some(provider_kind);
        policy.default_model = Some(model);
        policy.economic_model = economic.filter(|m| !m.trim().is_empty());
    }
}

async fn attach_product_allowed_models(pool: &sqlx::PgPool, catalog: &mut CanonicalCatalog) {
    let Ok(rows) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT product_code, provider, model FROM llm_product_allowed_models WHERE is_active = true ORDER BY product_code, model",
    )
    .fetch_all(pool)
    .await
    else {
        return;
    };

    for (product_code, provider, model) in rows {
        let Some(provider_kind) = parse_catalog_provider(&provider) else {
            continue;
        };
        let Some(policy) = catalog
            .product_generation_policies
            .iter_mut()
            .find(|p| p.product_code == product_code)
        else {
            continue;
        };
        let reference = ProviderModelRef::new(provider_kind, model);
        if !policy
            .allowed_models
            .iter()
            .any(|m| m.provider == reference.provider && m.model.eq_ignore_ascii_case(&reference.model))
        {
            policy.allowed_models.push(reference);
        }
    }
}

fn parse_catalog_provider(raw: &str) -> Option<ProviderKind> {
    match raw.trim().to_lowercase().as_str() {
        "openai" | "open_ai" => Some(ProviderKind::OpenAi),
        "anthropic" => Some(ProviderKind::Anthropic),
        "mistral" => Some(ProviderKind::Mistral),
        "fake" => Some(ProviderKind::Fake),
        _ => None,
    }
}

pub fn bootstrap_product_policies() -> Vec<ProductGenerationPolicy> {
    vec![ProductGenerationPolicy::bootstrap_natal_prompter()]
}

pub fn bootstrap_interpretation_profiles() -> HashMap<String, InterpretationProfile> {
    let seeds = [
        include_str!("../../../../config/natal_interpretation_profiles/natal_light.json"),
        include_str!("../../../../config/natal_interpretation_profiles/natal_basic.json"),
        include_str!("../../../../config/natal_interpretation_profiles/natal_premium.json"),
        include_str!("../../../../config/natal_interpretation_profiles/natal_premium_plus.json"),
    ];
    let mut map = HashMap::new();
    for json in seeds {
        if let Ok(doc) = serde_json::from_str::<InterpretationProfileDocument>(json) {
            let code = doc.profile_code.clone();
            let profile = InterpretationProfile::from_document(doc);
            if profile.validate().is_ok() {
                map.insert(code, profile);
            }
        }
    }
    map
}

pub fn bootstrap_safety_patterns() -> Vec<SafetyPattern> {
    vec![
        SafetyPattern {
            pattern_type: "injection".into(),
            locale: "en".into(),
            pattern: "ignore previous".into(),
        },
        SafetyPattern {
            pattern_type: "injection".into(),
            locale: "fr".into(),
            pattern: "ignore les instructions".into(),
        },
        SafetyPattern {
            pattern_type: "medical".into(),
            locale: "fr".into(),
            pattern: "diagnostic medical".into(),
        },
        SafetyPattern {
            pattern_type: "medical".into(),
            locale: "en".into(),
            pattern: "medical diagnosis".into(),
        },
        SafetyPattern {
            pattern_type: "death".into(),
            locale: "fr".into(),
            pattern: "vous allez mourir".into(),
        },
        SafetyPattern {
            pattern_type: "deterministic".into(),
            locale: "fr".into(),
            pattern: "destin inevitable".into(),
        },
        SafetyPattern {
            pattern_type: "symbolic".into(),
            locale: "fr".into(),
            pattern: "symbolique".into(),
        },
        SafetyPattern {
            pattern_type: "symbolic".into(),
            locale: "en".into(),
            pattern: "interpretation".into(),
        },
    ]
}

/// Fallback statique minimal si la base n'est pas disponible (bootstrap dev uniquement).
pub fn bootstrap_domains() -> Vec<String> {
    vec![
        "identity".into(),
        "emotional_life".into(),
        "relationships".into(),
        "career".into(),
        "resources".into(),
        "family_roots".into(),
        "communication_mind".into(),
        "money".into(),
        "family".into(),
        "inner_conflicts".into(),
        "talents".into(),
        "growth_path".into(),
        "synthesis".into(),
    ]
}

pub fn service_limits_from_env() -> ServiceLimits {
    ServiceLimits {
        max_body_bytes: env_usize("ASTRAL_LLM_MAX_BODY_BYTES", 2 * 1024 * 1024),
        max_astro_json_bytes: env_usize("ASTRAL_LLM_MAX_ASTRO_JSON_BYTES", 512 * 1024),
        max_domain_count: env_u8("ASTRAL_LLM_MAX_DOMAIN_COUNT", 12),
        max_chapters_per_request: env_u8("ASTRAL_LLM_MAX_CHAPTERS", 12),
        default_request_timeout_ms: env_u64("ASTRAL_LLM_REQUEST_TIMEOUT_MS", 120_000),
        max_custom_instructions_chars: env_usize("ASTRAL_LLM_MAX_CUSTOM_INSTRUCTIONS_CHARS", 2_000),
    }
}

fn env_usize(key: &str, default: usize) -> usize {
    crate::config::env_var(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u8(key: &str, default: u8) -> u8 {
    crate::config::env_var(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    crate::config::env_var(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

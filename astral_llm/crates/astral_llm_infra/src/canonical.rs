use std::sync::Arc;

use astral_llm_domain::ServiceLimits;

/// Referentiel canonique charge depuis PostgreSQL (tables `llm_*`).
#[derive(Debug, Clone, Default)]
pub struct CanonicalCatalog {
    pub astrological_domains: Vec<String>,
    pub safety_patterns: Vec<SafetyPattern>,
    pub product_prompt_families: Vec<ProductPromptFamily>,
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

    catalog
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
}

/// Fallback statique minimal si la base n'est pas disponible (bootstrap dev uniquement).
pub fn bootstrap_domains() -> Vec<String> {
    vec![
        "identity".into(),
        "emotional_life".into(),
        "relationships".into(),
        "career".into(),
        "money".into(),
        "family".into(),
        "inner_conflicts".into(),
        "talents".into(),
        "growth_path".into(),
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

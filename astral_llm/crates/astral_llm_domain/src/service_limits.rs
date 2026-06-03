use serde::{Deserialize, Serialize};

/// Limites operationnelles du service (source canonique : `.env` / table `llm_service_limits`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLimits {
    pub max_body_bytes: usize,
    pub max_astro_json_bytes: usize,
    pub max_domain_count: u8,
    pub max_chapters_per_request: u8,
    pub default_request_timeout_ms: u64,
    pub max_custom_instructions_chars: usize,
}

impl Default for ServiceLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 2 * 1024 * 1024,
            max_astro_json_bytes: 512 * 1024,
            max_domain_count: 12,
            max_chapters_per_request: 12,
            default_request_timeout_ms: 120_000,
            max_custom_instructions_chars: 2_000,
        }
    }
}

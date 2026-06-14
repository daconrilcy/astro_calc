//! Tests de charge / saturation locaux (sans CI dediee).

use std::sync::Arc;

use astral_llm_api::rate_limit::{ApiKeyRateLimiter, RateLimitReason};
use astral_llm_application::ProviderCircuitBreaker;
use astral_llm_domain::ProviderKind;
use astral_llm_infra::{hash_json, AppConfig, IdempotencyClaim, RunPersistence};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[test]
fn global_semaphore_saturates() {
    let sem = Semaphore::new(2);
    let p1 = sem.try_acquire().expect("p1");
    let p2 = sem.try_acquire().expect("p2");
    assert!(sem.try_acquire().is_err());
    drop(p1);
    drop(p2);
    assert!(sem.try_acquire().is_ok());
}

#[test]
fn api_key_concurrent_limit_saturates() {
    let limiter = ApiKeyRateLimiter::new(2, 120, 4);
    let key = "key:test";
    let _a = limiter.try_acquire(key, false).expect("a");
    let _b = limiter.try_acquire(key, false).expect("b");
    assert!(matches!(
        limiter.try_acquire(key, false),
        Err(RateLimitReason::ConcurrentPerKey)
    ));
}

#[test]
fn api_key_rpm_limit_saturates() {
    let limiter = ApiKeyRateLimiter::new(8, 2, 4);
    let key = "key:rpm";
    let _a = limiter.try_acquire(key, false).expect("a");
    let _b = limiter.try_acquire(key, false).expect("b");
    assert!(matches!(
        limiter.try_acquire(key, false),
        Err(RateLimitReason::RequestsPerMinute)
    ));
}

#[test]
fn premium_concurrent_limit_saturates() {
    let limiter = ApiKeyRateLimiter::new(8, 120, 1);
    let key = "key:premium";
    let _a = limiter.reserve_premium_slot(key).expect("p1");
    assert!(matches!(
        limiter.reserve_premium_slot(key),
        Err(RateLimitReason::PremiumConcurrent)
    ));
}

#[test]
fn circuit_breaker_opens_after_transient_failures() {
    let cb = Arc::new(ProviderCircuitBreaker::new(3, 30));
    let provider = ProviderKind::OpenAi;
    for _ in 0..3 {
        cb.record_transient_failure(&provider);
    }
    assert!(!cb.allows_call(&provider));
}

#[tokio::test]
async fn circuit_breaker_parallel_checks_respect_open_state() {
    let cb = Arc::new(ProviderCircuitBreaker::new(2, 60));
    let provider = ProviderKind::Mistral;
    cb.record_transient_failure(&provider);
    cb.record_transient_failure(&provider);
    let cb_clone = cb.clone();
    let allowed = tokio::spawn(async move { cb_clone.allows_call(&provider) })
        .await
        .expect("join");
    assert!(!allowed);
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and applied migrations"]
async fn idempotency_concurrent_claim_single_winner() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_env = manifest.join("../../../.env");
    if repo_env.is_file() {
        dotenvy::from_path(&repo_env).ok();
    } else {
        dotenvy::dotenv().ok();
    }
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("db");
    let persistence = RunPersistence::new(pool);
    persistence.ensure_schema().await.expect("schema");

    let key = format!("load-test-{}", Uuid::new_v4());
    let product = "natal_prompter";
    let payload = serde_json::json!({ "probe": "idempotency-load" });
    let input_hash = hash_json(&payload);

    let run_a = Uuid::new_v4();
    let run_b = Uuid::new_v4();

    let (claim_a, claim_b) = tokio::join!(
        persistence.claim_idempotency(&key, product, run_a, &input_hash, 1),
        persistence.claim_idempotency(&key, product, run_b, &input_hash, 1),
    );

    let mut acquired = 0usize;
    let mut in_progress = 0usize;
    for claim in [claim_a.expect("claim a"), claim_b.expect("claim b")] {
        match claim {
            IdempotencyClaim::Acquired { .. } => acquired += 1,
            IdempotencyClaim::InProgress { .. } => in_progress += 1,
            _ => {}
        }
    }

    assert_eq!(acquired, 1, "exactly one concurrent claim should win");
    assert!(in_progress >= 1, "loser should observe in-progress state");

    persistence
        .delete_idempotency_record(&key, product)
        .await
        .expect("cleanup idempotency test row");
}

#[test]
fn production_public_config_requires_persistence() {
    let mut config = AppConfig {
        runtime_env: astral_llm_domain::AstralLlmEnv::Production,
        production_exposure: astral_llm_domain::ProductionExposureMode::Public,
        bind_addr: "0.0.0.0:8081".parse().unwrap(),
        allow_public_bind: true,
        database_url: None,
        prompts_dir: "astral_llm/prompts".into(),
        default_provider: ProviderKind::OpenAi,
        default_model: "gpt-4.1".into(),
        fallback_policy: astral_llm_domain::FallbackPolicy::disabled(),
        enable_fake_provider: false,
        enable_persistence: false,
        db_auto_migrate: false,
        store_sanitized_payloads: false,
        openai_base_url: "https://api.openai.com".into(),
        anthropic_base_url: "https://api.anthropic.com".into(),
        mistral_base_url: "https://api.mistral.ai".into(),
        api_key: Some("secret".into()),
        privacy_policy: astral_llm_domain::PrivacyPolicy::default(),
        limits: astral_llm_domain::ServiceLimits::default(),
        max_concurrent_requests: 32,
        max_concurrent_requests_per_key: 8,
        max_requests_per_minute_per_key: 120,
        max_premium_runs_per_key: 4,
        idempotency_ttl_hours: 24,
        circuit_breaker_failure_threshold: 5,
        circuit_breaker_open_secs: 60,
        enable_legacy_product_code_shim: true,
        legacy_product_code_shim_cutoff_date: None,
    };
    assert!(config.requires_strict_persistence());
    let mut secrets = astral_llm_infra::ProviderSecrets::default();
    secrets.openai_api_key = Some(secrecy::SecretString::from("k".to_string()));
    assert!(astral_llm_infra::ConfigValidator::validate(&config, &secrets).is_err());
    config.enable_persistence = true;
    config.database_url = Some("postgres://localhost/astral".into());
    assert!(astral_llm_infra::ConfigValidator::validate(&config, &secrets).is_ok());
}

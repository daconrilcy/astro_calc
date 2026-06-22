use std::sync::Arc;

use astral_llm_api::{
    auth::require_api_key,
    rate_limit::{api_key_rate_limit, concurrency_limit, new_api_key_limiter, new_semaphore},
    routes,
    state::AppState,
};

use astral_llm_application::{
    build_capability_registry_with_db, build_fallback_policy, build_providers,
    prompt_trace::{configure_prompt_trace, PromptTraceSettings},
    raw_provider_trace::{configure_raw_provider_trace, RawProviderTraceSettings},
    GenerateReadingUseCase, IntegrationJobValidator, PromptCompiler, ProviderCircuitBreaker,
    ProviderRouter, ResponseValidator, SchemaRegistry,
};
use astral_llm_infra::{
    bootstrap_domains, bootstrap_product_policies, bootstrap_safety_patterns,
    calculator_api_key_from_env, calculator_base_url_from_env, enrich_catalog_from_bootstrap,
    init_tracing, load_active_provider_codes, load_canonical_catalog, load_model_capabilities,
    prompt_trace_dir_from_env, prompt_trace_enabled_from_env, raw_provider_trace_dir_from_env,
    raw_provider_trace_enabled_from_env, AppConfig, CalculatorClient, CanonicalCatalog,
    ConfigValidator, ProviderSecrets, RunPersistence, SharedCanonicalCatalog,
};
use axum::http::StatusCode;
use axum::middleware;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};

#[tokio::main]
async fn main() {
    init_tracing();
    let config = AppConfig::try_from_env().unwrap_or_else(|err| {
        panic!("invalid astral_llm configuration: {err}");
    });
    let bind_addr = config.bind_addr;
    let secrets = ProviderSecrets::from_env();

    if let Err(err) = ConfigValidator::validate(&config, &secrets) {
        panic!("invalid astral_llm configuration: {err}");
    }
    configure_prompt_trace(PromptTraceSettings::from_runtime(
        prompt_trace_enabled_from_env(),
        prompt_trace_dir_from_env().map(Into::into),
    ));
    configure_raw_provider_trace(RawProviderTraceSettings::from_runtime(
        config.runtime_env,
        raw_provider_trace_enabled_from_env(config.runtime_env),
        raw_provider_trace_dir_from_env().map(Into::into),
    ));

    let engine_defaults = config.engine_defaults();
    let limits = config.limits.clone();
    let privacy_policy = config.privacy_policy.clone();

    let provider_map = build_providers(&config, &secrets).expect("LLM provider bootstrap failed");

    let mut bootstrap_catalog = CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        safety_patterns: bootstrap_safety_patterns(),
        product_generation_policies: bootstrap_product_policies(),
        ..Default::default()
    };
    enrich_catalog_from_bootstrap(&mut bootstrap_catalog);
    let mut catalog: SharedCanonicalCatalog = Arc::new(bootstrap_catalog);

    let mut db_models = Vec::new();
    let mut active_providers = Vec::new();
    let mut persistence = None;
    let mut job_persistence = None;
    if config.enable_persistence {
        if let Some(database_url) = &config.database_url {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await
                .expect("database connection");
            let run_persistence = RunPersistence::new(pool.clone());
            if config.db_auto_migrate {
                run_persistence.ensure_schema().await.expect("schema");
            } else {
                tracing::info!("db auto-migrate disabled; verifying schema");
                run_persistence
                    .verify_schema()
                    .await
                    .expect("expected PostgreSQL schema missing; apply SQL migrations before boot");
            }
            let mut loaded = load_canonical_catalog(&pool).await;
            enrich_catalog_from_bootstrap(&mut loaded);
            catalog = Arc::new(loaded);
            active_providers = load_active_provider_codes(&pool).await;
            db_models = load_model_capabilities(&pool).await;
            persistence = Some(Arc::new(run_persistence));
            job_persistence = Some(Arc::new(astral_llm_infra::JobPersistence::new(pool)));
        } else {
            tracing::warn!("persistence enabled but DATABASE_URL missing");
        }
    }
    let persistence = persistence;
    let job_persistence = job_persistence;

    let capability_registry = if db_models.is_empty() {
        astral_llm_application::build_capability_registry()
    } else {
        build_capability_registry_with_db(active_providers, db_models)
    };

    let circuit_breaker = Arc::new(ProviderCircuitBreaker::new(
        config.circuit_breaker_failure_threshold,
        config.circuit_breaker_open_secs,
    ));

    let router = ProviderRouter::new(
        provider_map,
        build_fallback_policy(&config),
        capability_registry,
        privacy_policy,
        circuit_breaker,
        persistence.clone(),
    );

    let schema_registry = Arc::new(SchemaRegistry::new());
    let compiler = PromptCompiler::new(&config.prompts_dir);
    let validator = ResponseValidator::new(schema_registry.clone());

    let use_case = Arc::new(GenerateReadingUseCase::new(
        router,
        compiler,
        validator,
        engine_defaults,
        limits.clone(),
        catalog.clone(),
        config.privacy_policy.clone(),
        config.legacy_product_code_shim_available(),
        persistence.clone(),
    ));

    tracing::info!(
        env = config.runtime_env.as_str(),
        default_provider = config.default_provider.as_str(),
        default_model = %config.default_model,
        fake = config.enable_fake_provider,
        auth = config.requires_auth(),
        persistence = persistence.is_some(),
        "astral_llm_api ready"
    );

    let calculator_client = CalculatorClient::new(
        calculator_base_url_from_env(),
        calculator_api_key_from_env(),
        config.limits.default_request_timeout_ms,
    )
    .ok();

    let integration_job_validator = Arc::new(IntegrationJobValidator::new());

    let state = AppState {
        use_case,
        schema_registry,
        config: config.clone(),
        persistence,
        job_persistence,
        integration_job_validator: Some(integration_job_validator),
        concurrency_limit: new_semaphore(config.max_concurrent_requests),
        api_key_limiter: new_api_key_limiter(&config),
        interpretation_profile_count: catalog.interpretation_profiles.len(),
        calculator_client,
    };

    let timeout = Duration::from_millis(state.config.limits.default_request_timeout_ms + 5_000);
    let body_limit = RequestBodyLimitLayer::new(state.config.limits.max_body_bytes);

    // Tower : derniere couche ajoutee = premiere sur la requete entrante.
    // Ordre voulu : trace -> auth -> rate limit par cle -> semaphore global -> handler.
    let app = routes::router(state.clone())
        .layer(body_limit)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            timeout,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            concurrency_limit,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api_key_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_key,
        ))
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(bind_addr).await.expect("bind");
    tracing::info!(addr = %bind_addr, "astral_llm_api listening");
    axum::serve(listener, app).await.expect("server");
}

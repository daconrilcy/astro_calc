mod auth;
mod routes;
mod state;

use std::sync::Arc;

use astral_llm_application::{
    build_fallback_policy, build_providers, GenerateReadingUseCase, PromptCompiler,
    ProviderRouter, ResponseValidator, SchemaRegistry,
};
use astral_llm_infra::{
    bootstrap_domains, init_tracing, load_canonical_catalog, AppConfig, CanonicalCatalog,
    ProviderSecrets, RunPersistence, SharedCanonicalCatalog,
};
use axum::middleware;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use axum::http::StatusCode;
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};
use std::time::Duration;

use crate::auth::require_api_key;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    init_tracing();
    let config = AppConfig::from_env();
    let bind_addr = config.bind_addr;
    let secrets = ProviderSecrets::from_env();
    let engine_defaults = config.engine_defaults();
    let limits = config.limits.clone();

    let provider_map = build_providers(&config, &secrets).expect("LLM provider bootstrap failed");
    let router = ProviderRouter::new(provider_map, build_fallback_policy(&config));

    let schema_registry = Arc::new(SchemaRegistry::new());
    let compiler = PromptCompiler::new(&config.prompts_dir);
    let validator = ResponseValidator::new(schema_registry.clone());

    let catalog: SharedCanonicalCatalog = Arc::new(CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        ..Default::default()
    });

    let mut catalog = catalog;
    let persistence = if config.enable_persistence {
        if let Some(database_url) = &config.database_url {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await
                .expect("database connection");
            let persistence = RunPersistence::new(pool.clone());
            persistence.ensure_schema().await.expect("schema");
            catalog = Arc::new(load_canonical_catalog(&pool).await);
            Some(Arc::new(persistence))
        } else {
            tracing::warn!("persistence enabled but DATABASE_URL missing");
            None
        }
    } else {
        None
    };

    let use_case = Arc::new(GenerateReadingUseCase::new(
        router,
        compiler,
        validator,
        engine_defaults,
        limits.clone(),
        catalog.clone(),
    ));

    tracing::info!(
        default_provider = config.default_provider.as_str(),
        default_model = %config.default_model,
        auth = config.requires_auth(),
        "astral_llm_api ready"
    );

    let state = AppState {
        use_case,
        schema_registry,
        config,
        persistence,
    };

    let timeout = Duration::from_millis(state.config.limits.default_request_timeout_ms + 5_000);
    let body_limit = RequestBodyLimitLayer::new(state.config.limits.max_body_bytes);

    let app = routes::router(state.clone())
        .layer(body_limit)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            timeout,
        ))
        .layer(middleware::from_fn_with_state(state, require_api_key))
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(bind_addr)
        .await
        .expect("bind");
    tracing::info!(addr = %bind_addr, "astral_llm_api listening");
    axum::serve(listener, app).await.expect("server");
}

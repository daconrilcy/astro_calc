use std::sync::Arc;
use std::time::Duration;

mod calculator_port;

use crate::calculator_port::WorkerCalculatorPort;
use astral_llm_application::{
    build_capability_registry_with_db, build_fallback_policy, build_providers,
    core::calculator::CalculatorPort,
    job_error_from_reading, job_status_from_reading,
    prompt_trace::{configure_prompt_trace, PromptTraceSettings},
    raw_provider_trace::{configure_raw_provider_trace, RawProviderTraceSettings},
    unified_result_envelope, GenerateReadingUseCase, IntegrationJobExecutor,
    IntegrationJobValidator, PromptCompiler, ProviderCircuitBreaker, ProviderRouter,
    ResponseValidator, SchemaRegistry, UnifiedReadingOutcome,
};
use astral_llm_domain::GenerateReadingResponse;
use astral_llm_infra::{
    bootstrap_domains, bootstrap_product_policies, bootstrap_safety_patterns,
    calculator_api_key_from_env, calculator_base_url_from_env, enrich_catalog_from_bootstrap,
    init_tracing, load_active_provider_codes, load_canonical_catalog, load_model_capabilities,
    prompt_trace_dir_from_env, prompt_trace_enabled_from_env, raw_provider_trace_dir_from_env,
    raw_provider_trace_enabled_from_env, AppConfig, CalculatorClient, CanonicalCatalog,
    ConfigValidator, JobPersistence, MercurePublisher, ProviderSecrets, RunPersistence,
    SharedCanonicalCatalog,
};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    run_worker().await;
}

async fn run_worker() {
    init_tracing();
    astral_llm_infra::load_dotenv();
    let config = AppConfig::try_from_env().unwrap_or_else(|err| {
        panic!("invalid astral_llm_worker configuration: {err}");
    });
    let secrets = ProviderSecrets::from_env();
    if let Err(err) = ConfigValidator::validate(&config, &secrets) {
        panic!("invalid astral_llm_worker configuration: {err}");
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

    let database_url = config
        .database_url
        .as_ref()
        .expect("DATABASE_URL required for astral_llm_worker");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(database_url)
        .await
        .expect("database connection");

    let run_persistence = Arc::new(RunPersistence::new(pool.clone()));
    if config.db_auto_migrate {
        run_persistence.ensure_schema().await.expect("schema");
    } else {
        run_persistence
            .verify_schema()
            .await
            .expect("schema verify");
    }
    let jobs = JobPersistence::new(pool.clone());

    let mut bootstrap_catalog = CanonicalCatalog {
        astrological_domains: bootstrap_domains(),
        safety_patterns: bootstrap_safety_patterns(),
        product_generation_policies: bootstrap_product_policies(),
        ..Default::default()
    };
    enrich_catalog_from_bootstrap(&mut bootstrap_catalog);
    let loaded = load_canonical_catalog(&pool).await;
    let mut enriched = loaded;
    enrich_catalog_from_bootstrap(&mut enriched);
    let catalog: SharedCanonicalCatalog = Arc::new(enriched);

    let active_providers = load_active_provider_codes(&pool).await;
    let db_models = load_model_capabilities(&pool).await;
    let capability_registry = if db_models.is_empty() {
        astral_llm_application::build_capability_registry()
    } else {
        build_capability_registry_with_db(active_providers, db_models)
    };

    let provider_map = build_providers(&config, &secrets).expect("LLM provider bootstrap failed");
    let router = ProviderRouter::new(
        provider_map,
        build_fallback_policy(&config),
        capability_registry,
        config.privacy_policy.clone(),
        Arc::new(ProviderCircuitBreaker::new(
            config.circuit_breaker_failure_threshold,
            config.circuit_breaker_open_secs,
        )),
        Some(run_persistence.clone()),
    );
    let schema_registry = Arc::new(SchemaRegistry::new());
    let use_case = GenerateReadingUseCase::new(
        router,
        PromptCompiler::new(&config.prompts_dir),
        ResponseValidator::new(schema_registry),
        config.engine_defaults(),
        config.limits.clone(),
        catalog,
        config.privacy_policy.clone(),
        config.legacy_product_code_shim_available(),
        Some(run_persistence.clone()),
    );

    let calculator = WorkerCalculatorPort::new(
        CalculatorClient::new(
            calculator_base_url_from_env(),
            calculator_api_key_from_env(),
            config.limits.default_request_timeout_ms,
        )
        .expect("calculator client"),
    );

    let orchestrator = IntegrationJobExecutor::new(&calculator, &use_case);
    let validator = IntegrationJobValidator::new();
    let mercure = MercurePublisher::from_env();

    let poll_ms = worker_poll_ms();
    let stale_secs = worker_stale_secs();
    let worker_id = format!("worker-{}", Uuid::new_v4());
    tracing::info!(worker_id = %worker_id, poll_ms, stale_secs, "astral_llm_worker started");

    loop {
        process_worker_tick(
            &jobs,
            &worker_id,
            stale_secs,
            poll_ms,
            &use_case,
            &validator,
            &orchestrator,
            &mercure,
        )
        .await;
    }
}

async fn process_worker_tick<C>(
    jobs: &astral_llm_infra::JobPersistence,
    worker_id: &str,
    stale_secs: i64,
    poll_ms: u64,
    use_case: &GenerateReadingUseCase,
    validator: &IntegrationJobValidator,
    orchestrator: &IntegrationJobExecutor<'_, C>,
    mercure: &MercurePublisher,
) where
    C: CalculatorPort + ?Sized,
{
    match jobs.purge_expired_terminal_jobs().await {
        Ok(count) if count > 0 => {
            tracing::info!(purged = count, "expired terminal integration jobs purged");
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "expired job purge failed"),
    }

    if let Err(err) = jobs.recover_stale_running_jobs().await {
        tracing::warn!(error = %err, "stale job recovery failed");
    }

    let job = match jobs.claim_next_queued_job(worker_id, stale_secs).await {
        Ok(Some(j)) => j,
        Ok(None) => {
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            return;
        }
        Err(err) => {
            tracing::error!(error = %err, "claim job failed");
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            return;
        }
    };

    let service = match use_case.catalog().integration_service(&job.service_code) {
        Some(s) => s.clone(),
        None => {
            let _ = jobs
                .mark_failed(
                    job.job_id,
                    &serde_json::json!({
                        "code": "SERVICE_NOT_FOUND",
                        "message": format!("unknown service: {}", job.service_code),
                    }),
                    false,
                )
                .await;
            return;
        }
    };

    let validated = match validator.validate_job(&job.request_json, &service) {
        Ok(v) => v,
        Err(err) => {
            let detail = err.detail();
            let _ = jobs
                .mark_failed(
                    job.job_id,
                    &serde_json::json!({
                        "code": detail.code.as_str(),
                        "message": detail.message,
                    }),
                    false,
                )
                .await;
            return;
        }
    };

    let _ = jobs.touch_heartbeat(job.job_id, stale_secs).await;
    let public_run_id = job.run_id.to_string();
    let outcome = orchestrator
        .execute(&service, &validated, Some(&public_run_id))
        .await;
    match outcome {
        Ok(result) => handle_job_result(jobs, mercure, job, service, result).await,
        Err(err) => handle_job_error(jobs, mercure, job, service, err).await,
    }
}

async fn handle_job_result(
    jobs: &astral_llm_infra::JobPersistence,
    mercure: &MercurePublisher,
    job: astral_llm_infra::JobRecord,
    service: astral_llm_domain::integration::IntegrationService,
    result: astral_llm_application::UnifiedReadingResult,
) {
    let gen_run_id = Uuid::parse_str(&result.run_id).ok();
    let (calculation, reading, reading_completeness) = match result.outcome {
        UnifiedReadingOutcome::Reading {
            calculation,
            reading,
            reading_completeness,
        } => (calculation, reading, reading_completeness),
        UnifiedReadingOutcome::Json(envelope) => {
            if let Err(err) = jobs.mark_completed(job.job_id, gen_run_id, &envelope).await {
                tracing::error!(
                    run_id = %job.run_id,
                    service = %job.service_code,
                    error = %err,
                    "job completion persistence failed"
                );
                return;
            }
            publish_mercure_if_enabled(mercure, &service, &job.tenant_id, &job.run_id, "completed")
                .await;
            tracing::info!(
                run_id = %job.run_id,
                service = %job.service_code,
                status = "completed",
                "job finished"
            );
            return;
        }
    };
    let envelope = unified_result_envelope(calculation, &reading, reading_completeness);
    let status = job_status_from_reading(&reading);
    match &reading {
        GenerateReadingResponse::Success { .. } => {
            if let Err(err) = jobs.mark_completed(job.job_id, gen_run_id, &envelope).await {
                tracing::error!(
                    run_id = %job.run_id,
                    service = %job.service_code,
                    error = %err,
                    "job completion persistence failed"
                );
                return;
            }
            publish_mercure_if_enabled(
                mercure,
                &service,
                &job.tenant_id,
                &job.run_id,
                status.as_str(),
            )
            .await;
        }
        GenerateReadingResponse::SafetyRejected { .. } => {
            let err = job_error_from_reading(&reading);
            if let Err(persist_err) = jobs
                .mark_safety_rejected(job.job_id, gen_run_id, &envelope, &err)
                .await
            {
                tracing::error!(
                    run_id = %job.run_id,
                    service = %job.service_code,
                    error = %persist_err,
                    "job safety rejection persistence failed"
                );
                return;
            }
            publish_mercure_if_enabled(
                mercure,
                &service,
                &job.tenant_id,
                &job.run_id,
                status.as_str(),
            )
            .await;
        }
        GenerateReadingResponse::Failed { .. } => {
            let err = job_error_from_reading(&reading);
            let retry = job.attempt_count < job.max_attempts;
            let _ = jobs.mark_failed(job.job_id, &err, retry).await;
            if !retry {
                publish_mercure_if_enabled(
                    mercure,
                    &service,
                    &job.tenant_id,
                    &job.run_id,
                    "failed",
                )
                .await;
            }
        }
    }
    tracing::info!(
        run_id = %job.run_id,
        service = %job.service_code,
        status = %status.as_str(),
        "job finished"
    );
}

async fn handle_job_error(
    jobs: &astral_llm_infra::JobPersistence,
    mercure: &MercurePublisher,
    job: astral_llm_infra::JobRecord,
    service: astral_llm_domain::integration::IntegrationService,
    err: astral_llm_domain::GenerationError,
) {
    let detail = err.detail();
    let retry = job.attempt_count < job.max_attempts;
    let _ = jobs
        .mark_failed(
            job.job_id,
            &serde_json::json!({
                "code": detail.code.as_str(),
                "message": detail.message,
            }),
            retry,
        )
        .await;
    if !retry {
        publish_mercure_if_enabled(mercure, &service, &job.tenant_id, &job.run_id, "failed").await;
    }
    tracing::warn!(
        run_id = %job.run_id,
        service = %job.service_code,
        error = %detail.message,
        details = ?detail.details,
        "job orchestration failed"
    );
}

fn worker_poll_ms() -> u64 {
    std::env::var("ASTRAL_LLM_WORKER_POLL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000)
}

fn worker_stale_secs() -> i64 {
    std::env::var("ASTRAL_LLM_WORKER_STALE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900)
}

async fn publish_mercure_if_enabled(
    mercure: &MercurePublisher,
    service: &astral_llm_domain::integration::IntegrationService,
    tenant_id: &str,
    run_id: &Uuid,
    status: &str,
) {
    if !service.supports_mercure || !mercure.is_enabled() {
        return;
    }
    mercure
        .publish_job_status(tenant_id, &run_id.to_string(), status)
        .await;
}

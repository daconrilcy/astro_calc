use std::sync::Arc;

use astral_llm_domain::chapter_orchestration::GenerationStepRecord;
use async_trait::async_trait;
use uuid::Uuid;

use astral_llm_infra::{
    GenerationPromptTraceRecord, GenerationRunRecord, GenerationTokenUsageRecord,
    NatalExplanationCacheKey, NatalExplanationCacheRecord, RunPersistence, RunStatus, SafetyStatus,
};

use super::{
    ExplanationCacheKeyRecord, ExplanationCacheRecord, PersistedGenerationRunRecord,
    PersistedPromptTraceRecord, PersistedRunStatus, PersistedSafetyStatus,
    PersistedTokenUsageRecord, ReadingPersistence, ReadingPersistenceError,
    SharedReadingPersistence,
};

impl PersistedRunStatus {
    fn into_infra(self) -> RunStatus {
        match self {
            Self::Success => RunStatus::Success,
            Self::Failed => RunStatus::Failed,
            Self::SafetyRejected => RunStatus::SafetyRejected,
            Self::Pending => RunStatus::Pending,
        }
    }
}

impl PersistedSafetyStatus {
    fn into_infra(self) -> SafetyStatus {
        match self {
            Self::Passed => SafetyStatus::Passed,
            Self::Rejected => SafetyStatus::Rejected,
            Self::NotChecked => SafetyStatus::NotChecked,
        }
    }
}

pub fn shared_reading_persistence(persistence: Arc<RunPersistence>) -> SharedReadingPersistence {
    Arc::new(InfraReadingPersistence { persistence })
}

struct InfraReadingPersistence {
    persistence: Arc<RunPersistence>,
}

#[async_trait]
impl ReadingPersistence for InfraReadingPersistence {
    async fn upsert_run(
        &self,
        record: &PersistedGenerationRunRecord,
    ) -> Result<(), ReadingPersistenceError> {
        let record = GenerationRunRecord {
            id: record.id,
            request_id: record.request_id.clone(),
            idempotency_key: record.idempotency_key.clone(),
            product_code: record.product_code.clone(),
            user_language: record.user_language.clone(),
            astro_contract_version: record.astro_contract_version.clone(),
            output_schema_version: record.output_schema_version.clone(),
            prompt_family: record.prompt_family.clone(),
            prompt_version: record.prompt_version.clone(),
            safety_policy_version: record.safety_policy_version.clone(),
            provider_requested: record.provider_requested.clone(),
            provider_used: record.provider_used.clone(),
            model_requested: record.model_requested.clone(),
            model_used: record.model_used.clone(),
            generation_mode: record.generation_mode.clone(),
            fallback_used: record.fallback_used,
            selected_domains: record.selected_domains.clone(),
            status: record.status.into_infra(),
            safety_status: record.safety_status.into_infra(),
            input_hash: record.input_hash.clone(),
            output_hash: record.output_hash.clone(),
            token_input: record.token_input,
            token_output: record.token_output,
            latency_ms: record.latency_ms,
            error_code: record.error_code.clone(),
            created_at: record.created_at,
        };
        self.persistence
            .upsert_run(&record)
            .await
            .map_err(|err| ReadingPersistenceError::from_source("upsert_run", &err))
    }

    async fn insert_prompt_trace(
        &self,
        record: &PersistedPromptTraceRecord,
    ) -> Result<(), ReadingPersistenceError> {
        let record = GenerationPromptTraceRecord {
            run_id: record.run_id,
            chapter_code: record.chapter_code.clone(),
            step_type: record.step_type.clone(),
            attempt: record.attempt.clone(),
            prompt_family: record.prompt_family.clone(),
            prompt_version: record.prompt_version.clone(),
            message_count: record.message_count,
            compiled_prompt: record.compiled_prompt.clone(),
            messages_json: record.messages_json.clone(),
        };
        self.persistence
            .insert_prompt_trace(&record)
            .await
            .map_err(|err| ReadingPersistenceError::from_source("insert_prompt_trace", &err))
    }

    async fn insert_steps(
        &self,
        run_id: Uuid,
        steps: &[GenerationStepRecord],
    ) -> Result<Vec<Uuid>, ReadingPersistenceError> {
        self.persistence
            .insert_steps(run_id, steps)
            .await
            .map_err(|err| ReadingPersistenceError::from_source("insert_steps", &err))
    }

    async fn replace_run_token_usages(
        &self,
        run_id: Uuid,
        usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError> {
        let usage_records = usage_records
            .iter()
            .map(|record| GenerationTokenUsageRecord {
                usage_type_code: record.usage_type_code.clone(),
                usage_subtype: record.usage_subtype.clone(),
                token_count: record.token_count,
                unit_price_usd_per_mtok: record.unit_price_usd_per_mtok,
                estimated_cost_usd: record.estimated_cost_usd,
                provider_metric_name: record.provider_metric_name.clone(),
            })
            .collect::<Vec<_>>();
        self.persistence
            .replace_run_token_usages(run_id, &usage_records)
            .await
            .map_err(|err| ReadingPersistenceError::from_source("replace_run_token_usages", &err))
    }

    async fn replace_step_token_usages(
        &self,
        step_id: Uuid,
        usage_records: &[PersistedTokenUsageRecord],
    ) -> Result<(), ReadingPersistenceError> {
        let usage_records = usage_records
            .iter()
            .map(|record| GenerationTokenUsageRecord {
                usage_type_code: record.usage_type_code.clone(),
                usage_subtype: record.usage_subtype.clone(),
                token_count: record.token_count,
                unit_price_usd_per_mtok: record.unit_price_usd_per_mtok,
                estimated_cost_usd: record.estimated_cost_usd,
                provider_metric_name: record.provider_metric_name.clone(),
            })
            .collect::<Vec<_>>();
        self.persistence
            .replace_step_token_usages(step_id, &usage_records)
            .await
            .map_err(|err| ReadingPersistenceError::from_source("replace_step_token_usages", &err))
    }

    async fn lookup_natal_explanations(
        &self,
        keys: &[ExplanationCacheKeyRecord],
    ) -> Result<Vec<ExplanationCacheRecord>, ReadingPersistenceError> {
        let keys = keys
            .iter()
            .map(|key| NatalExplanationCacheKey {
                language_code: key.language_code.clone(),
                key_hash: key.key_hash.clone(),
            })
            .collect::<Vec<_>>();
        self.persistence
            .lookup_natal_explanations(&keys)
            .await
            .map(|records| {
                records
                    .into_iter()
                    .map(|record| ExplanationCacheRecord {
                        language_code: record.language_code,
                        kind_code: record.kind_code,
                        key_hash: record.key_hash,
                        key_json: record.key_json,
                        title: record.title,
                        explanation: record.explanation,
                        expression_primary: record.expression_primary,
                        provider: record.provider,
                        model: record.model,
                        prompt_version: record.prompt_version,
                    })
                    .collect()
            })
            .map_err(|err| ReadingPersistenceError::from_source("lookup_natal_explanations", &err))
    }

    async fn upsert_natal_explanations(
        &self,
        records: &[ExplanationCacheRecord],
    ) -> Result<(), ReadingPersistenceError> {
        let records = records
            .iter()
            .map(|record| NatalExplanationCacheRecord {
                language_code: record.language_code.clone(),
                kind_code: record.kind_code.clone(),
                key_hash: record.key_hash.clone(),
                key_json: record.key_json.clone(),
                title: record.title.clone(),
                explanation: record.explanation.clone(),
                expression_primary: record.expression_primary.clone(),
                provider: record.provider.clone(),
                model: record.model.clone(),
                prompt_version: record.prompt_version.clone(),
            })
            .collect::<Vec<_>>();
        self.persistence
            .upsert_natal_explanations(&records)
            .await
            .map_err(|err| ReadingPersistenceError::from_source("upsert_natal_explanations", &err))
    }
}

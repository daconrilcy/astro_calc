//! Catalogue canonique des moteurs LLM et de leurs modeles (PostgreSQL).

use astral_llm_domain::{
    model_capability::{ModelCapability, StructuredOutputAdapterKind},
    model_usage_tier::ModelUsageTierPolicy,
    provider::{ProviderKind, ReasoningEffort, StructuredOutputMode},
};

const MODEL_SELECT: &str = "SELECT m.id, m.provider, m.model, m.catalog_notes, \
    m.supports_json_schema_strict, m.supports_json_object, \
    m.supports_reasoning_effort, m.supports_streaming, \
    m.max_input_tokens, m.max_output_tokens, m.structured_output_adapter, \
    m.storage_disable_supported, m.is_active, m.supports_temperature, m.reasoning_output_reserve_min, \
    m.reasoning_effort_subtask, m.reasoning_effort_primary, m.reasoning_effort_oracle, \
    m.usage_tier_code, \
    COALESCE(t.allows_primary_reading, true) AS allows_primary_reading, \
    COALESCE(t.allows_subtask, true) AS allows_subtask, \
    COALESCE(t.allows_oracle_benchmark, false) AS allows_oracle_benchmark";

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LlmProviderRow {
    pub id: i32,
    pub provider_code: String,
    pub label_fr: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LlmProviderModelRow {
    pub id: i32,
    pub provider: String,
    pub model: String,
    pub catalog_notes: Option<String>,
    pub supports_json_schema_strict: bool,
    pub supports_json_object: bool,
    pub supports_reasoning_effort: bool,
    pub supports_streaming: bool,
    pub max_input_tokens: i32,
    pub max_output_tokens: i32,
    pub structured_output_adapter: String,
    pub storage_disable_supported: bool,
    pub is_active: bool,
    pub supports_temperature: bool,
    pub reasoning_output_reserve_min: Option<i32>,
    pub reasoning_effort_subtask: Option<String>,
    pub reasoning_effort_primary: Option<String>,
    pub reasoning_effort_oracle: Option<String>,
    pub usage_tier_code: Option<String>,
    pub allows_primary_reading: bool,
    pub allows_subtask: bool,
    pub allows_oracle_benchmark: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UpsertProviderModelInput {
    pub supports_json_schema_strict: bool,
    pub supports_json_object: bool,
    pub supports_reasoning_effort: bool,
    pub supports_streaming: bool,
    pub max_input_tokens: i32,
    pub max_output_tokens: i32,
    pub structured_output_adapter: String,
    pub storage_disable_supported: bool,
    pub is_active: bool,
    pub catalog_notes: Option<String>,
    pub usage_tier_code: Option<String>,
}

pub struct ProviderCatalogRepository {
    pool: sqlx::PgPool,
}

impl ProviderCatalogRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub async fn list_providers(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<LlmProviderRow>, sqlx::Error> {
        if include_inactive {
            sqlx::query_as(
                "SELECT id, provider_code, label_fr, sort_order, is_active \
                 FROM llm_providers ORDER BY sort_order, provider_code",
            )
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT id, provider_code, label_fr, sort_order, is_active \
                 FROM llm_providers WHERE is_active = true ORDER BY sort_order, provider_code",
            )
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn add_provider(
        &self,
        provider_code: &str,
        label_fr: Option<&str>,
        sort_order: i32,
    ) -> Result<LlmProviderRow, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO llm_providers (provider_code, label_fr, sort_order) \
             VALUES ($1, $2, $3) \
             RETURNING id, provider_code, label_fr, sort_order, is_active",
        )
        .bind(provider_code.trim().to_lowercase())
        .bind(label_fr)
        .bind(sort_order)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn set_provider_active(
        &self,
        provider_code: &str,
        is_active: bool,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE llm_providers SET is_active = $2, updated_at = NOW() \
             WHERE provider_code = $1",
        )
        .bind(provider_code.trim().to_lowercase())
        .bind(is_active)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_provider(&self, provider_code: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM llm_providers WHERE provider_code = $1")
            .bind(provider_code.trim().to_lowercase())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_models(
        &self,
        provider_code: Option<&str>,
        include_inactive: bool,
    ) -> Result<Vec<LlmProviderModelRow>, sqlx::Error> {
        match (provider_code, include_inactive) {
            (Some(code), false) => {
                let sql = format!(
                    "{MODEL_SELECT} FROM llm_provider_models m \
                     INNER JOIN llm_providers p ON p.id = m.provider_id \
                     LEFT JOIN llm_model_usage_tiers t ON t.tier_code = m.usage_tier_code \
                     WHERE p.provider_code = $1 AND m.is_active = true AND p.is_active = true \
                     ORDER BY m.model"
                );
                sqlx::query_as(&sql)
                    .bind(code.trim().to_lowercase())
                    .fetch_all(&self.pool)
                    .await
            }
            (Some(code), true) => {
                let sql = format!(
                    "{MODEL_SELECT} FROM llm_provider_models m \
                     INNER JOIN llm_providers p ON p.id = m.provider_id \
                     LEFT JOIN llm_model_usage_tiers t ON t.tier_code = m.usage_tier_code \
                     WHERE p.provider_code = $1 ORDER BY m.model"
                );
                sqlx::query_as(&sql)
                    .bind(code.trim().to_lowercase())
                    .fetch_all(&self.pool)
                    .await
            }
            (None, false) => {
                let sql = format!(
                    "{MODEL_SELECT} FROM llm_provider_models m \
                     INNER JOIN llm_providers p ON p.id = m.provider_id \
                     LEFT JOIN llm_model_usage_tiers t ON t.tier_code = m.usage_tier_code \
                     WHERE m.is_active = true AND p.is_active = true \
                     ORDER BY m.provider, m.model"
                );
                sqlx::query_as(&sql).fetch_all(&self.pool).await
            }
            (None, true) => {
                let sql = format!(
                    "{MODEL_SELECT} FROM llm_provider_models m \
                     LEFT JOIN llm_model_usage_tiers t ON t.tier_code = m.usage_tier_code \
                     ORDER BY m.provider, m.model"
                );
                sqlx::query_as(&sql).fetch_all(&self.pool).await
            }
        }
    }

    pub async fn add_model(
        &self,
        provider_code: &str,
        model: &str,
        input: &UpsertProviderModelInput,
    ) -> Result<LlmProviderModelRow, sqlx::Error> {
        let provider_id: i32 =
            sqlx::query_scalar("SELECT id FROM llm_providers WHERE provider_code = $1")
                .bind(provider_code.trim().to_lowercase())
                .fetch_one(&self.pool)
                .await?;

        let sql = format!(
            "INSERT INTO llm_provider_models ( \
                provider, provider_id, model, catalog_notes, usage_tier_code, \
                supports_json_schema_strict, supports_json_object, supports_reasoning_effort, \
                supports_streaming, max_input_tokens, max_output_tokens, \
                structured_output_adapter, storage_disable_supported, is_active \
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             RETURNING id, provider, model, catalog_notes, \
             supports_json_schema_strict, supports_json_object, supports_reasoning_effort, \
             supports_streaming, max_input_tokens, max_output_tokens, structured_output_adapter, \
             storage_disable_supported, is_active, usage_tier_code, \
             true AS allows_primary_reading, true AS allows_subtask, false AS allows_oracle_benchmark"
        );
        sqlx::query_as(&sql)
            .bind(provider_code.trim().to_lowercase())
            .bind(provider_id)
            .bind(model.trim())
            .bind(&input.catalog_notes)
            .bind(&input.usage_tier_code)
            .bind(input.supports_json_schema_strict)
            .bind(input.supports_json_object)
            .bind(input.supports_reasoning_effort)
            .bind(input.supports_streaming)
            .bind(input.max_input_tokens)
            .bind(input.max_output_tokens)
            .bind(&input.structured_output_adapter)
            .bind(input.storage_disable_supported)
            .bind(input.is_active)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn set_model_active(
        &self,
        provider_code: &str,
        model: &str,
        is_active: bool,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE llm_provider_models SET is_active = $3, updated_at = NOW() \
             WHERE provider = $1 AND model = $2",
        )
        .bind(provider_code.trim().to_lowercase())
        .bind(model.trim())
        .bind(is_active)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn set_model_usage_tier(
        &self,
        provider_code: &str,
        model: &str,
        usage_tier_code: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE llm_provider_models SET usage_tier_code = $3, updated_at = NOW() \
             WHERE provider = $1 AND model = $2",
        )
        .bind(provider_code.trim().to_lowercase())
        .bind(model.trim())
        .bind(usage_tier_code.trim())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_model(
        &self,
        provider_code: &str,
        model: &str,
    ) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM llm_provider_models WHERE provider = $1 AND model = $2")
                .bind(provider_code.trim().to_lowercase())
                .bind(model.trim())
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }
}

pub async fn load_active_provider_codes(pool: &sqlx::PgPool) -> Vec<String> {
    let rows: Result<Vec<(String,)>, _> = sqlx::query_as(
        "SELECT provider_code FROM llm_providers WHERE is_active = true ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await;

    rows.unwrap_or_default().into_iter().map(|r| r.0).collect()
}

pub fn row_to_capability(row: &LlmProviderModelRow) -> Option<ModelCapability> {
    let provider = parse_provider(&row.provider)?;
    let native_safety = row.provider.eq_ignore_ascii_case("anthropic");
    let tier_policy = if row.usage_tier_code.is_some() {
        ModelUsageTierPolicy {
            allows_primary_reading: row.allows_primary_reading,
            allows_subtask: row.allows_subtask,
            allows_oracle_benchmark: row.allows_oracle_benchmark,
        }
    } else {
        ModelUsageTierPolicy::unrestricted()
    };
    Some(ModelCapability {
        provider,
        model: row.model.clone(),
        supports_json_schema_strict: row.supports_json_schema_strict,
        supports_json_object: row.supports_json_object,
        supports_reasoning_effort: row.supports_reasoning_effort,
        supports_streaming: row.supports_streaming,
        supports_native_safety_prompt: native_safety,
        max_input_tokens: row.max_input_tokens as u32,
        max_output_tokens: row.max_output_tokens as u32,
        structured_output_mode: if row.supports_json_schema_strict {
            StructuredOutputMode::JsonSchemaStrict
        } else {
            StructuredOutputMode::JsonObjectOnly
        },
        structured_output_adapter: parse_adapter(&row.structured_output_adapter),
        storage_disable_supported: row.storage_disable_supported,
        is_active: row.is_active,
        supports_temperature: row.supports_temperature,
        reasoning_output_reserve_min: row
            .reasoning_output_reserve_min
            .filter(|&n| n > 0)
            .map(|n| n as u32),
        reasoning_effort_subtask: row
            .reasoning_effort_subtask
            .as_deref()
            .and_then(ReasoningEffort::parse_api_value),
        reasoning_effort_primary: row
            .reasoning_effort_primary
            .as_deref()
            .and_then(ReasoningEffort::parse_api_value),
        reasoning_effort_oracle: row
            .reasoning_effort_oracle
            .as_deref()
            .and_then(ReasoningEffort::parse_api_value),
        usage_tier_code: row.usage_tier_code.clone(),
        tier_policy,
    })
}

fn parse_provider(raw: &str) -> Option<ProviderKind> {
    match raw.trim().to_lowercase().as_str() {
        "openai" => Some(ProviderKind::OpenAi),
        "anthropic" => Some(ProviderKind::Anthropic),
        "mistral" => Some(ProviderKind::Mistral),
        "fake" => Some(ProviderKind::Fake),
        _ => None,
    }
}

fn parse_adapter(raw: &str) -> StructuredOutputAdapterKind {
    match raw.trim().to_lowercase().as_str() {
        "anthropic_output_config_format" => {
            StructuredOutputAdapterKind::AnthropicOutputConfigFormat
        }
        "mistral_response_format_json_schema" => {
            StructuredOutputAdapterKind::MistralResponseFormatJsonSchema
        }
        "mistral_response_format_json_object" => {
            StructuredOutputAdapterKind::MistralResponseFormatJsonObject
        }
        "openai_responses_text_format" => StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
        _ => StructuredOutputAdapterKind::PromptOnly,
    }
}

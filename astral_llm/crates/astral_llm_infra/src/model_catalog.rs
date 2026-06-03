use astral_llm_domain::{
    model_capability::{ModelCapability, StructuredOutputAdapterKind},
    provider::{ProviderKind, StructuredOutputMode},
};

pub async fn load_model_capabilities(pool: &sqlx::PgPool) -> Vec<ModelCapability> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            bool,
            bool,
            bool,
            bool,
            i32,
            i32,
            String,
            bool,
        ),
    >(
        "SELECT provider, model, supports_json_schema_strict, supports_json_object, \
         supports_reasoning_effort, supports_streaming, max_input_tokens, max_output_tokens, \
         structured_output_adapter, storage_disable_supported \
         FROM llm_provider_models WHERE is_active = true",
    )
    .fetch_all(pool)
    .await;

    let Ok(rows) = rows else {
        return Vec::new();
    };

    rows.into_iter()
        .filter_map(|row| {
            let provider = parse_provider(&row.0)?;
            Some(ModelCapability {
                provider,
                model: row.1,
                supports_json_schema_strict: row.2,
                supports_json_object: row.3,
                supports_reasoning_effort: row.4,
                supports_streaming: row.5,
                supports_native_safety_prompt: false,
                max_input_tokens: row.6 as u32,
                max_output_tokens: row.7 as u32,
                structured_output_mode: if row.2 {
                    StructuredOutputMode::JsonSchemaStrict
                } else {
                    StructuredOutputMode::JsonObjectOnly
                },
                structured_output_adapter: parse_adapter(&row.8),
                storage_disable_supported: row.9,
                is_active: true,
            })
        })
        .collect()
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
        "anthropic_output_config_format" => StructuredOutputAdapterKind::AnthropicOutputConfigFormat,
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

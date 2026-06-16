use astral_llm_domain::{
    ChapterGenerationStatus, GenerationStepRecord, TokenUsage, TokenUsageType,
};

#[derive(Debug, Clone, Default)]
pub struct ExecutionAudit {
    pub run_id: String,
    pub selected_domains: Vec<String>,
    pub steps: Vec<GenerationStepRecord>,
    pub idempotency_key: Option<String>,
}

impl ExecutionAudit {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            ..Default::default()
        }
    }

    pub fn push_step(&mut self, step: GenerationStepRecord) {
        self.steps.push(step);
    }

    pub fn record_chapter_step(
        &mut self,
        chapter_code: &str,
        provider: &str,
        model: &str,
        status: ChapterGenerationStatus,
        token_usage: Option<TokenUsage>,
        latency_ms: u64,
        error_code: Option<String>,
        step_type: Option<&str>,
    ) {
        let input_tokens = token_usage
            .as_ref()
            .and_then(|usage| usage.tokens_for(TokenUsageType::Input));
        let output_tokens = token_usage
            .as_ref()
            .and_then(|usage| usage.tokens_for(TokenUsageType::Output));
        self.push_step(GenerationStepRecord {
            step_type: step_type.unwrap_or("chapter_generate").into(),
            chapter_code: Some(chapter_code.to_string()),
            provider: provider.to_string(),
            model: model.to_string(),
            status,
            token_usage,
            input_tokens,
            output_tokens,
            latency_ms: Some(latency_ms as u32),
            error_code,
        });
    }

    /// Somme des tokens enregistrés sur chaque step (chapitres + summary).
    pub fn aggregate_token_usage(&self) -> (Option<i32>, Option<i32>) {
        let input: Option<u32> = self
            .steps
            .iter()
            .filter_map(|s| s.input_tokens)
            .reduce(|a, b| a.saturating_add(b));
        let output: Option<u32> = self
            .steps
            .iter()
            .filter_map(|s| s.output_tokens)
            .reduce(|a, b| a.saturating_add(b));
        (
            input.map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
            output.map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        )
    }

    pub fn aggregate_detailed_usage(&self) -> Option<TokenUsage> {
        let mut usage = TokenUsage::default();
        for step in &self.steps {
            if let Some(step_usage) = &step.token_usage {
                for item in &step_usage.items {
                    usage.push(item.clone());
                }
            }
        }
        (!usage.is_empty()).then_some(usage)
    }
}

use astral_llm_domain::{ChapterGenerationStatus, GenerationStepRecord};

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
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
        latency_ms: u64,
        error_code: Option<String>,
    ) {
        self.push_step(GenerationStepRecord {
            step_type: "chapter_generate".into(),
            chapter_code: Some(chapter_code.to_string()),
            provider: provider.to_string(),
            model: model.to_string(),
            status,
            input_tokens,
            output_tokens,
            latency_ms: Some(latency_ms as u32),
            error_code,
        });
    }
}

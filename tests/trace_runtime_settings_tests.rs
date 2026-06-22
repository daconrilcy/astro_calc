use std::path::PathBuf;

use astral_llm_application::prompt_trace::{
    configure_prompt_trace, prompt_log_base_dir, PromptTraceSettings,
};
use astral_llm_application::raw_provider_trace::{
    configure_raw_provider_trace, raw_provider_trace_base_dir, RawProviderTraceSettings,
};
use astral_llm_domain::AstralLlmEnv;

#[test]
fn trace_runtime_settings_control_log_directories_without_env_reads() {
    let disabled_prompt = PromptTraceSettings::from_runtime(false, Some(PathBuf::from("ignored")));
    configure_prompt_trace(disabled_prompt);
    assert_eq!(prompt_log_base_dir(), None);

    let prompt_dir = PathBuf::from("output/test-prompts");
    configure_prompt_trace(PromptTraceSettings::from_runtime(
        true,
        Some(prompt_dir.clone()),
    ));
    assert_eq!(prompt_log_base_dir(), Some(prompt_dir));

    let disabled_raw = RawProviderTraceSettings::from_runtime(
        AstralLlmEnv::Local,
        false,
        Some(PathBuf::from("ignored")),
    );
    configure_raw_provider_trace(disabled_raw);
    assert_eq!(raw_provider_trace_base_dir(), None);

    let raw_dir = PathBuf::from("output/test-raw");
    configure_raw_provider_trace(RawProviderTraceSettings::from_runtime(
        AstralLlmEnv::Local,
        true,
        Some(raw_dir.clone()),
    ));
    assert_eq!(raw_provider_trace_base_dir(), Some(raw_dir));

    configure_prompt_trace(PromptTraceSettings::default());
    configure_raw_provider_trace(RawProviderTraceSettings::for_runtime_env(
        AstralLlmEnv::Local,
    ));
}

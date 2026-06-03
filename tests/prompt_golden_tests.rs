//! Golden test : le prompt compile ne doit jamais contenir de PII ni d'injection.

#[test]
fn compiled_prompt_excludes_pii_and_injection() {
    let prompts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    astral_llm_application::prompt_golden::assert_compiled_prompt_is_safe(&prompts)
        .expect("compiled prompt must not leak PII or injection strings");
}

//! Golden test : le prompt compile ne doit jamais contenir de PII ni d'injection.

mod prompt_golden_support;

#[test]
fn compiled_prompt_excludes_pii_and_injection() {
    let prompts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    prompt_golden_support::assert_compiled_prompt_is_safe(&prompts)
        .expect("compiled prompt must not leak PII or injection strings");
}

#[test]
fn premium_plus_prompt_keeps_editorial_structure() {
    let prompts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    prompt_golden_support::assert_premium_plus_prompt_structure(&prompts)
        .expect("premium_plus prompt structure must stay editorial");
}

#[test]
fn premium_prompt_keeps_strengthened_compact_structure() {
    let prompts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    prompt_golden_support::assert_premium_compact_prompt_structure(&prompts)
        .expect("premium prompt structure must stay strengthened and compact");
}

use astral_llm_infra::SharedCanonicalCatalog;

pub struct WritingLanguageDirective;

impl WritingLanguageDirective {
    pub fn prompt_block(catalog: &SharedCanonicalCatalog, user_language: &str) -> String {
        if let Some(locale) = catalog.writing_locale(user_language) {
            return locale.prompt_instruction.clone();
        }
        let code = user_language.trim().to_lowercase();
        format!(
            "OUTPUT_LANGUAGE: {code}. Write title, body, summary fields, and human-readable astro_basis strings (factor, label) in language {code}. Never translate fact_ids."
        )
    }
}

use astral_llm_infra::{bootstrap_writing_locales, SharedCanonicalCatalog};

pub struct WritingLanguageDirective;

impl WritingLanguageDirective {
    pub fn prompt_block(catalog: &SharedCanonicalCatalog, user_language: &str) -> String {
        if let Some(locale) = catalog.writing_locale(user_language) {
            return locale.prompt_instruction.clone();
        }
        let code = user_language.trim().to_lowercase();
        for locale in bootstrap_writing_locales() {
            if locale.locale_code == code {
                return locale.prompt_instruction;
            }
        }
        format!(
            "OUTPUT_LANGUAGE: {code}. Write title, body, summary fields, and human-readable astro_basis strings (factor, label) in language {code}. Never translate fact_ids."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_infra::{bootstrap_writing_locales, CanonicalCatalog};
    use std::sync::Arc;

    #[test]
    fn includes_spanish_instruction() {
        let mut c = CanonicalCatalog::default();
        c.writing_locales = bootstrap_writing_locales();
        let block = WritingLanguageDirective::prompt_block(&Arc::new(c), "es");
        assert!(block.contains("OUTPUT_LANGUAGE: es"));
    }
}

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

    pub fn public_abbreviation_rule(user_language: &str) -> String {
        let code = user_language.trim().to_lowercase();
        if code.starts_with("fr") {
            return "PUBLIC_ASTRO_ABBREVIATIONS: Dans le texte public, ne reprenez pas les abreviations astrologiques isolees issues des donnees. Remplacez-les par un libelle parlant pour un non-initie : ecrivez \"Milieu du Ciel\" au lieu de \"MC\", \"Fond du Ciel\" au lieu de \"IC\", \"Ascendant\" au lieu de \"ASC\" et \"Descendant\" au lieu de \"DSC\".".into();
        }
        if code.starts_with("en") {
            return "PUBLIC_ASTRO_ABBREVIATIONS: In public prose, do not copy isolated astrological abbreviations from the data. Expand them for non-specialist readers: write \"Midheaven\" instead of \"MC\", \"Imum Coeli\" instead of \"IC\", \"Ascendant\" instead of \"ASC\", and \"Descendant\" instead of \"DSC\".".into();
        }
        "PUBLIC_ASTRO_ABBREVIATIONS: In public prose, do not copy isolated astrological abbreviations from the data. Use the full localized public label for the output language: for example Midheaven/Milieu du Ciel instead of MC, Imum Coeli/Fond du Ciel instead of IC, Ascendant instead of ASC, and Descendant instead of DSC.".into()
    }
}

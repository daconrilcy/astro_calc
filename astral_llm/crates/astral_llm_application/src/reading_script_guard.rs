use astral_llm_domain::NatalReadingResponse;

/// Rejette les contaminations de script (ex. devanagari dans un texte `fr`).
pub fn script_violations_for_reading(language: &str, reading: &NatalReadingResponse) -> Vec<String> {
    let lang = language.trim().to_lowercase();
    if lang != "fr" {
        return Vec::new();
    }

    let mut violations = Vec::new();
    check_field(&mut violations, "summary.title", &reading.summary.title, &lang);
    check_field(
        &mut violations,
        "summary.short_text",
        &reading.summary.short_text,
        &lang,
    );
    check_field(
        &mut violations,
        "legal.disclaimer",
        &reading.legal.disclaimer,
        &lang,
    );
    for (i, ch) in reading.chapters.iter().enumerate() {
        check_field(&mut violations, &format!("chapters[{i}].title"), &ch.title, &lang);
        check_field(&mut violations, &format!("chapters[{i}].body"), &ch.body, &lang);
    }
    violations
}

fn check_field(out: &mut Vec<String>, field: &str, text: &str, lang: &str) {
    if let Some(ch) = first_unexpected_french_script_char(text) {
        out.push(format!(
            "unexpected script in {field} (language={lang}): '{ch}' U+{:04X}",
            ch as u32
        ));
    }
}

fn first_unexpected_french_script_char(text: &str) -> Option<char> {
    for ch in text.chars() {
        if ch.is_whitespace() || ch.is_ascii_digit() || is_french_punctuation(ch) {
            continue;
        }
        if ch.is_ascii() {
            continue;
        }
        if is_latin_extended_for_french(ch) {
            continue;
        }
        return Some(ch);
    }
    None
}

fn is_french_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '«' | '»' | '—' | '–' | '…' | '’' | '“' | '”' | '•' | '·' | ' ' | ' '
    )
}

fn is_latin_extended_for_french(ch: char) -> bool {
    ('\u{00C0}'..='\u{024F}').contains(&ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::generation_response::{
        ConfidenceLevel, LegalBlock, NatalReadingResponse, QualityMetadata, ReadingChapter,
        ReadingSummary,
    };
    use astral_llm_domain::output_contract::GenerationMode;

    fn sample_reading(body: &str) -> NatalReadingResponse {
        NatalReadingResponse {
            schema_version: "natal_reading_v1".into(),
            language: "fr".into(),
            reading_type: "natal_prompter".into(),
            summary: ReadingSummary {
                title: "T".into(),
                short_text: "S".into(),
            },
            chapters: vec![ReadingChapter {
                code: "identity".into(),
                title: "Identite".into(),
                body: body.into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::Medium,
                safety_flags: vec![],
            }],
            legal: LegalBlock {
                disclaimer: "Disclaimer".into(),
            },
            quality: QualityMetadata {
                used_provider: "fake".into(),
                used_model: "fake".into(),
                generation_mode: GenerationMode::SinglePass,
                prompt_family: "natal_prompter".into(),
                prompt_version: "v1".into(),
                astro_contract_version: "natal_simplified_structured_v1".into(),
                fallback_used: false,
            },
        }
    }

    #[test]
    fn rejects_devanagari_in_french_body() {
        let reading = sample_reading("fondée sur des संकेत astrologiques");
        let v = script_violations_for_reading("fr", &reading);
        assert!(!v.is_empty());
    }

    #[test]
    fn allows_french_accents() {
        let reading = sample_reading("Interprétation partielle — élégante.");
        assert!(script_violations_for_reading("fr", &reading).is_empty());
    }
}

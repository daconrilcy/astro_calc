use astral_llm_domain::NatalReadingResponse;

/// Rejette les contaminations de script (ex. devanagari dans un texte `fr`).
pub fn script_violations_for_reading(
    language: &str,
    reading: &NatalReadingResponse,
) -> Vec<String> {
    let lang = language.trim().to_lowercase();
    if lang != "fr" {
        return Vec::new();
    }

    let mut violations = Vec::new();
    check_field(
        &mut violations,
        "summary.title",
        &reading.summary.title,
        &lang,
    );
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
        check_field(
            &mut violations,
            &format!("chapters[{i}].title"),
            &ch.title,
            &lang,
        );
        check_field(
            &mut violations,
            &format!("chapters[{i}].body"),
            &ch.body,
            &lang,
        );
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

/// Retire les caractères hors alphabet français autorisé.
pub fn sanitize_text_for_french_script(text: &str) -> (String, bool) {
    let mut changed = false;
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_whitespace()
            || ch.is_ascii_digit()
            || is_french_punctuation(ch)
            || ch.is_ascii()
            || is_latin_extended_for_french(ch)
        {
            out.push(ch);
        } else {
            changed = true;
        }
    }
    let collapsed = collapse_whitespace(&out);
    if collapsed != text {
        changed = true;
    }
    (collapsed, changed)
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn violations_are_script_only(violations: &[String]) -> bool {
    !violations.is_empty()
        && violations
            .iter()
            .all(|v| v.contains("unexpected script in"))
}

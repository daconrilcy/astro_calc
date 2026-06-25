//! Helpers de langue pour les textes natals.

/// Normalise un code langue vers le locale supporte.
pub fn locale_key(language: &str) -> &str {
    let code = language.trim().to_lowercase();
    match code.as_str() {
        s if s.starts_with("fr") => "fr",
        s if s.starts_with("es") => "es",
        s if s.starts_with("de") => "de",
        _ => "en",
    }
}

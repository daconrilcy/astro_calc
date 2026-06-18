//! Module astral_calculator\src\features\natal\signals\utils.rs du moteur astral_calculator.

pub fn indefinite_article(phrase: &str) -> &'static str {
    match phrase
        .chars()
        .next()
        .map(|letter| letter.to_ascii_lowercase())
    {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

pub(super) fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

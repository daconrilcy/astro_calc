//! Normalisation typographique française (élisions manquantes après génération LLM).

use std::sync::OnceLock;

use regex::{Captures, Regex};

static ELISION_RE: OnceLock<Regex> = OnceLock::new();

fn elision_re() -> &'static Regex {
    ELISION_RE.get_or_init(|| {
        Regex::new(r"(?i)\b(l|d|j|m|n|t|s|c|qu) ([aeiouyhAEIOUYHÉÈÊÀÂËÏÎÔÙÛÜéèêàâëïîôùûü])")
            .expect("elision regex")
    })
}

/// Restaure les apostrophes d'élision (`l impression` → `l'impression`).
pub fn restore_french_elisions(text: &str) -> (String, bool) {
    if !text.contains(' ') {
        return (text.to_string(), false);
    }
    let mut changed = false;
    let out = elision_re()
        .replace_all(text, |caps: &Captures| {
            changed = true;
            let particle = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let next = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            format!("{particle}'{next}")
        })
        .into_owned();
    (out, changed)
}

/// Détecte les élisions manquantes typiques (pour tests / garde qualité).
pub fn french_elision_violations(text: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for caps in elision_re().captures_iter(text) {
        let particle = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let next = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        violations.push(format!("missing elision: {particle} {next}"));
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_common_elisions() {
        let (out, changed) = restore_french_elisions(
            "l impression générale est celle d une personne. Ce n est pas figé. \
             qu elle montre, s appuie sur l observation d abord.",
        );
        assert!(changed);
        assert!(out.contains("l'impression"));
        assert!(out.contains("d'une"));
        assert!(out.contains("n'est"));
        assert!(out.contains("qu'elle"));
        assert!(out.contains("s'appuie"));
        assert!(out.contains("l'observation"));
        assert!(out.contains("d'abord"));
        assert!(french_elision_violations(&out).is_empty());
    }

    #[test]
    fn leaves_correct_apostrophes_unchanged() {
        let input = "l'identité apparaît d'abord très mobile.";
        let (out, changed) = restore_french_elisions(input);
        assert!(!changed);
        assert_eq!(out, input);
    }
}

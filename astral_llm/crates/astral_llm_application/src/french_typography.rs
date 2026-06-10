//! Normalisation typographique française (élisions manquantes après génération LLM).

use std::sync::OnceLock;

use regex::{Captures, Regex};

static ELISION_RE: OnceLock<Regex> = OnceLock::new();
static BAD_ELISION_RE: OnceLock<Regex> = OnceLock::new();
static GLUED_IMPERATIVE_RE: OnceLock<Regex> = OnceLock::new();

const GLUED_COMPOUND_FIXES: &[(&str, &str)] = &[
    ("rendezvous", "rendez-vous"),
    ("Rendezvous", "Rendez-vous"),
    ("bouclezla", "bouclez-la"),
    ("Bouclezla", "Bouclez-la"),
    ("fermezla", "fermez-la"),
    ("Fermezla", "Fermez-la"),
    ("laissezle", "laissez-le"),
    ("Laissezle", "Laissez-le"),
    ("faitesle", "faites-le"),
    ("Faitesle", "Faites-le"),
    ("retirezvous", "retirez-vous"),
    ("Retirezvous", "Retirez-vous"),
    ("réduisezle", "réduisez-le"),
    ("Réduisezle", "Réduisez-le"),
    ("allegezle", "allégez-le"),
    ("Allegezle", "Allégez-le"),
    ("allégezle", "allégez-le"),
    ("Allégezle", "Allégez-le"),
    ("terminezla", "terminez-la"),
    ("Terminezla", "Terminez-la"),
    ("diminuezle", "diminuez-le"),
    ("Diminuezle", "Diminuez-le"),
    ("déléguezla", "déléguez-la"),
    ("Déléguezla", "Déléguez-la"),
    ("transformezle", "transformez-le"),
    ("Transformezle", "Transformez-le"),
    ("accordezvous", "accordez-vous"),
    ("Accordezvous", "Accordez-vous"),
    ("autorisezvous", "autorisez-vous"),
    ("Autorisezvous", "Autorisez-vous"),
    ("arrêtezvous", "arrêtez-vous"),
    ("Arrêtezvous", "Arrêtez-vous"),
    ("arretezvous", "arrêtez-vous"),
    ("Arretezvous", "Arrêtez-vous"),
    ("formulezle", "formulez-le"),
    ("Formulezle", "Formulez-le"),
    ("utilisezles", "utilisez-les"),
    ("Utilisezles", "Utilisez-les"),
    ("revenezy", "revenez-y"),
    ("Revenezy", "Revenez-y"),
    ("joursclés", "jours clés"),
    ("Joursclés", "Jours clés"),
    ("jourscles", "jours clés"),
    ("Jourscles", "Jours clés"),
    ("phraseclé", "phrase-clé"),
    ("Phraseclé", "Phrase-clé"),
];

fn elision_re() -> &'static Regex {
    ELISION_RE.get_or_init(|| {
        Regex::new(r"(?i)\b(l|d|j|m|n|t|s|c|qu) ([aeiouyhAEIOUYHÉÈÊÀÂËÏÎÔÙÛÜéèêàâëïîôùûü])")
            .expect("elision regex")
    })
}

fn bad_elision_re() -> &'static Regex {
    BAD_ELISION_RE.get_or_init(|| {
        Regex::new(r"(?i)\b(l|d|j|m|n|t|s|c|qu)['’]([bcdfgjklmnpqrstvwxzç])")
            .expect("bad elision regex")
    })
}

fn glued_imperative_re() -> &'static Regex {
    GLUED_IMPERATIVE_RE.get_or_init(|| {
        Regex::new(r"\b(\p{L}+ez)(le|la|les|vous|y)\b").expect("glued imperative regex")
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
    for caps in bad_elision_re().captures_iter(text) {
        let particle = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let next = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        violations.push(format!("bad elision: {particle}'{next}"));
    }
    violations
}

/// Restaure quelques mots composés français que les modèles collent parfois.
pub fn restore_french_glued_compounds(text: &str) -> (String, bool) {
    let mut out = text.to_string();
    let mut changed = false;
    for (bad, replacement) in GLUED_COMPOUND_FIXES {
        if out.contains(bad) {
            out = out.replace(bad, replacement);
            changed = true;
        }
    }
    out = glued_imperative_re()
        .replace_all(&out, |caps: &Captures| {
            changed = true;
            let verb = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let pronoun = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            format!("{verb}-{pronoun}")
        })
        .into_owned();
    (out, changed)
}

/// Détecte les mots composés collés que la sortie publique ne doit pas exposer.
pub fn french_glued_compound_violations(text: &str) -> Vec<String> {
    let mut violations = GLUED_COMPOUND_FIXES
        .iter()
        .filter_map(|(bad, _)| text.contains(bad).then(|| (*bad).to_string()))
        .collect::<Vec<_>>();
    violations.extend(
        glued_imperative_re()
            .captures_iter(text)
            .filter_map(|caps| caps.get(0).map(|m| m.as_str().to_string())),
    );
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

    #[test]
    fn detects_bad_elision_before_consonant() {
        let violations = french_elision_violations("La semaine permet d’réaccorder le cadre.");
        assert!(violations.iter().any(|item| item == "bad elision: d'r"));
    }

    #[test]
    fn restores_glued_compounds() {
        let (out, changed) = restore_french_glued_compounds(
            "Confirmez un rendezvous, bouclezla, puis laissezle reposer. Terminezla, diminuezle et Autorisezvous une pause.",
        );
        assert!(changed);
        assert!(out.contains("rendez-vous"));
        assert!(out.contains("bouclez-la"));
        assert!(out.contains("laissez-le"));
        assert!(out.contains("Terminez-la"));
        assert!(out.contains("diminuez-le"));
        assert!(out.contains("Autorisez-vous"));
        let (out, changed) =
            restore_french_glued_compounds("Utilisezles, revenezy et arrêtezvous.");
        assert!(changed);
        assert!(out.contains("Utilisez-les"));
        assert!(out.contains("revenez-y"));
        assert!(out.contains("arrêtez-vous"));
        assert!(french_glued_compound_violations(&out).is_empty());
    }
}

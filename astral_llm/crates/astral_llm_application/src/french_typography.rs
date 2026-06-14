//! Normalisation typographique française (élisions manquantes après génération LLM).

use std::sync::OnceLock;

use regex::{Captures, Regex};

static ELISION_RE: OnceLock<Regex> = OnceLock::new();
static BAD_ELISION_RE: OnceLock<Regex> = OnceLock::new();
static GLUED_IMPERATIVE_RE: OnceLock<Regex> = OnceLock::new();

const GLUED_COMPOUND_FIXES: &[(&str, &str)] = &[
    ("aprèsmidi", "après-midi"),
    ("Aprèsmidi", "Après-midi"),
    ("apresmidi", "après-midi"),
    ("Apresmidi", "Après-midi"),
    ("qu’estce", "qu’est-ce"),
    ("Qu’estce", "Qu’est-ce"),
    ("qu'estce", "qu'est-ce"),
    ("Qu'estce", "Qu'est-ce"),
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
    ("allègerez", "allégez"),
    ("Allègerez", "Allégez"),
    ("allegerez", "allégez"),
    ("Allegerez", "Allégez"),
    ("allége la", "allégez la"),
    ("Allége la", "Allégez la"),
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
    ("mesurezl", "mesurez-la"),
    ("Mesurezl", "Mesurez-la"),
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
    ("demipromesses", "demi-promesses"),
    ("Demipromesses", "Demi-promesses"),
    ("demi promesses", "demi-promesses"),
    ("Demi promesses", "Demi-promesses"),
    ("Evitez", "Évitez"),
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

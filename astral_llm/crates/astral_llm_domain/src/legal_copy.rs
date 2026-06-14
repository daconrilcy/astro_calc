//! Textes legaux affiches dans la lecture (disclaimer).

pub fn default_legal_disclaimer(language: &str, include: bool) -> String {
    if !include {
        return String::new();
    }
    if language.starts_with("fr") {
        "Cette lecture est une interprétation symbolique et ne remplace aucun avis médical, \
         psychologique, juridique ou financier."
            .into()
    } else {
        "This reading is a symbolic interpretation and does not replace medical, psychological, \
         legal, or financial advice."
            .into()
    }
}

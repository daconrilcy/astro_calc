//! Trigrammes pour detection de repetition et consignes anti-repetition en amont.

use std::collections::HashMap;

const STOPWORDS_FR: &[&str] = &[
    "a", "au", "aux", "avec", "ce", "ces", "cette", "d", "dans", "de", "des", "du", "en", "et",
    "il", "la", "le", "les", "l", "mais", "ne", "on", "ou", "par", "pas", "pour", "que", "qui",
    "se", "son", "sur", "un", "une", "vos", "votre", "vous", "est", "sont", "peut",
];

const STOPWORDS_EN: &[&str] = &[
    "a", "an", "and", "as", "at", "be", "by", "for", "from", "in", "is", "it", "of", "on", "or",
    "that", "the", "this", "to", "was", "with", "you", "your", "are", "may",
];

pub fn trigram_phrases(body: &str) -> Vec<String> {
    let words: Vec<&str> = body.split_whitespace().collect();
    if words.len() < 3 {
        return Vec::new();
    }
    words
        .windows(3)
        .map(|w| format!("{} {} {}", w[0].to_lowercase(), w[1].to_lowercase(), w[2].to_lowercase()))
        .collect()
}

pub fn is_low_signal_trigram(phrase: &str, locale: &str) -> bool {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    if words.len() != 3 {
        return true;
    }
    let stops = match locale {
        "fr" | "es" => STOPWORDS_FR,
        _ => STOPWORDS_EN,
    };
    let stop_count = words.iter().filter(|w| stops.contains(w)).count();
    stop_count >= 2
}

pub fn count_repeated_trigrams(body: &str, locale: &str) -> usize {
    let mut counts = HashMap::<String, usize>::new();
    for phrase in trigram_phrases(body) {
        if is_low_signal_trigram(&phrase, locale) {
            continue;
        }
        *counts.entry(phrase).or_insert(0) += 1;
    }
    counts.values().filter(|&&n| n > 1).count()
}

/// Phrases deja employees dans les chapitres precedents (meme trigramme dans 2+ chapitres).
pub const STOCK_OPENINGS_FR: &[&str] = &[
    "des le premier regard",
    "dès le premier regard",
    "cette configuration",
    "votre parcours",
    "dans l'ensemble",
    "en toile de fond",
    "cette dynamique",
    "ainsi, votre",
];

/// Amorces de paragraphe trop generiques pour bloquer en cross-chapitre (connecteurs redactionnels).
const GENERIC_PARA_OPENING_PREFIXES_FR: &[&str] = &[
    "par ailleurs",
    "en synthese",
    "en synthèse",
    "pour finir",
    "en conclusion",
    "dans un second",
    "dans cette",
    "cependant",
    "toutefois",
    "en outre",
    "de plus",
    "ainsi la",
    "en explorant",
    "en integrant",
    "sous l'influence",
    "sous l influence",
    "dans le même",
    "dans le meme",
    "dans cette dynamique",
    "au fil du",
    "dans la vie",
    "au quotidien",
    "concrètement cela",
    "concretement cela",
    "dans l'expression",
    "dans l expression",
    "en pratique cela",
];

pub fn is_generic_paragraph_opening(phrase: &str) -> bool {
    GENERIC_PARA_OPENING_PREFIXES_FR
        .iter()
        .any(|prefix| phrase.starts_with(prefix))
}

/// Amorce « planète en signe en » (ex. jupiter en cancer en) — collision frequente entre chapitres.
pub fn is_planet_in_sign_paragraph_opening(phrase: &str) -> bool {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    words.len() >= 4 && words[1] == "en" && words[3] == "en"
}

pub fn source_chapter_from_duplicate_kind(kind: &str) -> Option<&str> {
    kind.strip_prefix("chapter_opening_duplicate_of:")
        .or_else(|| kind.strip_prefix("paragraph_opening_duplicate_of:"))
        .or_else(|| kind.strip_prefix("stock_opening_duplicate_of:"))
}

fn normalize_opening_words(text: &str) -> String {
    text.split_whitespace()
        .take(8)
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Cinq premiers mots significatifs du chapitre (ouverture).
pub fn chapter_opening_phrase(body: &str, _locale: &str) -> String {
    let trimmed = body.trim();
    let first_block = trimmed.split("\n\n").next().unwrap_or(trimmed);
    normalize_opening_words(first_block)
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quatre premiers mots de chaque paragraphe.
pub fn paragraph_opening_phrases(body: &str) -> Vec<String> {
    body.split("\n\n")
        .map(|p| {
            normalize_opening_words(p)
                .split_whitespace()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|p| p.split_whitespace().count() >= 3)
        .collect()
}

/// Amorces des chapitres precedents a proscrire pour le chapitre courant.
pub fn openings_to_avoid_from_prior(
    prior_bodies: &[&str],
    locale: &str,
    max: usize,
) -> Vec<String> {
    let mut phrases = Vec::new();
    for body in prior_bodies {
        let opening = chapter_opening_phrase(body, locale);
        if opening.split_whitespace().count() >= 3 && !phrases.contains(&opening) {
            phrases.push(opening);
        }
        for p in paragraph_opening_phrases(body) {
            if !phrases.contains(&p) {
                phrases.push(p);
            }
        }
    }
    phrases.truncate(max);
    phrases
}

pub fn detect_duplicate_openings(
    chapters: &[astral_llm_domain::generation_response::ReadingChapter],
    locale: &str,
) -> Vec<(String, String, String)> {
    let mut violations = Vec::new();
    let mut seen_chapter_openings: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut seen_para_openings: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for chapter in chapters {
        let ch_open = chapter_opening_phrase(&chapter.body, locale);
        if ch_open.split_whitespace().count() >= 4 {
            if let Some(other) = seen_chapter_openings.get(&ch_open) {
                violations.push((
                    chapter.code.clone(),
                    ch_open.clone(),
                    format!("chapter_opening_duplicate_of:{other}"),
                ));
            } else {
                seen_chapter_openings.insert(ch_open.clone(), chapter.code.clone());
            }
            for stock in STOCK_OPENINGS_FR {
                if ch_open.starts_with(stock) {
                    let key = format!("stock:{stock}");
                    if let Some(other) = seen_chapter_openings.get(&key) {
                        violations.push((
                            chapter.code.clone(),
                            ch_open.clone(),
                            format!("stock_opening_duplicate_of:{other}"),
                        ));
                    } else {
                        seen_chapter_openings.insert(key, chapter.code.clone());
                    }
                }
            }
        }
        for para in paragraph_opening_phrases(&chapter.body) {
            if para.split_whitespace().count() < 4 || is_generic_paragraph_opening(&para) {
                continue;
            }
            if let Some(other) = seen_para_openings.get(&para) {
                violations.push((
                    chapter.code.clone(),
                    para.clone(),
                    format!("paragraph_opening_duplicate_of:{other}"),
                ));
            } else {
                seen_para_openings.insert(para.clone(), chapter.code.clone());
            }
        }
    }
    violations
}

pub fn phrases_to_avoid_from_prior(prior_bodies: &[&str], locale: &str, max: usize) -> Vec<String> {
    let mut chapter_hits: HashMap<String, usize> = HashMap::new();
    for body in prior_bodies {
        let mut seen = std::collections::HashSet::new();
        for phrase in trigram_phrases(body) {
            if is_low_signal_trigram(&phrase, locale) {
                continue;
            }
            if seen.insert(phrase.clone()) {
                *chapter_hits.entry(phrase).or_insert(0) += 1;
            }
        }
    }
    let mut phrases: Vec<_> = chapter_hits
        .into_iter()
        .filter(|(_, chapters)| *chapters >= 2)
        .map(|(p, _)| p)
        .collect();
    phrases.sort_by(|a, b| b.len().cmp(&a.len()));
    phrases.truncate(max);
    phrases
}

fn normalize_paragraph_for_placement_check(paragraph: &str) -> String {
    paragraph
        .trim()
        .chars()
        .map(|c| match c {
            '’' | '\'' => '\'',
            c if c.is_alphanumeric() || c.is_whitespace() => c,
            _ => ' ',
        })
        .collect::<String>()
        .to_lowercase()
}

fn placement_preposition(locale: &str) -> &'static str {
    match locale {
        "fr" | "es" => "en",
        _ => "in",
    }
}

/// Paragraphe commençant par « [planète] en/in » (article optionnel), noms fournis par le catalogue locale.
pub fn paragraph_starts_with_raw_placement(
    paragraph: &str,
    planet_names: &[String],
    locale: &str,
) -> bool {
    let normalized = normalize_paragraph_for_placement_check(paragraph);
    if normalized.is_empty() {
        return false;
    }
    let prep = placement_preposition(locale);
    let articles = ["le ", "la ", "l'"];
    for planet in planet_names {
        let planet_lower = planet.trim().to_lowercase();
        if planet_lower.is_empty() {
            continue;
        }
        if normalized.starts_with(&format!("{planet_lower} {prep} ")) {
            return true;
        }
        for art in articles {
            if normalized.starts_with(&format!("{art}{planet_lower} {prep} ")) {
                return true;
            }
        }
    }
    false
}

fn raw_placement_opening_prefix(locale: &str) -> &'static str {
    match locale {
        "fr" => "Dans cette perspective, ",
        "es" => "En esta perspectiva, ",
        "de" => "In dieser Perspektive ",
        _ => "In this perspective, ",
    }
}

/// Préfixe neutre si un paragraphe commence encore par « planète en signe » (fallback déterministe post-repair).
pub fn soften_raw_placement_openings_in_body(
    body: &str,
    planet_names: &[String],
    locale: &str,
) -> String {
    body.split("\n\n")
        .map(|para| {
            if paragraph_starts_with_raw_placement(para, planet_names, locale) {
                format!("{}{para}", raw_placement_opening_prefix(locale))
            } else {
                para.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn detect_raw_placement_paragraph_openings(
    body: &str,
    planet_names: &[String],
    locale: &str,
) -> Vec<String> {
    body.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter(|p| paragraph_starts_with_raw_placement(p, planet_names, locale))
        .map(|p| {
            let words: Vec<_> = p.split_whitespace().take(8).collect();
            words.join(" ")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_in_sign_opening_pattern() {
        assert!(is_planet_in_sign_paragraph_opening("jupiter en cancer en"));
        assert!(!is_planet_in_sign_paragraph_opening("par ailleurs la presence"));
    }

    #[test]
    fn detects_raw_placement_paragraph_opening_fr() {
        let planets = vec![
            "Soleil".into(),
            "Lune".into(),
            "Mars".into(),
            "Jupiter".into(),
            "Saturne".into(),
        ];
        assert!(paragraph_starts_with_raw_placement(
            "Mars en Sagittaire en maison 1 colore votre présence.",
            &planets,
            "fr",
        ));
        assert!(!paragraph_starts_with_raw_placement(
            "Par ailleurs Mars en Sagittaire structure le récit.",
            &planets,
            "fr",
        ));
        let openings = detect_raw_placement_paragraph_openings(
            "Intro neutre.\n\nMars en Sagittaire en maison 1 ouvre le chapitre.\n\nSuite.",
            &planets,
            "fr",
        );
        assert_eq!(openings.len(), 1);

        let planets_en = vec!["Sun".into(), "Mars".into()];
        assert!(paragraph_starts_with_raw_placement(
            "Mars in Sagittarius in house 1 shapes your presence.",
            &planets_en,
            "en",
        ));
        assert!(!paragraph_starts_with_raw_placement(
            "Mars en Sagittaire en maison 1",
            &planets_en,
            "en",
        ));
    }

    #[test]
    fn soften_prefixes_raw_placement_opening() {
        let planets = vec!["Jupiter".into()];
        let body = "Intro.\n\nJupiter en Cancer en maison 8 apporte une nuance.\n\nSuite.";
        let softened = soften_raw_placement_openings_in_body(&body, &planets, "fr");
        assert!(!paragraph_starts_with_raw_placement(
            softened.split("\n\n").nth(1).unwrap(),
            &planets,
            "fr",
        ));
        assert!(softened.contains("Dans cette perspective, Jupiter"));
    }

    #[test]
    fn parses_duplicate_source_chapter() {
        assert_eq!(
            source_chapter_from_duplicate_kind("paragraph_opening_duplicate_of:career"),
            Some("career")
        );
    }

    #[test]
    fn ignores_grammatical_trigrams() {
        let body = "de la vie de la vie de la vie dans votre parcours professionnel \
            avec une energie creative et une ambition structuree pour rayonner";
        assert_eq!(count_repeated_trigrams(body, "fr"), 0);
    }

    #[test]
    fn counts_meaningful_repeats() {
        let body = "cette dynamique invite cette dynamique invite cette dynamique invite \
            a explorer votre vocation avec patience et creativite authentique";
        assert!(count_repeated_trigrams(body, "fr") >= 1);
    }

    #[test]
    fn detects_duplicate_chapter_openings() {
        use astral_llm_domain::generation_response::{ConfidenceLevel, ReadingChapter};
        let chapters = vec![
            ReadingChapter {
                code: "identity".into(),
                title: "I".into(),
                body: "Des le premier regard votre carte revele une force interieure.\n\nParagraphe deux.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
            ReadingChapter {
                code: "emotional_life".into(),
                title: "E".into(),
                body: "Des le premier regard la vie emotionnelle s'exprime avec finesse.\n\nSuite.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
        ];
        let dupes = super::detect_duplicate_openings(&chapters, "fr");
        assert!(!dupes.is_empty());
    }

    #[test]
    fn ignores_generic_paragraph_transition_openings() {
        use astral_llm_domain::generation_response::{ConfidenceLevel, ReadingChapter};
        let chapters = vec![
            ReadingChapter {
                code: "identity".into(),
                title: "I".into(),
                body: "Ouverture unique sur l ascendant.\n\nPar ailleurs la presence \
                    du soleil colore le tableau.\n\nTroisieme bloc.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
            ReadingChapter {
                code: "relationships".into(),
                title: "R".into(),
                body: "Autre entree venus et partenaires.\n\nPar ailleurs la presence \
                    de venus invite a nuancer.\n\nFin.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
        ];
        assert!(super::detect_duplicate_openings(&chapters, "fr").is_empty());
    }

    #[test]
    fn ignores_dans_le_meme_mouvement_transition() {
        use astral_llm_domain::generation_response::{ConfidenceLevel, ReadingChapter};
        let chapters = vec![
            ReadingChapter {
                code: "career".into(),
                title: "C".into(),
                body: "Ouverture carriere.\n\nDans le même mouvement le milieu \
                    du ciel colore la trajectoire.\n\nSuite.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
            ReadingChapter {
                code: "growth_path".into(),
                title: "G".into(),
                body: "Ouverture croissance.\n\nDans le même mouvement saturne \
                    invite a consolider.\n\nFin.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
        ];
        assert!(super::detect_duplicate_openings(&chapters, "fr").is_empty());
    }

    #[test]
    fn ignores_dans_la_vie_quotidienne_transition() {
        use astral_llm_domain::generation_response::{ConfidenceLevel, ReadingChapter};
        let chapters = vec![
            ReadingChapter {
                code: "career".into(),
                title: "C".into(),
                body: "Ouverture carriere.\n\nDans la vie quotidienne le milieu \
                    du ciel colore la trajectoire.\n\nSuite.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
            ReadingChapter {
                code: "resources".into(),
                title: "R".into(),
                body: "Ouverture ressources.\n\nDans la vie quotidienne saturne \
                    invite a consolider.\n\nFin.".into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
        ];
        assert!(super::detect_duplicate_openings(&chapters, "fr").is_empty());
    }

    #[test]
    fn flags_distinctive_paragraph_opening_repeat() {
        use astral_llm_domain::generation_response::{ConfidenceLevel, ReadingChapter};
        let body_a = "Intro A.\n\nCette dynamique invite une lecture \
            singuliere du theme.\n\nSuite A.";
        let body_b = "Intro B.\n\nCette dynamique invite une autre \
            facette du vecu.\n\nSuite B.";
        let chapters = vec![
            ReadingChapter {
                code: "identity".into(),
                title: "I".into(),
                body: body_a.into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
            ReadingChapter {
                code: "career".into(),
                title: "C".into(),
                body: body_b.into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::High,
                safety_flags: vec![],
            },
        ];
        assert!(!super::detect_duplicate_openings(&chapters, "fr").is_empty());
    }
}

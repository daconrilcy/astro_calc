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

#[cfg(test)]
mod tests {
    use super::*;

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
}

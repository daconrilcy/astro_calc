//! Contrat UX compact pour `summary.title` et `summary.short_text`.

use serde_json::json;

use astral_llm_domain::{GenerationError, GenerationErrorCode};

#[derive(Debug, Clone, Copy)]
pub struct SummaryUxRules {
    pub max_title_words: usize,
    pub max_short_text_sentences: usize,
    pub max_short_text_words: usize,
}

impl Default for SummaryUxRules {
    fn default() -> Self {
        Self {
            max_title_words: 12,
            max_short_text_sentences: 2,
            max_short_text_words: 75,
        }
    }
}

pub fn count_words(text: &str) -> usize {
    text.split_whitespace().filter(|w| !w.trim().is_empty()).count()
}

/// Compte les phrases comme le gate E2E PowerShell : frontiere apres `.?!` suivis d'espaces.
pub fn count_sentences_fr(text: &str) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0;
    }

    let mut sentences = 1usize;
    let mut i = 0;
    while i < trimmed.len() {
        let ch = match trimmed[i..].chars().next() {
            Some(c) => c,
            None => break,
        };
        if matches!(ch, '.' | '!' | '?') {
            let mut j = i + ch.len_utf8();
            let ws_start = j;
            while j < trimmed.len() {
                let next = match trimmed[j..].chars().next() {
                    Some(c) => c,
                    None => break,
                };
                if next.is_whitespace() {
                    j += next.len_utf8();
                } else {
                    break;
                }
            }
            if j > ws_start && j < trimmed.len() {
                sentences += 1;
                i = j;
                continue;
            }
        }
        i += ch.len_utf8();
    }
    sentences
}

/// Découpe le texte en phrases (frontière après `.?!` suivis d'espaces).
pub fn split_sentences_fr(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut sentences = Vec::new();
    let mut start = 0usize;
    let mut i = 0;
    while i < trimmed.len() {
        let ch = match trimmed[i..].chars().next() {
            Some(c) => c,
            None => break,
        };
        if matches!(ch, '.' | '!' | '?') {
            let mut j = i + ch.len_utf8();
            let ws_start = j;
            while j < trimmed.len() {
                let next = match trimmed[j..].chars().next() {
                    Some(c) => c,
                    None => break,
                };
                if next.is_whitespace() {
                    j += next.len_utf8();
                } else {
                    break;
                }
            }
            if j > ws_start && j < trimmed.len() {
                sentences.push(trimmed[start..j].trim().to_string());
                start = j;
                i = j;
                continue;
            }
        }
        i += ch.len_utf8();
    }

    let tail = trimmed[start..].trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }
    sentences
}

pub fn validate_summary_ux(
    title: &str,
    short_text: &str,
    rules: &SummaryUxRules,
) -> Result<(), GenerationError> {
    let title_words = count_words(title);
    let short_words = count_words(short_text);
    let sentence_count = count_sentences_fr(short_text);

    if title_words > rules.max_title_words {
        return Err(summary_ux_error(
            "title_too_long",
            json!({ "words": title_words, "max": rules.max_title_words }),
        ));
    }

    if sentence_count > rules.max_short_text_sentences {
        return Err(summary_ux_error(
            "too_many_sentences",
            json!({ "sentences": sentence_count, "max": rules.max_short_text_sentences }),
        ));
    }

    if short_words > rules.max_short_text_words {
        return Err(summary_ux_error(
            "short_text_too_long",
            json!({ "words": short_words, "max": rules.max_short_text_words }),
        ));
    }

    Ok(())
}

fn summary_ux_error(violation: &str, details: serde_json::Value) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::ReadingQualityFailed,
        "summary UX contract violated",
        json!({
            "summary_retryable": true,
            "reason": "ux_violation",
            "violation": violation,
            "details": details,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_sentences_fr_handles_two_sentences() {
        let parts = split_sentences_fr(
            "Première phrase. Deuxième phrase plus longue, avec une virgule.",
        );
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "Première phrase.");
    }

    #[test]
    fn accepts_two_sentence_compact_summary() {
        let rules = SummaryUxRules::default();
        assert!(validate_summary_ux(
            "Présence intérieure et construction",
            "Cette lecture symbolique met en lumière une carte natale structurée par la profondeur. \
             Les chapitres développent ensuite les nuances du thème.",
            &rules,
        )
        .is_ok());
    }

    #[test]
    fn rejects_three_sentences() {
        let rules = SummaryUxRules::default();
        assert!(validate_summary_ux(
            "Titre court",
            "Première phrase. Deuxième phrase. Troisième phrase.",
            &rules,
        )
        .is_err());
    }

    #[test]
    fn sentence_count_aligns_with_whitespace_after_punctuation() {
        assert_eq!(count_sentences_fr("Première. Deuxième. Troisième."), 3);
        assert_eq!(count_sentences_fr("Première. Deuxième"), 2);
        assert_eq!(count_sentences_fr("Première.Deuxième. Troisième."), 2);
        assert_eq!(count_sentences_fr("Une seule phrase sans ponctuation finale"), 1);
    }

    #[test]
    fn rejects_title_over_twelve_words() {
        let rules = SummaryUxRules::default();
        assert!(validate_summary_ux(
            "Un deux trois quatre cinq six sept huit neuf dix onze douze treize",
            "Cette lecture symbolique met en lumière une carte natale structurée par la profondeur \
             et la constance dans la vie quotidienne et relationnelle.",
            &rules,
        )
        .is_err());
    }
}

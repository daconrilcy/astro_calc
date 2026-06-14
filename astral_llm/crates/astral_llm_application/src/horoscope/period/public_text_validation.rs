use super::*;

pub(crate) fn explicit_date_count(text: &str) -> usize {
    static DATE_RE: OnceLock<Regex> = OnceLock::new();
    DATE_RE
        .get_or_init(|| Regex::new(r"\b\d{1,2}/\d{1,2}\b").expect("explicit date regex"))
        .find_iter(text)
        .count()
}

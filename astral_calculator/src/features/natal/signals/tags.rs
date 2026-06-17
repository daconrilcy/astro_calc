pub(super) fn cluster_semantic_tags(
    sign_code: &str,
    house_number: i32,
    house_theme_code: &str,
) -> Vec<String> {
    let mut tags = vec![
        "cluster".to_string(),
        sign_code.to_string(),
        format!("house_{house_number}"),
        house_theme_code.to_string(),
    ];
    tags.extend(sign_tags(sign_code));
    tags.extend(house_tags(house_number));
    dedupe_tags(tags)
}

pub(super) fn sign_tags(sign_code: &str) -> Vec<String> {
    match sign_code {
        "aries" => vec!["initiative", "assertion"],
        "taurus" => vec!["stability", "embodiment"],
        "gemini" => vec!["learning", "adaptability"],
        "cancer" => vec!["protection", "belonging"],
        "leo" => vec!["expression", "confidence"],
        "virgo" => vec!["analysis", "service"],
        "libra" => vec!["balance", "relationship"],
        "scorpio" => vec!["intensity", "transformation"],
        "sagittarius" => vec!["meaning", "exploration"],
        "capricorn" => vec!["structure", "responsibility"],
        "aquarius" => vec!["systems", "independence"],
        "pisces" => vec!["imagination", "sensitivity"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

pub(super) fn house_tags(house_number: i32) -> Vec<String> {
    match house_number {
        1 => vec!["self_expression", "temperament"],
        2 => vec!["security", "value"],
        3 => vec!["learning", "local_environment"],
        4 => vec!["home", "family"],
        5 => vec!["pleasure", "creation"],
        6 => vec!["routine", "maintenance"],
        7 => vec!["partnership", "contracts"],
        8 => vec!["intimacy", "transformation"],
        9 => vec!["philosophy", "travel"],
        10 => vec!["vocation", "reputation"],
        11 => vec!["groups", "future_plans"],
        12 => vec!["retreat", "unconscious"],
        _ => Vec::new(),
    }
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

pub(super) fn dedupe_tags(tags: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for tag in tags {
        if !deduped.contains(&tag) {
            deduped.push(tag);
        }
    }
    deduped
}

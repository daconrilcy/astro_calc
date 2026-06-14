//! Libelles affichables pour astro_basis (sortie API post-LLM).

use astral_llm_domain::{
    astro_fact::{AstroFactKind, NormalizedAstroFact},
    generation_response::AstroBasisItem,
    NormalizedAstroFacts,
};
use astral_llm_infra::CanonicalCatalog;

pub struct AstroLabelHumanizer<'a> {
    catalog: &'a CanonicalCatalog,
}

impl<'a> AstroLabelHumanizer<'a> {
    pub fn new(catalog: &'a CanonicalCatalog) -> Self {
        Self { catalog }
    }

    pub fn locale_key(language: &str) -> &str {
        let code = language.trim().to_lowercase();
        match code.as_str() {
            s if s.starts_with("fr") => "fr",
            s if s.starts_with("es") => "es",
            s if s.starts_with("de") => "de",
            _ => "en",
        }
    }

    pub fn object_label(&self, locale: &str, code: &str) -> String {
        self.catalog
            .object_label(locale, code)
            .map(str::to_string)
            .unwrap_or_else(|| title_case_token(code))
    }

    pub fn sign_label(&self, locale: &str, code: &str) -> String {
        self.catalog
            .sign_label(locale, code)
            .map(str::to_string)
            .unwrap_or_else(|| title_case_token(code))
    }

    pub fn placement_label(
        &self,
        locale: &str,
        object_code: &str,
        sign_code: &str,
        house: Option<u64>,
    ) -> String {
        let object = self.object_label(locale, object_code);
        let sign = self.sign_label(locale, sign_code);
        match (locale, house.filter(|&h| h > 0)) {
            ("fr", Some(h)) => format!("{object} en {sign} en maison {h}"),
            ("es", Some(h)) => format!("{object} en {sign} en casa {h}"),
            ("de", Some(h)) => format!("{object} in {sign} im Haus {h}"),
            (_, Some(h)) => format!("{object} in {sign} in house {h}"),
            ("fr", None) => format!("{object} en {sign}"),
            ("es", None) => format!("{object} en {sign}"),
            ("de", None) => format!("{object} in {sign}"),
            _ => format!("{object} in {sign}"),
        }
    }

    pub fn humanize_fact_label(
        &self,
        fact: &NormalizedAstroFact,
        language: &str,
        facts: Option<&NormalizedAstroFacts>,
    ) -> String {
        let locale = Self::locale_key(language);
        if let Some(label) = self.humanize_signal_fact(fact, locale, facts) {
            return label;
        }
        if let Some((object, sign, house)) = parse_placement_fact_id(&fact.id) {
            return self.placement_label(locale, &object, &sign, Some(house));
        }
        if fact.id.starts_with("ruler:ascendant:traditional:") {
            let ruler = fact.id.rsplit(':').next().unwrap_or("ruler");
            let ruler_label = self.object_label(locale, ruler);
            return match locale {
                "fr" => format!("Maître traditionnel de l'Ascendant : {ruler_label}"),
                "es" => format!("Regente tradicional del Ascendente : {ruler_label}"),
                "de" => format!("Traditioneller Aszendenten-Herrscher : {ruler_label}"),
                _ => format!("Traditional Ascendant ruler: {ruler_label}"),
            };
        }
        if let Some((angle, sign)) = parse_angle_fact_id(&fact.id) {
            return humanize_angle_sign_label(self, locale, &angle, &sign);
        }
        if let Some((angle, sign)) = parse_signal_angle_sign_fact_id(&fact.id) {
            return humanize_angle_sign_label(self, locale, &angle, &sign);
        }
        if let Some(label) = humanize_ruler_fact_id(&fact.id, self, locale) {
            return label;
        }
        if fact.kind == AstroFactKind::PlanetPosition || fact.kind == AstroFactKind::Angle {
            if let Some(label) = label_from_fact_value(self, fact, locale) {
                return label;
            }
        }
        if let Some(label) = label_from_fact_id(&fact.id, self, language, facts) {
            return label;
        }
        fact.label.clone()
    }

    pub fn label_for_fact_id(
        &self,
        fact_id: &str,
        language: &str,
        facts: Option<&NormalizedAstroFacts>,
    ) -> Option<String> {
        label_from_fact_id(fact_id, self, language, facts)
    }

    pub fn natal_planet_display_names(&self, locale: &str) -> Vec<String> {
        const NATAL_OBJECTS: &[&str] = &[
            "sun", "moon", "mercury", "venus", "mars", "jupiter", "saturn", "uranus", "neptune",
            "pluto",
        ];
        NATAL_OBJECTS
            .iter()
            .map(|code| self.object_label(locale, code))
            .collect()
    }

    pub fn interpretive_hint_for_fact_id(
        &self,
        fact_id: &str,
        language: &str,
        facts: Option<&NormalizedAstroFacts>,
    ) -> Option<String> {
        interpretive_hint_from_fact_id(fact_id, self, language, facts)
    }

    pub fn enrich_chapter_astro_basis(
        &self,
        items: &mut [AstroBasisItem],
        facts: &NormalizedAstroFacts,
        language: &str,
    ) {
        for item in items.iter_mut() {
            let label = if let Some(fact_id) = &item.fact_id {
                if let Some(fact) = facts.fact_by_id(fact_id) {
                    Some(self.humanize_fact_label(fact, language, Some(facts)))
                } else {
                    label_from_fact_id(fact_id, self, language, Some(facts))
                }
            } else {
                None
            };
            if let Some(l) = label {
                item.label = Some(l.clone());
                // Toujours aligner factor sur le libelle canonique post-LLM (evite paraphrases du modele).
                item.factor = l;
            }
        }
    }
}

fn humanize_signal_fact_impl(
    humanizer: &AstroLabelHumanizer<'_>,
    fact: &NormalizedAstroFact,
    locale: &str,
    facts: Option<&NormalizedAstroFacts>,
) -> Option<String> {
    if !fact.id.starts_with("signal:") {
        return None;
    }
    if fact.id.starts_with("signal:aspect:") {
        return humanize_aspect_signal(humanizer, &fact.id, locale);
    }
    if fact.id.contains("dignity") {
        return humanize_dignity_signal(humanizer, fact, locale);
    }
    if fact.id.starts_with("signal:object_position:") {
        return humanize_object_position_signal(humanizer, fact, locale, facts);
    }
    if let Some((angle, sign)) = parse_signal_angle_sign_fact_id(&fact.id) {
        return Some(humanize_angle_sign_label(humanizer, locale, &angle, &sign));
    }
    None
}

impl<'a> AstroLabelHumanizer<'a> {
    fn humanize_signal_fact(
        &self,
        fact: &NormalizedAstroFact,
        locale: &str,
        facts: Option<&NormalizedAstroFacts>,
    ) -> Option<String> {
        humanize_signal_fact_impl(self, fact, locale, facts)
    }
}

fn normalize_sign_code(raw: &str) -> String {
    raw.trim().to_lowercase().replace(' ', "_")
}

fn resolve_object_placement(
    humanizer: &AstroLabelHumanizer<'_>,
    object: &str,
    fact: &NormalizedAstroFact,
    facts: Option<&NormalizedAstroFacts>,
) -> Option<(String, Option<u64>)> {
    if let Some(ev) = fact.value.get("evidence") {
        if let Some(sign) = ev
            .get("sign_code")
            .and_then(|v| v.as_str())
            .map(normalize_sign_code)
            .filter(|s| s != "unknown")
        {
            let house = ev.get("house_number").and_then(|v| v.as_u64());
            return Some((sign, house));
        }
        if let Some(name) = ev.get("sign_name").and_then(|v| v.as_str()) {
            let code = normalize_sign_code(name);
            if humanizer.catalog.sign_label("en", &code).is_some() {
                let house = ev.get("house_number").and_then(|v| v.as_u64());
                return Some((code, house));
            }
        }
    }
    if let Some(title) = fact.value.get("title").and_then(|v| v.as_str()) {
        if let Some((_, sign, house)) = parse_placement_title(title) {
            return Some((sign, Some(house)));
        }
    }
    if let Some(pool) = facts {
        if let Some((sign, house)) = placement_from_pool(pool, object) {
            return Some((sign, house));
        }
    }
    None
}

fn placement_from_pool(
    facts: &NormalizedAstroFacts,
    object: &str,
) -> Option<(String, Option<u64>)> {
    let prefix = format!("placement:{object}:");
    facts
        .facts
        .iter()
        .find(|f| f.id.starts_with(&prefix))
        .map(|f| {
            if let Some((_, sign, house)) = parse_placement_fact_id(&f.id) {
                (sign, Some(house))
            } else {
                ("unknown".into(), None)
            }
        })
}

fn humanize_object_position_signal(
    humanizer: &AstroLabelHumanizer<'_>,
    fact: &NormalizedAstroFact,
    locale: &str,
    facts: Option<&NormalizedAstroFacts>,
) -> Option<String> {
    let object = fact
        .value
        .pointer("/evidence/object_code")
        .or_else(|| fact.value.get("object_code"))
        .and_then(|v| v.as_str())
        .or_else(|| fact.id.strip_prefix("signal:object_position:"))?;
    let (sign, house) = resolve_object_placement(humanizer, object, fact, facts)?;
    Some(humanizer.placement_label(locale, object, &sign, house))
}

fn parse_placement_title(title: &str) -> Option<(String, String, u64)> {
    // "Moon in Pisces, house 4"
    let lower = title.to_lowercase();
    let rest = lower.split(" in ").nth(1)?;
    let mut parts = rest.split(", house ");
    let sign = normalize_sign_code(parts.next()?);
    let house: u64 = parts.next()?.parse().ok()?;
    Some((String::new(), sign, house))
}

fn humanize_aspect_signal(
    humanizer: &AstroLabelHumanizer<'_>,
    fact_id: &str,
    locale: &str,
) -> Option<String> {
    let rest = fact_id.strip_prefix("signal:aspect:")?;
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let aspect_code = parts[parts.len() - 1];
    let a = humanizer.object_label(locale, parts[0]);
    let b = humanizer.object_label(locale, parts[1]);
    let aspect = humanizer
        .catalog
        .aspect_label(locale, aspect_code)
        .unwrap_or(aspect_code);
    Some(match locale {
        "fr" => format!("{a} en {aspect} à {b}"),
        "es" => format!("{a} en {aspect} con {b}"),
        "de" => format!("{a} in {aspect} zu {b}"),
        _ => format!("{a} {aspect} {b}"),
    })
}

fn humanize_dignity_signal(
    humanizer: &AstroLabelHumanizer<'_>,
    fact: &NormalizedAstroFact,
    locale: &str,
) -> Option<String> {
    humanize_dignity_from_fact_id(humanizer, &fact.id, locale)
}

fn humanize_dignity_from_fact_id(
    humanizer: &AstroLabelHumanizer<'_>,
    fact_id: &str,
    locale: &str,
) -> Option<String> {
    let parts: Vec<&str> = fact_id.split(':').collect();
    if parts.len() < 5 || parts[0] != "signal" || parts[1] != "dignity" {
        return None;
    }
    let object = parts[2];
    let dignity = parts[3];
    let sign = parts[4];
    let obj = humanizer.object_label(locale, object);
    let sign_l = humanizer.sign_label(locale, sign);
    Some(match (locale, dignity) {
        ("fr", "domicile") => format!("{obj} en domicile en {sign_l}"),
        ("es", "domicile") => format!("{obj} en domicilio en {sign_l}"),
        ("de", "domicile") => format!("{obj} in Domizil in {sign_l}"),
        ("fr", _) => format!("{obj} dignité {dignity} en {sign_l}"),
        ("es", _) => format!("{obj} dignidad {dignity} en {sign_l}"),
        ("de", _) => format!("{obj} Würde {dignity} in {sign_l}"),
        _ => format!("{obj} {dignity} in {sign_l}"),
    })
}

fn label_from_fact_id(
    fact_id: &str,
    humanizer: &AstroLabelHumanizer<'_>,
    language: &str,
    facts: Option<&NormalizedAstroFacts>,
) -> Option<String> {
    let locale = AstroLabelHumanizer::locale_key(language);
    if let Some((object, sign, house)) = parse_placement_fact_id(fact_id) {
        return Some(humanizer.placement_label(locale, &object, &sign, Some(house)));
    }
    if fact_id.starts_with("signal:object_position:") {
        let object = fact_id.strip_prefix("signal:object_position:")?;
        if let Some(pool) = facts {
            if let Some((sign, house)) = placement_from_pool(pool, object) {
                return Some(humanizer.placement_label(locale, object, &sign, house));
            }
        }
    }
    if fact_id.starts_with("signal:aspect:") {
        return humanize_aspect_signal(humanizer, fact_id, locale);
    }
    if fact_id.starts_with("signal:dignity:") {
        return humanize_dignity_from_fact_id(humanizer, fact_id, locale);
    }
    if let Some((angle, sign)) = parse_signal_angle_sign_fact_id(fact_id) {
        return Some(humanize_angle_sign_label(humanizer, locale, &angle, &sign));
    }
    if let Some(label) = humanize_ruler_fact_id(fact_id, humanizer, locale) {
        return Some(label);
    }
    if let Some((kind, code)) = fact_id.split_once(':') {
        match kind {
            "element_balance" => {
                return humanizer
                    .catalog
                    .element_balance_label(locale, code)
                    .map(|p| p.display_label.clone());
            }
            "modality_balance" => {
                return humanizer
                    .catalog
                    .modality_balance_label(locale, code)
                    .map(|p| p.display_label.clone());
            }
            "sect_condition" => {
                return humanizer
                    .catalog
                    .sect_label(locale, code)
                    .map(|p| p.display_label.clone());
            }
            "house_emphasis" if fact_id.starts_with("house_emphasis:house:") => {
                let house = fact_id.rsplit(':').next()?.parse().ok()?;
                return humanizer
                    .catalog
                    .house_theme_label(locale, house)
                    .map(|p| p.display_label.clone());
            }
            "house_axis" => {
                return humanizer
                    .catalog
                    .house_axis_label(locale, code)
                    .map(|p| p.display_label.clone())
                    .or_else(|| Some(humanized_axis_code_fallback(locale, code)));
            }
            "dominant_planet" => {
                let object = humanizer.object_label(locale, code);
                return Some(match locale {
                    "fr" => format!("{object} dominante"),
                    "es" => format!("{object} dominante"),
                    "de" => format!("{object} dominant"),
                    _ => format!("{object} dominant"),
                });
            }
            "signal" if fact_id.starts_with("signal:cluster:") => {
                return humanize_cluster_signal(humanizer, fact_id, locale);
            }
            _ => {}
        }
    }
    parse_angle_fact_id(fact_id)
        .map(|(angle, sign)| humanize_angle_sign_label(humanizer, locale, &angle, &sign))
}

fn interpretive_hint_from_fact_id(
    fact_id: &str,
    humanizer: &AstroLabelHumanizer<'_>,
    language: &str,
    facts: Option<&NormalizedAstroFacts>,
) -> Option<String> {
    let locale = AstroLabelHumanizer::locale_key(language);
    if let Some((kind, code)) = fact_id.split_once(':') {
        match kind {
            "house_axis" => {
                return humanizer
                    .catalog
                    .house_axis_label(locale, code)
                    .map(|p| p.interpretive_label.clone())
                    .or_else(|| Some(humanized_axis_code_fallback(locale, code)));
            }
            "element_balance" => {
                return humanizer
                    .catalog
                    .element_balance_label(locale, code)
                    .map(|p| p.interpretive_label.clone());
            }
            "modality_balance" => {
                return humanizer
                    .catalog
                    .modality_balance_label(locale, code)
                    .map(|p| p.interpretive_label.clone());
            }
            "sect_condition" => {
                return humanizer
                    .catalog
                    .sect_label(locale, code)
                    .map(|p| p.interpretive_label.clone());
            }
            "house_emphasis" if fact_id.starts_with("house_emphasis:house:") => {
                let house = fact_id.rsplit(':').next()?.parse().ok()?;
                return humanizer
                    .catalog
                    .house_theme_label(locale, house)
                    .map(|p| p.interpretive_label.clone());
            }
            _ => {}
        }
    }
    label_from_fact_id(fact_id, humanizer, language, facts)
}

fn humanized_axis_code_fallback(locale: &str, axis_code: &str) -> String {
    let readable = axis_code.replace('_', " ");
    match locale {
        "fr" => format!("Axe {readable}"),
        "es" => format!("Eje {readable}"),
        "de" => format!("Achse {readable}"),
        _ => format!("Axis {readable}"),
    }
}

fn humanize_cluster_signal(
    humanizer: &AstroLabelHumanizer<'_>,
    fact_id: &str,
    locale: &str,
) -> Option<String> {
    let rest = fact_id.strip_prefix("signal:cluster:")?;
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    let sign = humanizer.sign_label(locale, parts[0]);
    let house = parts[1]
        .strip_prefix("house_")
        .and_then(|h| h.parse::<u8>().ok());
    Some(match (locale, house) {
        ("fr", Some(h)) => format!("Concentration en {sign} en maison {h}"),
        ("es", Some(h)) => format!("Concentración en {sign} en casa {h}"),
        ("de", Some(h)) => format!("Konzentration in {sign} im Haus {h}"),
        (_, Some(h)) => format!("Concentration in {sign} in house {h}"),
        ("fr", None) => format!("Concentration en {sign}"),
        _ => format!("Concentration in {sign}"),
    })
}

/// `ruler:angle:mc:sun`, `ruler:angle:descendant:venus`, `ruler:dominant_house:house_1:mars`
fn parse_ruler_fact_id(fact_id: &str) -> Option<(String, String, String)> {
    let rest = fact_id.strip_prefix("ruler:")?;
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}

fn humanize_ruler_fact_id(
    fact_id: &str,
    humanizer: &AstroLabelHumanizer<'_>,
    locale: &str,
) -> Option<String> {
    let (source_kind, source_code, ruler_object) = parse_ruler_fact_id(fact_id)?;
    Some(humanize_ruler_label(
        humanizer,
        locale,
        &source_kind,
        &source_code,
        &ruler_object,
    ))
}

fn humanize_ruler_label(
    humanizer: &AstroLabelHumanizer<'_>,
    locale: &str,
    source_kind: &str,
    source_code: &str,
    ruler_object: &str,
) -> String {
    let ruler_label = humanizer.object_label(locale, ruler_object);
    match locale {
        "fr" => match (source_kind, source_code) {
            ("angle", "mc") => format!("Maître du Milieu du Ciel : {ruler_label}"),
            ("angle", "descendant") => format!("Maître du Descendant : {ruler_label}"),
            ("angle", "ascendant") => format!("Maître de l'Ascendant : {ruler_label}"),
            ("angle", "ic") => format!("Maître du Fond du Ciel : {ruler_label}"),
            ("angle", other) => {
                let angle = humanizer.object_label(locale, other);
                format!("Maître de {angle} : {ruler_label}")
            }
            ("dominant_house", code) if let Some(n) = code.strip_prefix("house_") => {
                format!("Maître de la maison {n} : {ruler_label}")
            }
            _ => format!("Maître ({source_code}) : {ruler_label}"),
        },
        "es" => match (source_kind, source_code) {
            ("angle", "mc") => format!("Regente del Medio Cielo : {ruler_label}"),
            ("angle", "descendant") => format!("Regente del Descendente : {ruler_label}"),
            ("angle", other) => {
                let angle = humanizer.object_label(locale, other);
                format!("Regente de {angle} : {ruler_label}")
            }
            ("dominant_house", code) if let Some(n) = code.strip_prefix("house_") => {
                format!("Regente de la casa {n} : {ruler_label}")
            }
            _ => format!("Regente ({source_code}) : {ruler_label}"),
        },
        "de" => match (source_kind, source_code) {
            ("angle", "mc") => format!("Herrscher des Medium Coeli : {ruler_label}"),
            ("angle", "descendant") => format!("Herrscher des Deszendenten : {ruler_label}"),
            ("angle", other) => {
                let angle = humanizer.object_label(locale, other);
                format!("Herrscher von {angle} : {ruler_label}")
            }
            ("dominant_house", code) if let Some(n) = code.strip_prefix("house_") => {
                format!("Herrscher des Hauses {n} : {ruler_label}")
            }
            _ => format!("Herrscher ({source_code}) : {ruler_label}"),
        },
        _ => match (source_kind, source_code) {
            ("angle", other) => {
                let angle = humanizer.object_label(locale, other);
                format!("Ruler of {angle}: {ruler_label}")
            }
            ("dominant_house", code) if let Some(n) = code.strip_prefix("house_") => {
                format!("Ruler of house {n}: {ruler_label}")
            }
            _ => format!("Ruler ({source_code}): {ruler_label}"),
        },
    }
}

fn parse_signal_angle_sign_fact_id(id: &str) -> Option<(String, String)> {
    let rest = id.strip_prefix("signal:angle:")?;
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() >= 3 && parts[1] == "sign" {
        return Some((parts[0].to_string(), parts[2].to_string()));
    }
    None
}

fn humanize_angle_sign_label(
    humanizer: &AstroLabelHumanizer<'_>,
    locale: &str,
    angle: &str,
    sign: &str,
) -> String {
    let angle_label = humanizer.object_label(locale, angle);
    let sign_label = humanizer.sign_label(locale, sign);
    match locale {
        "fr" | "es" => format!("{angle_label} en {sign_label}"),
        "de" => format!("{angle_label} in {sign_label}"),
        _ => format!("{angle_label} in {sign_label}"),
    }
}

fn label_from_fact_value(
    humanizer: &AstroLabelHumanizer<'_>,
    fact: &NormalizedAstroFact,
    locale: &str,
) -> Option<String> {
    let object = fact
        .value
        .get("object")
        .and_then(|v| v.as_str())
        .or_else(|| {
            fact.value
                .pointer("/placement/object")
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            fact.value
                .pointer("/evidence/object_code")
                .and_then(|v| v.as_str())
        })?;
    let sign = fact
        .value
        .get("sign")
        .and_then(|v| v.as_str())
        .or_else(|| {
            fact.value
                .pointer("/placement/sign")
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            fact.value
                .pointer("/evidence/sign_code")
                .and_then(|v| v.as_str())
        })?;
    let house = fact
        .value
        .get("house")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            fact.value
                .pointer("/placement/house/number")
                .and_then(|v| v.as_u64())
        })
        .or_else(|| {
            fact.value
                .pointer("/evidence/house_number")
                .and_then(|v| v.as_u64())
        });
    Some(humanizer.placement_label(locale, object, sign, house))
}

fn parse_placement_fact_id(id: &str) -> Option<(String, String, u64)> {
    let rest = id.strip_prefix("placement:")?;
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() >= 4 && parts[parts.len() - 2] == "house" {
        let house = parts[parts.len() - 1].parse().ok()?;
        let sign = parts.get(parts.len() - 3)?.to_string();
        let object = parts[..parts.len() - 3].join(":");
        return Some((object, sign, house));
    }
    None
}

fn parse_angle_fact_id(id: &str) -> Option<(String, String)> {
    let rest = id.strip_prefix("angle:")?;
    let mut parts = rest.split(':');
    let angle = parts.next()?.to_string();
    let sign = parts.next()?.to_string();
    Some((angle, sign))
}

fn title_case_token(code: &str) -> String {
    let mut chars = code.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

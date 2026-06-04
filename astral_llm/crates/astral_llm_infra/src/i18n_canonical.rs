use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct WritingLocale {
    pub locale_code: String,
    pub iso_639_1: String,
    pub display_name: String,
    pub prompt_instruction: String,
}

pub fn bootstrap_writing_locales() -> Vec<WritingLocale> {
    vec![
        WritingLocale {
            locale_code: "fr".into(),
            iso_639_1: "fr".into(),
            display_name: "French".into(),
            prompt_instruction: "OUTPUT_LANGUAGE: fr. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in French. Astro DATA may be in English; never translate fact_ids.".into(),
        },
        WritingLocale {
            locale_code: "en".into(),
            iso_639_1: "en".into(),
            display_name: "English".into(),
            prompt_instruction: "OUTPUT_LANGUAGE: en. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in English. Astro DATA may be in another language; never translate fact_ids.".into(),
        },
        WritingLocale {
            locale_code: "es".into(),
            iso_639_1: "es".into(),
            display_name: "Spanish".into(),
            prompt_instruction: "OUTPUT_LANGUAGE: es. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in Spanish. Astro DATA may be in English; never translate fact_ids.".into(),
        },
        WritingLocale {
            locale_code: "de".into(),
            iso_639_1: "de".into(),
            display_name: "German".into(),
            prompt_instruction: "OUTPUT_LANGUAGE: de. Write title, body, summary fields, and any human-readable strings you generate in astro_basis (factor, label) in German. Astro DATA may be in English; never translate fact_ids.".into(),
        },
    ]
}

pub fn bootstrap_astro_basis_roles() -> HashSet<String> {
    ["core", "supporting", "nuance", "domain_score"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub fn bootstrap_aspect_type_labels() -> HashMap<(String, String), String> {
    let mut m = HashMap::new();
    for (aspect, fr, en, es, de) in [
        ("conjunction", "conjonction", "conjunction", "conjunción", "Konjunktion"),
        ("opposition", "opposition", "opposition", "oposición", "Opposition"),
        ("trine", "trigone", "trine", "trígono", "Trigon"),
        ("square", "carré", "square", "cuadratura", "Quadrat"),
        ("sextile", "sextile", "sextile", "sextil", "Sextil"),
    ] {
        m.insert(("fr".into(), aspect.into()), fr.into());
        m.insert(("en".into(), aspect.into()), en.into());
        m.insert(("es".into(), aspect.into()), es.into());
        m.insert(("de".into(), aspect.into()), de.into());
    }
    m
}

/// Labels planetes/signes es/de (complement au bootstrap fr/en de llm_canonical).
pub fn bootstrap_extra_object_sign_labels(
    objects: &mut HashMap<(String, String), String>,
    signs: &mut HashMap<(String, String), String>,
) {
    for (code, es, de) in [
        ("sun", "Sol", "Sonne"),
        ("moon", "Luna", "Mond"),
        ("mercury", "Mercurio", "Merkur"),
        ("venus", "Venus", "Venus"),
        ("mars", "Marte", "Mars"),
        ("jupiter", "Júpiter", "Jupiter"),
        ("saturn", "Saturno", "Saturn"),
        ("uranus", "Urano", "Uranus"),
        ("neptune", "Neptuno", "Neptun"),
        ("pluto", "Plutón", "Pluto"),
        ("ascendant", "Ascendente", "Aszendent"),
        ("descendant", "Descendente", "Deszendent"),
        ("mc", "Medio Cielo", "Medium Coeli"),
        ("ic", "Fondo del Cielo", "Imum Coeli"),
    ] {
        objects.insert(("es".into(), code.into()), es.into());
        objects.insert(("de".into(), code.into()), de.into());
    }
    for (code, es, de) in [
        ("aries", "Aries", "Widder"),
        ("taurus", "Tauro", "Stier"),
        ("gemini", "Géminis", "Zwillinge"),
        ("cancer", "Cáncer", "Krebs"),
        ("leo", "Leo", "Löwe"),
        ("virgo", "Virgo", "Jungfrau"),
        ("libra", "Libra", "Waage"),
        ("scorpio", "Escorpio", "Skorpion"),
        ("sagittarius", "Sagitario", "Schütze"),
        ("capricorn", "Capricornio", "Steinbock"),
        ("aquarius", "Acuario", "Wassermann"),
        ("pisces", "Piscis", "Fische"),
    ] {
        signs.insert(("es".into(), code.into()), es.into());
        signs.insert(("de".into(), code.into()), de.into());
    }
}

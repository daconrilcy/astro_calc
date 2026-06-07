use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct I18nLabelPair {
    pub display_label: String,
    pub interpretive_label: String,
}

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
        (
            "conjunction",
            "conjonction",
            "conjunction",
            "conjunción",
            "Konjunktion",
        ),
        (
            "opposition",
            "opposition",
            "opposition",
            "oposición",
            "Opposition",
        ),
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

pub fn bootstrap_element_balance_labels() -> HashMap<(String, String), I18nLabelPair> {
    let mut m = HashMap::new();
    for (code, fr_display, fr_interp) in [
        (
            "fire",
            "Dominante élément Feu",
            "Dominante Feu : élan, intuition et mouvement",
        ),
        (
            "earth",
            "Dominante élément Terre",
            "Dominante Terre : stabilité, réalisme et construction",
        ),
        (
            "air",
            "Dominante élément Air",
            "Dominante Air : idées, échanges et perspective",
        ),
        (
            "water",
            "Dominante élément Eau",
            "Dominante Eau : sensibilité, mémoire et lien intérieur",
        ),
    ] {
        m.insert(
            ("fr".into(), code.into()),
            I18nLabelPair {
                display_label: fr_display.into(),
                interpretive_label: fr_interp.into(),
            },
        );
    }
    m
}

pub fn bootstrap_modality_balance_labels() -> HashMap<(String, String), I18nLabelPair> {
    let mut m = HashMap::new();
    for (code, fr_display, fr_interp) in [
        (
            "cardinal",
            "Dominante cardinale",
            "Dominante cardinale : initiative et ouverture des cycles",
        ),
        (
            "fixed",
            "Dominante fixe",
            "Dominante fixe : persévérance et ancrage",
        ),
        (
            "mutable",
            "Dominante mutable",
            "Dominante mutable : adaptabilité et transition",
        ),
    ] {
        m.insert(
            ("fr".into(), code.into()),
            I18nLabelPair {
                display_label: fr_display.into(),
                interpretive_label: fr_interp.into(),
            },
        );
    }
    m
}

pub fn bootstrap_sect_labels() -> HashMap<(String, String), I18nLabelPair> {
    let mut m = HashMap::new();
    for (code, fr_display, fr_interp) in [
        (
            "day",
            "Thème diurne",
            "Thème diurne : visibilité et rayonnement",
        ),
        (
            "night",
            "Thème nocturne",
            "Thème nocturne : intériorité et réceptivité",
        ),
    ] {
        m.insert(
            ("fr".into(), code.into()),
            I18nLabelPair {
                display_label: fr_display.into(),
                interpretive_label: fr_interp.into(),
            },
        );
    }
    m
}

pub fn bootstrap_house_axis_labels() -> HashMap<(String, String), I18nLabelPair> {
    let mut m = HashMap::new();
    for (code, fr_display, fr_interp, en_display, en_interp) in [
        (
            "private_public",
            "Axe vie privée / vie publique",
            "Axe vie privée / vie publique : tension entre foyer intérieur, exposition et rôle social",
            "Private / public life axis",
            "Private / public life axis: tension between inner home, visibility and social role",
        ),
        (
            "self_relationship",
            "Axe identité / relation",
            "Axe identité / relation : équilibre entre affirmation personnelle et rencontre de l'autre",
            "Identity / relationship axis",
            "Identity / relationship axis: balance between personal assertion and encounter with others",
        ),
        (
            "resources_sharing",
            "Axe ressources personnelles / ressources partagées",
            "Axe ressources personnelles / ressources partagées : circulation entre sécurité propre, confiance et engagement commun",
            "Personal / shared resources axis",
            "Personal / shared resources axis: flow between self-security, trust and shared commitment",
        ),
        (
            "local_distant",
            "Axe proche / lointain",
            "Axe proche / lointain : tension entre environnement immédiat et horizons élargis",
            "Near / distant axis",
            "Near / distant axis: tension between immediate environment and wider horizons",
        ),
        (
            "creation_collective",
            "Axe création personnelle / collectif",
            "Axe création personnelle / collectif : tension entre expression individuelle et idéaux partagés",
            "Personal creation / collective axis",
            "Personal creation / collective axis: tension between individual expression and shared ideals",
        ),
        (
            "control_surrender",
            "Axe maîtrise / lâcher-prise",
            "Axe maîtrise / lâcher-prise : tension entre ordre quotidien et besoin de relâchement intérieur",
            "Control / surrender axis",
            "Control / surrender axis: tension between daily order and inner release",
        ),
    ] {
        m.insert(
            ("fr".into(), code.into()),
            I18nLabelPair {
                display_label: fr_display.into(),
                interpretive_label: fr_interp.into(),
            },
        );
        m.insert(
            ("en".into(), code.into()),
            I18nLabelPair {
                display_label: en_display.into(),
                interpretive_label: en_interp.into(),
            },
        );
    }
    m
}

pub fn bootstrap_house_theme_labels() -> HashMap<(String, u8), I18nLabelPair> {
    let mut m = HashMap::new();
    for (house, fr_display, fr_interp) in [
        (
            2u8,
            "Emphase de la maison 2",
            "Emphase de la maison 2 : ressources, valeur et sécurité",
        ),
        (
            3u8,
            "Emphase de la maison 3",
            "Emphase de la maison 3 : communication et environnement proche",
        ),
        (
            4u8,
            "Emphase de la maison 4",
            "Emphase de la maison 4 : racines, foyer et mémoire",
        ),
        (
            10u8,
            "Emphase de la maison 10",
            "Emphase de la maison 10 : vocation et reconnaissance",
        ),
    ] {
        m.insert(
            ("fr".into(), house),
            I18nLabelPair {
                display_label: fr_display.into(),
                interpretive_label: fr_interp.into(),
            },
        );
    }
    m
}

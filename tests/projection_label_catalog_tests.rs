use std::collections::{BTreeMap, BTreeSet};

use astral_calculator::domain::ProjectionLabelDefinition;
use serde_json::Value;

const PROJECTION_LABELS_JSON: &str =
    include_str!("../json_db/astral_projection_label_definitions.json");

fn projection_label_seed_rows() -> Vec<ProjectionLabelDefinition> {
    serde_json::from_str::<Value>(PROJECTION_LABELS_JSON)
        .expect("projection label seed")
        .get("data")
        .and_then(Value::as_array)
        .expect("projection label data")
        .iter()
        .map(|row| ProjectionLabelDefinition {
            label_family: row["label_family"]
                .as_str()
                .expect("label_family")
                .to_string(),
            label_code: row["label_code"].as_str().expect("label_code").to_string(),
            label_template_en: row["label_template_en"]
                .as_str()
                .expect("label_template_en")
                .to_string(),
            is_active: row["is_active"].as_bool().expect("is_active"),
            sort_order: row["sort_order"].as_i64().expect("sort_order") as i32,
        })
        .collect()
}

#[test]
fn projection_label_seed_loads() {
    let rows = projection_label_seed_rows();
    assert!(!rows.is_empty(), "projection label seed must not be empty");
}

#[test]
fn projection_label_seed_unique_by_family_and_code() {
    let rows = projection_label_seed_rows();
    let mut seen = BTreeSet::new();
    for row in rows {
        assert!(
            seen.insert((row.label_family.clone(), row.label_code.clone())),
            "duplicate projection label definition for ({}, {})",
            row.label_family,
            row.label_code
        );
    }
}

#[test]
fn test_catalog_projection_labels_match_seed_strictly() {
    let mut seed = projection_label_seed_rows();
    let mut catalog = astral_calculator::catalog::test_catalog().projection_label_definitions;
    seed.sort_by(|a, b| {
        a.label_family
            .cmp(&b.label_family)
            .then_with(|| a.sort_order.cmp(&b.sort_order))
            .then_with(|| a.label_code.cmp(&b.label_code))
    });
    catalog.sort_by(|a, b| {
        a.label_family
            .cmp(&b.label_family)
            .then_with(|| a.sort_order.cmp(&b.sort_order))
            .then_with(|| a.label_code.cmp(&b.label_code))
    });

    assert_eq!(catalog.len(), seed.len(), "seed/catalog row count mismatch");
    for (left, right) in catalog.iter().zip(seed.iter()) {
        assert_eq!(left.label_family, right.label_family);
        assert_eq!(left.label_code, right.label_code);
        assert_eq!(left.label_template_en, right.label_template_en);
        assert_eq!(left.is_active, right.is_active);
        assert_eq!(left.sort_order, right.sort_order);
    }
}

#[test]
fn projection_label_seed_contains_expected_family_minimums() {
    let rows = projection_label_seed_rows();
    let mut counts = BTreeMap::<String, usize>::new();
    for row in rows {
        *counts.entry(row.label_family).or_default() += 1;
    }

    let expected = [
        ("angle_display", 4usize),
        ("dynamic_quality", 7usize),
        ("valence", 13),
        ("phase", 3),
        ("motion_display", 3),
        ("reading_slot", 5),
        ("axis_balance", 3),
        ("chart_sect", 2),
        ("hemisphere_area", 3),
        ("dignity_meaning", 5),
        ("condition_variant", 3),
    ];

    for (family, minimum) in expected {
        assert_eq!(
            counts.get(family).copied(),
            Some(minimum),
            "unexpected projection label family coverage for {family}"
        );
    }
}

#[test]
fn projection_label_seed_contains_runtime_axis_balance_codes() {
    let rows = projection_label_seed_rows();
    let labels = rows
        .iter()
        .filter(|row| row.label_family == "axis_balance")
        .map(|row| row.label_code.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        labels,
        BTreeSet::from([
            "primary_house_dominant",
            "secondary_house_dominant",
            "balanced_axis"
        ])
    );
}

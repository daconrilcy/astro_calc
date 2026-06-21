mod refactor_governance_support;

use std::path::Path;

use refactor_governance_support::{collect_rs_files, read, workspace_root};
use serde_json::Value;

#[test]
fn domain_does_not_import_infra_db_models() {
    let root = workspace_root().join("astral_calculator/src/domain");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("crate::infra::db::models"),
            "domain file {} imports infra::db::models",
            file.display()
        );
        assert!(
            !content.contains("crate::infra::"),
            "domain file {} imports infra",
            file.display()
        );
        assert!(
            !content.contains("sqlx::") && !content.contains("FromRow"),
            "domain file {} imports sqlx or derives SQL row bindings; keep DB mapping under infra/db",
            file.display()
        );
    }
}

#[test]
fn non_infra_source_does_not_import_sqlx_models_directly() {
    let root = workspace_root().join("astral_calculator/src");
    for file in collect_rs_files(&root) {
        if file.starts_with(root.join("infra/db")) {
            continue;
        }

        let content = read(&file);
        assert!(
            !content.contains("use crate::infra::db::models"),
            "{} imports crate::infra::db::models directly",
            file.display()
        );
        assert!(
            !content.contains("crate::infra::db::models::"),
            "{} references crate::infra::db::models directly",
            file.display()
        );
    }
}

#[test]
fn business_layers_do_not_use_runtime_db_shortcuts() {
    let root = workspace_root().join("astral_calculator/src");
    let restricted_roots = [
        root.join("domain"),
        root.join("engine"),
        root.join("features/natal"),
        root.join("features/horoscope"),
        root.join("features/simplified"),
    ];

    for restricted_root in restricted_roots {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            for forbidden in ["PgPool", "connect_from_env", "block_on", "run_blocking"] {
                assert!(
                    !content.contains(forbidden),
                    "{} contains forbidden runtime/db shortcut {forbidden}",
                    file.display()
                );
            }
        }
    }
}

#[test]
fn product_features_are_physically_grouped_under_features() {
    let root = workspace_root().join("astral_calculator/src/features");
    for feature in ["natal", "simplified", "horoscope"] {
        let path = root.join(feature);
        assert!(
            path.is_dir(),
            "missing product feature directory {}",
            path.display()
        );
    }
}

#[test]
fn removed_root_feature_modules_do_not_reappear() {
    let root = workspace_root().join("astral_calculator/src");
    for feature in ["natal", "simplified", "horoscope"] {
        let legacy_dir = root.join(feature);
        let legacy_file = root.join(format!("{feature}.rs"));
        assert!(
            !legacy_dir.exists(),
            "removed root feature module {} must not reappear; use astral_calculator::features::{feature}",
            legacy_dir.display()
        );
        assert!(
            !legacy_file.exists(),
            "removed root feature module file {} must not reappear; use astral_calculator::features::{feature}",
            legacy_file.display()
        );
    }
}

#[test]
fn horoscope_supported_objects_seed_matches_runtime_query_contract() {
    let path = workspace_root().join("json_db/horoscope_supported_objects.json");
    let value: Value = serde_json::from_str(&read(&path)).expect("valid horoscope seed json");
    let structure = value["structure"].as_object().expect("structure object");

    for required in ["object_code", "is_enabled", "weight"] {
        assert!(
            structure.contains_key(required),
            "horoscope_supported_objects.json must define {required} for runtime query contract"
        );
    }
    assert!(
        !structure.contains_key("is_enabled_v1"),
        "horoscope_supported_objects.json must not keep stale is_enabled_v1 column"
    );

    for row in value["data"].as_array().expect("data array") {
        assert!(
            row["object_code"].is_string(),
            "object_code must be a string"
        );
        assert!(
            row["is_enabled"].is_boolean(),
            "is_enabled must be a boolean"
        );
        assert!(row["weight"].is_number(), "weight must be a number");
    }
}

#[test]
fn horoscope_signal_theme_mappings_seed_matches_runtime_query_contract() {
    let path = workspace_root().join("json_db/horoscope_signal_theme_mappings.json");
    let value: Value = serde_json::from_str(&read(&path)).expect("valid horoscope seed json");
    let structure = value["structure"].as_object().expect("structure object");

    for required in [
        "mapping_code",
        "match_object",
        "match_aspect",
        "match_natal_target",
        "theme_code",
    ] {
        assert!(
            structure.contains_key(required),
            "horoscope_signal_theme_mappings.json must define {required} for runtime query contract"
        );
    }
    for stale in ["service_code", "priority"] {
        assert!(
            !structure.contains_key(stale),
            "horoscope_signal_theme_mappings.json must not define stale {stale} column"
        );
    }

    for row in value["data"].as_array().expect("data array") {
        assert!(
            row["mapping_code"].is_string(),
            "mapping_code must be a string"
        );
        assert!(
            row["match_object"].is_string(),
            "match_object must be a string"
        );
        assert!(row["theme_code"].is_string(), "theme_code must be a string");
    }
}

#[test]
fn removed_natal_astrology_wrappers_do_not_reappear() {
    let root = workspace_root().join("astral_calculator/src/features/natal");
    for module in ["aspects", "ephemeris"] {
        let wrapper_file = root.join(format!("{module}.rs"));
        let wrapper_dir = root.join(module);
        assert!(
            !wrapper_file.exists(),
            "removed natal astrology wrapper {} must not reappear; use astral_calculator::astrology",
            wrapper_file.display()
        );
        assert!(
            !wrapper_dir.exists(),
            "removed natal astrology wrapper directory {} must not reappear; use astral_calculator::astrology",
            wrapper_dir.display()
        );
    }
}

#[test]
fn root_lib_does_not_export_removed_feature_modules() {
    let root = workspace_root().join("astral_calculator/src");
    let lib = read(&root.join("lib.rs"));
    for feature in ["natal", "simplified", "horoscope"] {
        let root_module_export = format!("pub mod {feature};");
        assert!(
            !lib.contains(&root_module_export),
            "astral_calculator/src/lib.rs must not export removed root module {feature}; use features::{feature}"
        );
    }
}

#[test]
fn feature_facade_does_not_export_alias_payload_or_signals_modules() {
    let features = workspace_root().join("astral_calculator/src/features/mod.rs");
    let content = read(&features);
    for forbidden in ["pub mod payload {", "pub mod signals {"] {
        assert!(
            !content.contains(forbidden),
            "{} still exports deprecated feature alias {forbidden}",
            features.display()
        );
    }
}

#[test]
fn calculator_production_source_does_not_contain_inline_tests() {
    let root = workspace_root().join("astral_calculator/src");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("#[cfg(test)]") && !content.contains("#[test]"),
            "{} contains inline tests; calculator behavior tests belong under root tests/",
            file.display()
        );
    }
}

#[test]
fn canonical_public_feature_paths_compile() {
    let _ = std::any::type_name::<
        astral_calculator::features::natal::application::NatalCalculationService<
            astral_calculator::infra::db::calculation_repository::CalculationRepository,
            astral_calculator::infra::db::catalog_repository::CatalogRepository,
            astral_calculator::infra::db::reference_repository::ReferenceRepository,
            astral_calculator::astrology::ephemeris::SwissEphemerisEngine,
        >,
    >();
    let _ = std::any::type_name::<
        astral_calculator::features::simplified::AstroSimplifiedNatalRequest,
    >();
    let _ = std::any::type_name::<
        astral_calculator::features::horoscope::HoroscopeCalculationRequest,
    >();
    let _ = std::any::type_name::<astral_calculator::astrology::ephemeris::SwissEphemerisEngine>();
    let _detect_aspects: fn(
        &[astral_calculator::domain::ObjectPositionFact],
        &[astral_calculator::domain::AspectDefinition],
    ) -> Vec<astral_calculator::domain::AspectFact> =
        astral_calculator::astrology::aspects::detect_aspects;
}

#[test]
fn natal_application_service_uses_standard_module_declarations() {
    let service_file = workspace_root()
        .join("astral_calculator/src/features/natal/application/natal_calculation_service.rs");
    let content = read(&service_file);
    assert!(
        !content.contains("#[path = "),
        "{} still uses #[path] module assembly",
        service_file.display()
    );
}

#[test]
fn internal_sources_do_not_use_historical_root_aliases() {
    let root = workspace_root().join("astral_calculator/src");
    let forbidden = [
        "crate::catalog",
        "crate::db",
        "crate::facts",
        "crate::aspects",
        "crate::cli",
        "crate::config",
        "crate::dignities",
        "crate::ephemeris",
        "crate::idempotency",
        "astral_calculator::catalog",
        "astral_calculator::db",
        "astral_calculator::facts",
        "astral_calculator::aspects",
        "astral_calculator::cli",
        "astral_calculator::config",
        "astral_calculator::dignities",
        "astral_calculator::ephemeris",
        "astral_calculator::idempotency",
    ];

    for file in collect_rs_files(&root) {
        let content = read(&file);
        for alias in forbidden {
            assert!(
                !content.contains(alias),
                "{} uses deprecated alias {alias}",
                file.display()
            );
        }
    }

    let tests_root = workspace_root().join("tests");
    for file in collect_rs_files(&tests_root) {
        let relative = file
            .strip_prefix(workspace_root())
            .expect("relative test path")
            .to_string_lossy()
            .replace('\\', "/");
        if matches!(
            relative.as_str(),
            "tests/deprecated_root_alias_compat_tests.rs"
                | "tests/refactor_governance_review_tests.rs"
                | "tests/refactor_governance_runtime_tests.rs"
                | "tests/refactor_governance_support.rs"
                | "tests/refactor_governance_tests.rs"
        ) {
            continue;
        }

        let content = read(&file);
        for alias in forbidden {
            assert!(
                !content.contains(alias),
                "{} uses deprecated alias {alias}",
                file.display()
            );
        }
    }
}

#[test]
fn calculator_http_uses_canonical_calculator_imports() {
    let root = workspace_root().join("astral_calculator_http/src");
    let forbidden = [
        "astral_calculator::catalog",
        "astral_calculator::db",
        "astral_calculator::facts",
        "astral_calculator::aspects",
        "astral_calculator::cli",
        "astral_calculator::config",
        "astral_calculator::dignities",
        "astral_calculator::ephemeris",
        "astral_calculator::idempotency",
    ];

    for file in collect_rs_files(&root) {
        let content = read(&file);
        for alias in forbidden {
            assert!(
                !content.contains(alias),
                "{} uses deprecated calculator alias {alias}",
                file.display()
            );
        }
    }
}

#[test]
fn natal_calculation_service_is_split_into_private_submodules() {
    let app_dir = workspace_root().join("astral_calculator/src/features/natal/application");
    for file in [
        "natal_calculation_service.rs",
        "snapshot_loader.rs",
        "reuse_policy.rs",
        "workflow.rs",
        "persisted_position_reuse.rs",
    ] {
        assert!(
            app_dir.join(file).is_file(),
            "missing {}",
            app_dir.join(file).display()
        );
    }
}

#[test]
fn natal_workflow_uses_typed_lifecycle_progress() {
    let workflow =
        workspace_root().join("astral_calculator/src/features/natal/application/workflow.rs");
    let content = read(&workflow);
    for legacy in [
        "\"calculating_facts\"",
        "\"aggregating_signals\"",
        "\"building_payload\"",
    ] {
        assert!(
            !content.contains(legacy),
            "{} still uses legacy string lifecycle state {}",
            workflow.display(),
            legacy
        );
    }
    assert!(content.contains("CalculationProgressState::CalculatingFacts"));
    assert!(content.contains("CalculationProgressState::AggregatingSignals"));
    assert!(content.contains("CalculationProgressState::BuildingPayload"));
}

#[test]
fn simplified_and_horoscope_do_not_import_natal_internals() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [
        root.join("features/simplified"),
        root.join("features/horoscope"),
    ] {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            let legacy_natal_aspects = ["crate::", "natal::", "aspects"].join("");
            let legacy_natal_ephemeris = ["crate::", "natal::", "ephemeris"].join("");
            let natal_feature_internals = ["crate::", "features::", "natal::"].join("");
            assert!(
                !content.contains(&legacy_natal_aspects),
                "{} imports {}",
                file.display(),
                legacy_natal_aspects
            );
            assert!(
                !content.contains(&legacy_natal_ephemeris),
                "{} imports {}",
                file.display(),
                legacy_natal_ephemeris
            );
            assert!(
                !content.contains(&natal_feature_internals),
                "{} imports {} internals",
                file.display(),
                natal_feature_internals
            );
        }
    }
}

#[test]
fn application_layer_does_not_import_feature_modules() {
    let root = workspace_root().join("astral_calculator/src/application");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("crate::features::"),
            "{} imports crate::features::*; application must depend on neutral domain/application records only",
            file.display()
        );
    }
}

#[test]
fn services_do_not_depend_on_reference_catalog_composite_trait() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [
        root.join("engine/application"),
        root.join("features/horoscope/application"),
        root.join("features/simplified/application"),
        root.join("features/simplified/service.rs"),
    ] {
        if restricted_root.is_dir() {
            for file in collect_rs_files(&restricted_root) {
                let content = read(&file);
                assert!(
                    !content.contains("ReferenceCatalog"),
                    "{} still depends on broad ReferenceCatalog; use narrow ports instead",
                    file.display()
                );
            }
            continue;
        }

        let content = read(&restricted_root);
        assert!(
            !content.contains("ReferenceCatalog"),
            "{} still depends on broad ReferenceCatalog; use narrow ports instead",
            restricted_root.display()
        );
    }
}

#[test]
fn simplified_service_does_not_hard_code_reference_system_ids() {
    let path = workspace_root().join("astral_calculator/src/features/simplified/service.rs");
    let content = read(&path);
    for forbidden in [
        "zodiacal_reference_system_id: 1",
        "coordinate_reference_system_id: 1",
    ] {
        assert!(
            !content.contains(forbidden),
            "{} still contains hard-coded canonical id {}",
            path.display(),
            forbidden
        );
    }
}

#[test]
fn non_natal_feature_services_use_shared_transient_chart_seam() {
    let root = workspace_root().join("astral_calculator/src");
    let transient_chart = root.join("application/transient_chart.rs");
    assert!(
        transient_chart.exists(),
        "missing shared transient chart seam {}",
        transient_chart.display()
    );

    for relative in [
        Path::new("features/simplified/service.rs"),
        Path::new("features/horoscope/application/horoscope_service.rs"),
    ] {
        let path = root.join(relative);
        let content = read(&path);
        assert!(
            content.contains("calculate_transient_chart_facts"),
            "{} must use application::transient_chart::calculate_transient_chart_facts",
            path.display()
        );
        assert!(
            !content.contains(".calculate_chart("),
            "{} must not call EphemerisEngine::calculate_chart directly; use the shared transient seam",
            path.display()
        );
    }
}

#[test]
fn horoscope_builder_period_profiles_do_not_decode_included_days_json_in_builder() {
    let path = workspace_root().join("astral_calculator/src/features/horoscope/builders.rs");
    let content = read(&path);
    assert!(
        !content.contains("serde_json::from_value::<Vec<String>>"),
        "{} must consume typed included_days data from the application boundary instead of decoding raw JSON in the builder",
        path.display()
    );
}

#[test]
fn horoscope_repository_keeps_included_days_decode_contextualized_at_adapter_edge() {
    let repository =
        workspace_root().join("astral_calculator/src/infra/db/horoscope_repository.rs");
    let content = read(&repository);
    assert!(
        content.contains("fn decode_included_days("),
        "{} must keep an explicit adapter-edge decoder for included_days",
        repository.display()
    );
    assert!(
        content.contains("serde_json::from_value::<Vec<String>>"),
        "{} must decode SQL JSON included_days into typed day codes exactly at the repository edge",
        repository.display()
    );
    assert!(
        content.contains("RuntimeError::InvalidRuntimeTable")
            && content.contains("period_profile_code")
            && content.contains("included_days invalid"),
        "{} must contextualize invalid included_days rows with the period profile code",
        repository.display()
    );
}

#[test]
fn astrology_module_exists_and_feature_shared_is_not_used_for_astrology() {
    let root = workspace_root().join("astral_calculator/src");
    let astrology_mod = root.join("astrology/mod.rs");
    assert!(
        astrology_mod.exists(),
        "missing {}",
        astrology_mod.display()
    );

    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("features/shared"),
            "{} references forbidden features/shared path",
            file.display()
        );
    }
}

#[test]
fn internal_code_uses_calculate_chart_instead_of_legacy_calculate_natal() {
    let root = workspace_root().join("astral_calculator/src");
    for file in collect_rs_files(&root) {
        let relative = file.strip_prefix(&root).expect("relative source path");
        if relative == Path::new("astrology").join("ephemeris.rs") {
            continue;
        }

        let content = read(&file);
        assert!(
            !content.contains(".calculate_natal("),
            "{} calls legacy EphemerisEngine::calculate_natal",
            file.display()
        );
    }
}

#[test]
fn position_fact_json_shaping_lives_in_domain_chart_facts() {
    let root = workspace_root().join("astral_calculator/src");
    let ephemeris = read(&root.join("astrology/ephemeris.rs"));
    let chart_facts = read(&root.join("domain/chart_facts.rs"));

    for helper in [
        "calculated_position_facts_json",
        "angle_position_facts_json",
    ] {
        assert!(
            !ephemeris.contains(&format!("fn {helper}")),
            "astrology/ephemeris.rs must not own position facts_json shaping helper {helper}"
        );
    }

    for helper in [
        "facts_json_for_calculated_position",
        "facts_json_for_angle_position",
    ] {
        assert!(
            chart_facts.contains(&format!("fn {helper}")),
            "domain/chart_facts.rs must own typed position facts_json helper {helper}"
        );
    }
}

mod refactor_governance_support;

use std::path::Path;

use refactor_governance_support::{collect_rs_files, read, workspace_root};

#[test]
fn calculator_runtime_source_does_not_use_horoscope_fake_calculators() {
    let root = workspace_root().join("astral_calculator/src");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("fake_calculator_"),
            "{} still references a fake horoscope calculator source",
            file.display()
        );
        assert!(
            !content.contains("FAKE_PREMIUM_LOCAL_DATA_STABLE_FOR_TESTS"),
            "{} still exposes fake premium horoscope data in runtime",
            file.display()
        );
    }
}

#[test]
fn shared_astro_math_stays_free_of_domain_types() {
    let path = workspace_root().join("astral_calculator/src/shared/astro_math.rs");
    let content = read(&path);
    assert!(
        !content.contains("crate::domain"),
        "{} imports domain types; move métier-specific geometry under astrology/",
        path.display()
    );
    assert!(
        !content.contains("HouseCuspFact"),
        "{} owns house cusp métier logic; use astrology::house_geometry",
        path.display()
    );
    assert!(
        !content.contains("motion_state_id"),
        "{} resolves canonical motion state ids; use astrology::motion with DB references",
        path.display()
    );
    for forbidden in ["Some(1)", "Some(2)", "Some(3)"] {
        assert!(
            !content.contains(forbidden),
            "{} contains hard-coded canonical motion state id {forbidden}",
            path.display()
        );
    }
}

#[test]
fn application_services_do_not_import_infra_db() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [
        root.join("engine/application"),
        root.join("features/natal/application"),
        root.join("features/horoscope/application"),
        root.join("features/simplified/application"),
    ] {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            assert!(
                !content.contains("crate::infra::db") && !content.contains("infra::db"),
                "{} imports infra::db instead of application ports",
                file.display()
            );
        }
    }
}

#[test]
fn application_services_do_not_import_runtime_facade() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [
        root.join("engine/application"),
        root.join("features/natal/application"),
        root.join("features/horoscope/application"),
        root.join("features/simplified/application"),
    ] {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            assert!(
                !content.contains("crate::runtime") && !content.contains("use crate::runtime::"),
                "{} imports crate::runtime; application services must use canonical feature/application modules",
                file.display()
            );
        }
    }
}

#[test]
fn feature_application_services_do_not_depend_on_engine_application() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [
        root.join("features/natal/application"),
        root.join("features/horoscope/application"),
        root.join("features/simplified/application"),
    ] {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            assert!(
                !content.contains("crate::engine::application"),
                "{} imports crate::engine::application; feature application must not depend on engine/application",
                file.display()
            );
        }
    }
}

#[test]
fn engine_and_horoscope_builders_use_ports_instead_of_infra_db() {
    let root = workspace_root().join("astral_calculator/src");
    for relative in [
        Path::new("engine"),
        Path::new("features/horoscope/builders.rs"),
    ] {
        let path = root.join(relative);
        if path.is_dir() {
            for file in collect_rs_files(&path) {
                let content = read(&file);
                assert!(
                    !content.contains("crate::infra::db") && !content.contains("infra::db"),
                    "{} imports infra::db instead of application ports",
                    file.display()
                );
            }
            continue;
        }
        let content = read(&path);
        assert!(
            !content.contains("crate::infra::db") && !content.contains("infra::db"),
            "{} imports infra::db instead of application ports",
            path.display()
        );
    }
}

#[test]
fn infra_db_uses_canonical_domain_catalog_paths() {
    let root = workspace_root().join("astral_calculator/src/infra/db");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("crate::features::natal::catalog::BasicPayloadCatalog"),
            "{} still imports compatibility path crate::features::natal::catalog::BasicPayloadCatalog instead of crate::domain::BasicPayloadCatalog",
            file.display()
        );
    }
}

#[test]
fn runtime_repository_is_residual_and_not_wrapped_by_repositories() {
    let root = workspace_root().join("astral_calculator/src/infra/db");
    let runtime_repository = root.join("runtime_repository.rs");
    let runtime_repository_text = read(&runtime_repository);
    assert!(
        runtime_repository_text.contains("parse_existing_basic_payload_value")
            && runtime_repository_text.contains("is_stale_basic_payload_shape"),
        "{} must remain the residual runtime lookup home until the dedicated wrapper-pruning slice lands",
        runtime_repository.display()
    );

    for file in collect_rs_files(&root) {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if !name.ends_with("_repository.rs") {
            continue;
        }
        let content = read(&file);
        assert!(
            !content.contains("RuntimeRepository"),
            "{} wraps RuntimeRepository; SQL must live in specialized repositories or internal query modules",
            file.display()
        );
    }

    let runtime_queries = root.join("runtime_queries.rs");
    let runtime_queries_text = read(&runtime_queries);
    for required in [
        "mod calculation;",
        "mod catalog;",
        "mod horoscope;",
        "mod mappers;",
        "mod projection;",
        "mod reference;",
    ] {
        assert!(
            runtime_queries_text.contains(required),
            "{} must stay a thin facade exposing {required}",
            runtime_queries.display()
        );
    }

    for module in [
        "runtime_queries/reference.rs",
        "runtime_queries/catalog.rs",
        "runtime_queries/horoscope.rs",
        "runtime_queries/projection.rs",
        "runtime_queries/calculation.rs",
        "runtime_queries/calculation/reads.rs",
        "runtime_queries/calculation/writes.rs",
    ] {
        assert!(
            root.join(module).exists(),
            "missing split runtime query module {}",
            module
        );
    }

    let calculation_facade = root.join("runtime_queries/calculation.rs");
    let calculation_text = read(&calculation_facade);
    for required in ["mod reads;", "mod writes;"] {
        assert!(
            calculation_text.contains(required),
            "{} must remain a thin calculation query facade exposing {required}",
            calculation_facade.display()
        );
    }
}

#[test]
fn astrology_and_features_do_not_depend_on_shared_astro_math() {
    let root = workspace_root().join("astral_calculator/src");
    for restricted_root in [root.join("astrology"), root.join("features")] {
        for file in collect_rs_files(&restricted_root) {
            let content = read(&file);
            assert!(
                !content.contains("crate::shared::astro_math"),
                "{} imports crate::shared::astro_math; use crate::astrology::angles|zodiac",
                file.display()
            );
        }
    }
}

#[test]
fn runtime_module_stays_a_wiring_facade() {
    let runtime_mod = workspace_root().join("astral_calculator/src/runtime/mod.rs");
    let content = read(&runtime_mod);
    for forbidden in [
        "validate_aspect_definitions",
        "validate_calculation_references",
        "is_current_basic_payload",
        "has_current_rulership_references",
        "parse_existing_basic_payload_value",
    ] {
        assert!(
            !content.contains(forbidden),
            "{} re-exports {forbidden}; keep helper compatibility under runtime::compat",
            runtime_mod.display()
        );
    }
}

#[test]
fn engine_facade_depends_on_capabilities_not_concrete_feature_services() {
    let path =
        workspace_root().join("astral_calculator/src/engine/application/runtime_facade_service.rs");
    let content = read(&path);
    for forbidden in [
        "NatalCalculationService<",
        "SimplifiedNatalService<",
        "HoroscopeService<",
        "use crate::features::natal::application::NatalCalculationService;",
        "use crate::features::simplified::application::SimplifiedNatalService;",
        "use crate::features::horoscope::application::HoroscopeService;",
    ] {
        assert!(
            !content.contains(forbidden),
            "{} still depends on concrete feature service {}",
            path.display(),
            forbidden
        );
    }

    for required in [
        "NatalCalculationCapability",
        "SimplifiedNatalCapability",
        "HoroscopeCapability",
    ] {
        assert!(
            content.contains(required),
            "{} must depend on capability trait {}",
            path.display(),
            required
        );
    }
}

#[test]
fn projection_builder_stays_split_across_named_submodules() {
    let root = workspace_root().join("astral_calculator/src/engine/projection");
    let builder_root = root.join("builder");
    let builder_entry = root.join("builder.rs");
    assert!(builder_root.is_dir(), "missing {}", builder_root.display());
    assert!(
        builder_entry.exists(),
        "missing {}",
        builder_entry.display()
    );

    let builder_text = read(&builder_entry);
    for required in [
        "mod chart;",
        "mod house_axes;",
        "mod identity;",
        "mod keywords;",
        "mod placements;",
        "mod reading_order;",
        "mod relationships;",
        "mod strengths;",
        "mod themes;",
    ] {
        assert!(
            builder_text.contains(required),
            "{} must stay a facade over projection builder submodule {}",
            builder_entry.display(),
            required
        );
    }

    for module in [
        "chart.rs",
        "reading_order.rs",
        "identity.rs",
        "themes.rs",
        "placements.rs",
        "strengths.rs",
        "relationships.rs",
        "house_axes.rs",
        "keywords.rs",
    ] {
        assert!(
            builder_root.join(module).exists(),
            "missing split projection builder module {}",
            module
        );
    }
}

#[test]
fn horoscope_runtime_has_no_derived_calculator_sources() {
    let root = workspace_root().join("astral_calculator/src");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        for forbidden in [
            "derived_daily_calculator_v1",
            "derived_period_calculator_v1",
        ] {
            assert!(
                !content.contains(forbidden),
                "{} contains forbidden synthetic horoscope source {forbidden}",
                file.display()
            );
        }
    }
}

#[test]
fn horoscope_runtime_does_not_embed_supported_object_catalog() {
    let root = workspace_root().join("astral_calculator/src/features/horoscope");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        for forbidden in [
            r#""sun" | "moon" | "mercury" | "venus" | "mars" | "jupiter" | "saturn""#,
            "transit_object_for_slot",
            "period_tone_for",
        ] {
            assert!(
                !content.contains(forbidden),
                "{} embeds horoscope catalog logic forbidden by DB-backed runtime catalog: {forbidden}",
                file.display()
            );
        }
    }
}

#[test]
fn swiss_ephemeris_lock_is_centralized() {
    let root = workspace_root().join("astral_calculator/src");
    let canonical_runtime = root.join("astrology/swisseph_runtime.rs");
    let canonical_runtime_text = read(&canonical_runtime);
    assert!(
        canonical_runtime_text.contains("OnceLock<Mutex"),
        "{} must own the canonical Swiss Ephemeris lock",
        canonical_runtime.display()
    );

    for file in collect_rs_files(&root) {
        if file == canonical_runtime {
            continue;
        }
        let content = read(&file);
        assert!(
            !content.contains("fn swiss_ephemeris_lock") && !content.contains("OnceLock<Mutex"),
            "{} reintroduces a local Swiss Ephemeris lock; use astrology::swisseph_runtime",
            file.display()
        );
    }
}

#[test]
fn horoscope_runtime_has_no_panic_paths() {
    let root = workspace_root().join("astral_calculator/src/features/horoscope");
    for file in collect_rs_files(&root) {
        let content = read(&file);
        assert!(
            !content.contains("panic!"),
            "{} contains panic! in horoscope runtime path",
            file.display()
        );
    }
}

#[test]
fn horoscope_public_period_api_has_no_expect_wrappers() {
    let path = workspace_root().join("astral_calculator/src/features/horoscope/period.rs");
    let content = read(&path);
    assert!(
        !content.contains(".expect("),
        "{} contains expect(...) in public period API path",
        path.display()
    );
}

#[test]
fn horoscope_daily_and_period_use_named_tropical_constant() {
    for relative in [
        "astral_calculator/src/features/horoscope/daily.rs",
        "astral_calculator/src/features/horoscope/period.rs",
    ] {
        let path = workspace_root().join(relative);
        let content = read(&path);
        assert!(
            content.contains("TROPICAL_ZODIACAL_REFERENCE_SYSTEM_CODE"),
            "{} should centralize the tropical zodiacal code in a named constant",
            path.display()
        );
        assert!(
            !content.contains("\"zodiacal_reference_system\": \"tropical\""),
            "{} should not inline the tropical zodiacal code in JSON assembly",
            path.display()
        );
    }
}

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rs_files_recursive(root, &mut files);
    files
}

fn collect_rs_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files_recursive(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn collect_governance_text_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_governance_text_files_recursive(root, &mut files);
    files
}

fn collect_governance_text_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_governance_text_files_recursive(&path, files);
            continue;
        }

        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };

        if matches!(
            extension,
            "rs" | "ps1" | "js" | "md" | "yaml" | "yml" | "toml"
        ) {
            files.push(path);
        }
    }
}

fn allows_legacy_calculator_route_reference(relative_path: &Path) -> bool {
    let path = relative_path.to_string_lossy().replace('\\', "/");
    matches!(
        path.as_str(),
        "astral_calculator_http/src/routes.rs"
            | "contracts/README.md"
            | "contracts/calculator/openapi.yaml"
            | "docs/BASIC_PAYLOAD_IMPLEMENTATION.md"
            | "docs/GUIDE_DEBUTANT_DOCKER.md"
            | "docs/integration_api_contract.md"
            | "docs/integration_api_guide.md"
            | "tests/astral_calculator_http_tests.rs"
            | "tests/refactor_governance_tests.rs"
    ) || path.starts_with("docs/reviews/")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

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
fn canonical_public_feature_paths_compile() {
    let _ = std::any::type_name::<
        astral_calculator::features::natal::application::NatalCalculationService<
            astral_calculator::infra::db::calculation_repository::CalculationRepository,
            astral_calculator::infra::db::catalog_repository::CatalogRepository,
            astral_calculator::infra::db::reference_repository::ReferenceRepository,
            astral_calculator::ephemeris::SwissEphemerisEngine,
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
            let allowed_natal_validate = "crate::features::natal::validate";
            let stripped = content.replace(allowed_natal_validate, "");
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
                !stripped.contains(&natal_feature_internals),
                "{} imports {} internals",
                file.display(),
                natal_feature_internals
            );
        }
    }
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
fn feature_boundary_refactor_reviews_are_closed() {
    let review_root =
        workspace_root().join("docs/reviews/astral_calculator_refactor_feature_boundaries");
    let expected_files = [
        "REV-W00-plan.md",
        "REV-W00-adversarial.md",
        "REV-W00-followup-1.md",
        "REV-W01-plan.md",
        "REV-W01-adversarial.md",
        "REV-W01-followup-1.md",
        "REV-W02-plan.md",
        "REV-W02-adversarial.md",
        "REV-W02-followup-1.md",
        "REV-W03-plan.md",
        "REV-W03-adversarial.md",
        "REV-W03-followup-1.md",
        "REV-W04-plan.md",
        "REV-W04-adversarial.md",
        "REV-W04-followup-1.md",
        "REV-GLOBAL-adversarial.md",
        "REV-IMPLEMENTATION-001-adversarial.md",
        "REV-IMPLEMENTATION-002-adversarial.md",
        "REV-IMPLEMENTATION-003-adversarial.md",
        "REV-IMPLEMENTATION-004-adversarial.md",
        "REV-IMPLEMENTATION-005-adversarial.md",
        "REV-IMPLEMENTATION-006-adversarial.md",
        "REV-IMPLEMENTATION-007-adversarial.md",
        "REV-FINAL.md",
    ];

    for file_name in expected_files {
        let path = review_root.join(file_name);
        assert!(path.exists(), "missing review artifact {}", path.display());
        let content = read(&path);
        assert!(
            content.contains("Statut: closed") || content.contains("Statut final: closed"),
            "{} is not marked closed",
            path.display()
        );
        assert!(
            content.contains("Aucun finding ouvert")
                || content.contains("Findings restants: Aucun"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}

#[test]
fn general_refactor_review_for_physical_features_is_closed() {
    let path = workspace_root()
        .join("docs/reviews/astral_calculator_refactor/REV-PHYSICAL-FEATURES-adversarial.md");
    assert!(path.exists(), "missing review artifact {}", path.display());

    let content = read(&path);
    assert!(
        content.contains("Status: `closed`") || content.contains("Statut: closed"),
        "{} is not marked closed",
        path.display()
    );
    assert!(
        content.contains("Aucun finding ouvert") || content.contains("Findings restants: Aucun"),
        "{} does not record a zero-open-finding state",
        path.display()
    );
}

#[test]
fn root_feature_wrapper_removal_reviews_are_closed() {
    let root = workspace_root();
    for review_path in [
        "docs/reviews/astral_calculator_refactor/REV-ROOT-FEATURE-WRAPPERS-REMOVAL-2026-06-18.md",
        "docs/reviews/astral_calculator_refactor/REV-ROOT-FEATURE-WRAPPERS-REMOVAL-LOOP-001-2026-06-18.md",
        "docs/reviews/astral_calculator_refactor/REV-ROOT-FEATURE-WRAPPERS-REMOVAL-LOOP-002-2026-06-18.md",
    ] {
        let path = root.join(review_path);
        assert!(path.exists(), "missing review artifact {}", path.display());

        let content = read(&path);
        assert!(
            content.contains("Status: `closed`") || content.contains("Statut: closed"),
            "{} is not marked closed",
            path.display()
        );
        assert!(
            content.contains("Aucun finding ouvert")
                || content.contains("Findings restants: Aucun"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}

#[test]
fn internal_calculator_consumers_use_canonical_calculation_routes() {
    let root = workspace_root();
    let scan_roots = [
        "astral_calculator_http",
        "astral_gateway",
        "astral_llm",
        "contracts",
        "docs",
        "scripts",
        "tests",
    ];

    for scan_root in scan_roots {
        for file in collect_governance_text_files(&root.join(scan_root)) {
            let relative = file.strip_prefix(&root).expect("relative workspace path");
            let content = read(&file);
            if content.contains("/v1/calculations")
                && !allows_legacy_calculator_route_reference(relative)
            {
                panic!(
                    "{} references legacy calculator routes; internal consumers must use /v1/internal/calculations/*",
                    relative.display()
                );
            }
        }
    }
}

#[test]
fn calculator_internal_consumer_refactor_reviews_are_closed() {
    let root = workspace_root();
    for review_path in [
        "docs/reviews/astral_calculator_refactor/REV-CALCULATOR-INTERNAL-CONSUMERS-2026-06-17.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-CALCULATOR-INTERNAL-CONSUMERS-2026-06-17.md",
    ] {
        let path = root.join(review_path);
        assert!(path.exists(), "missing review artifact {}", path.display());
        let content = read(&path);
        assert!(
            content.contains("Statut: closed") || content.contains("Status: `closed`"),
            "{} is not marked closed",
            path.display()
        );
        assert!(
            content.contains("Aucun finding ouvert"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}

#[test]
fn calculator_http_rename_has_no_active_legacy_service_name() {
    let root = workspace_root();
    let removed_name = ["astral", "calculator", "api"].join("_");
    let scan_roots = [
        "astral_calculator_http",
        "astral_gateway",
        "astral_llm",
        "contracts",
        "docker",
        "docs",
        "scripts",
        "tests",
    ];

    for scan_root in scan_roots {
        for file in collect_governance_text_files(&root.join(scan_root)) {
            let relative = file.strip_prefix(&root).expect("relative workspace path");
            let path = relative.to_string_lossy().replace('\\', "/");
            if path.starts_with("docs/reviews/") {
                continue;
            }
            let content = read(&file);
            assert!(
                !content.contains(&removed_name),
                "{} still references removed service/crate name {}",
                relative.display(),
                removed_name
            );
        }
    }

    for relative in [
        "Cargo.toml",
        "docker-compose.yml",
        ".env.example",
        "AGENTS.md",
    ] {
        let path = root.join(relative);
        if path.exists() {
            let content = read(&path);
            assert!(
                !content.contains(&removed_name),
                "{relative} still references removed service/crate name {removed_name}"
            );
        }
    }
}

#[test]
fn gateway_does_not_depend_on_internal_calculator_or_llm_crates() {
    let manifest = read(&workspace_root().join("astral_gateway/Cargo.toml"));
    for forbidden in [
        "astral_calculator",
        "astral_llm_application",
        "astral_llm_domain",
        "astral_llm_infra",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "astral_gateway/Cargo.toml still depends on forbidden crate {forbidden}"
        );
    }
}

#[test]
fn gateway_does_not_embed_canonical_reference_data() {
    let root = workspace_root().join("astral_gateway");
    for file in collect_governance_text_files(&root) {
        let relative = file.strip_prefix(&root).expect("relative gateway path");
        let content = read(&file);
        for forbidden in ["json_db", "include_str!"] {
            assert!(
                !content.contains(forbidden),
                "astral_gateway/{} embeds canonical reference data via {forbidden}",
                relative.display()
            );
        }
    }
}

#[test]
fn calculator_http_rename_and_gateway_decoupling_reviews_are_closed() {
    let root = workspace_root();
    for review_path in [
        "docs/reviews/astral_calculator_refactor/REV-CALCULATOR-HTTP-RENAME-2026-06-17.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-GATEWAY-DECOUPLING-2026-06-17.md",
    ] {
        let path = root.join(review_path);
        assert!(path.exists(), "missing review artifact {}", path.display());
        let content = read(&path);
        assert!(
            content.contains("Statut: closed") || content.contains("Status: `closed`"),
            "{} is not marked closed",
            path.display()
        );
        assert!(
            content.contains("Aucun finding ouvert"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}

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
fn runtime_repository_is_residual_and_not_wrapped_by_repositories() {
    let root = workspace_root().join("astral_calculator/src/infra/db");
    let runtime_repository = root.join("runtime_repository.rs");
    let line_count = read(&runtime_repository).lines().count();
    assert!(
        line_count <= 80,
        "{} has {line_count} lines; keep runtime_repository.rs as residual helper only",
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
    let runtime_queries_line_count = read(&runtime_queries).lines().count();
    assert!(
        runtime_queries_line_count <= 260,
        "{} has {runtime_queries_line_count} lines; keep it as a thin query-module facade",
        runtime_queries.display()
    );

    for module in [
        "runtime_queries/reference.rs",
        "runtime_queries/catalog.rs",
        "runtime_queries/horoscope.rs",
        "runtime_queries/projection.rs",
        "runtime_queries/calculation.rs",
    ] {
        assert!(
            root.join(module).exists(),
            "missing split runtime query module {}",
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
fn calculator_refactor_plan_reviews_are_closed() {
    let root = workspace_root();
    for review_path in [
        "docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-REAL-DAILY-adversarial.md",
        "docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-REAL-DAILY-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-ASTROLOGY-TRANSITS-adversarial.md",
        "docs/reviews/astral_calculator_refactor/REV-ASTROLOGY-TRANSITS-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-APPLICATION-PORTS-adversarial.md",
        "docs/reviews/astral_calculator_refactor/REV-APPLICATION-PORTS-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-SHARED-ASTRO-MATH-adversarial.md",
        "docs/reviews/astral_calculator_refactor/REV-SHARED-ASTRO-MATH-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-RUNTIME-REPOSITORY-SPLIT-adversarial.md",
        "docs/reviews/astral_calculator_refactor/REV-RUNTIME-REPOSITORY-SPLIT-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-CANONICAL-CATALOG-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-HOROSCOPE-DERIVED-FALLBACKS-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-GLOBAL-FINDINGS-CORRECTION-2026-06-18.md",
        "docs/reviews/astral_calculator_refactor/REV-GLOBAL-FINDINGS-CORRECTION-LOOP-001-2026-06-18.md",
    ] {
        let path = root.join(review_path);
        assert!(path.exists(), "missing review artifact {}", path.display());
        let content = read(&path);
        assert!(
            content.contains("Statut: closed") || content.contains("Status: `closed`"),
            "{} is not marked closed",
            path.display()
        );
        assert!(
            content.contains("Aucun finding ouvert"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}

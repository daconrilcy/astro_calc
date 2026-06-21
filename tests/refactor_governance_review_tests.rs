mod refactor_governance_support;

use refactor_governance_support::{
    allows_legacy_calculator_route_reference, collect_governance_text_files, read, workspace_root,
};

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
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PORTS-BUILDERS-FAILFAST-2026-06-19.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PORTS-BUILDERS-FAILFAST-2026-06-19-followup-1.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PORTS-BUILDERS-FAILFAST-2026-06-19-followup-2.md",
        "docs/reviews/astral_calculator_refactor/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19.md",
        "docs/reviews/astral_calculator_refactor/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19-followup-2.md",
        "docs/reviews/astral_calculator_refactor/REV-REFERENCE-PORT-TIGHTENING-2026-06-19.md",
        "docs/reviews/astral_calculator_refactor/REV-REFERENCE-PORT-TIGHTENING-2026-06-19-followup-1.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19-followup-1.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-PROJECTION-PORTS-SIMPLIFIED-2026-06-19-followup-2.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-REFERENCE-PORT-TIGHTENING-2026-06-19.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-REFERENCE-PORT-TIGHTENING-2026-06-19-followup-1.md",
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
        "docs/reviews/astral_calculator_refactor/REV-PORTS-BUILDERS-FAILFAST-2026-06-19.md",
        "docs/reviews/astral_calculator_refactor/REV-PORTS-BUILDERS-FAILFAST-2026-06-19-followup-1.md",
        "docs/reviews/astral_calculator_refactor/REV-PORTS-BUILDERS-FAILFAST-2026-06-19-followup-2.md",
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
fn governance_split_reviews_are_closed() {
    let root = workspace_root();
    for review_path in [
        "docs/reviews/astral_calculator_refactor/REV-GOVERNANCE-SPLIT-2026-06-21.md",
        "docs/reviews/astral_calculator_refactor_feature_boundaries/REV-GOVERNANCE-SPLIT-2026-06-21.md",
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
            content.contains("Aucun finding ouvert")
                || content.contains("Findings restants: Aucun"),
            "{} does not record a zero-open-finding state",
            path.display()
        );
    }
}

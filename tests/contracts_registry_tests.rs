use std::{fs, path::PathBuf};

use astral_contracts::{
    horoscope_service_descriptor, NatalProductCode, NatalVariant, OrchestrationMode, ProductTier,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_SERVICE_DESCRIPTORS,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn natal_product_codes_cover_all_variant_tier_pairs() {
    let cases = [
        (
            NatalVariant::Simplified,
            ProductTier::Free,
            "natal_simplified_free",
        ),
        (
            NatalVariant::Simplified,
            ProductTier::Basic,
            "natal_simplified_basic",
        ),
        (
            NatalVariant::Simplified,
            ProductTier::Premium,
            "natal_simplified_premium",
        ),
        (NatalVariant::Full, ProductTier::Free, "natal_full_free"),
        (NatalVariant::Full, ProductTier::Basic, "natal_full_basic"),
        (
            NatalVariant::Full,
            ProductTier::Premium,
            "natal_full_premium",
        ),
    ];

    for (variant, tier, expected) in cases {
        let code = NatalProductCode::from_parts(variant, tier);
        assert_eq!(code.as_str(), expected);
    }
}

#[test]
fn horoscope_descriptor_lookup_matches_expected_contracts() {
    let daily = horoscope_service_descriptor(HOROSCOPE_FREE_DAILY_SERVICE_CODE).expect("daily");
    assert_eq!(
        daily.orchestration_mode,
        OrchestrationMode::CalculatorThenLlm
    );
    assert_eq!(
        daily.contracts.public_request_contract,
        "horoscope_daily_request_v2"
    );
    assert_eq!(
        daily.contracts.calculator_request_contract,
        "horoscope_calculation_request"
    );

    let period = horoscope_service_descriptor(HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE)
        .expect("period");
    assert_eq!(
        period.contracts.public_request_contract,
        "horoscope_period_request_v2"
    );
    assert_eq!(
        period.contracts.llm_request_contract,
        "horoscope_period_writer_request"
    );
}

#[test]
fn horoscope_registry_is_unique_and_complete_for_v2_contracts() {
    let expected = [
        "horoscope_daily_request_v2",
        "horoscope_period_request_v2",
        "horoscope_response",
        "horoscope_period_response",
    ];
    let service_codes = HOROSCOPE_SERVICE_DESCRIPTORS
        .iter()
        .map(|descriptor| descriptor.service_code)
        .collect::<std::collections::HashSet<_>>();

    assert_eq!(service_codes.len(), HOROSCOPE_SERVICE_DESCRIPTORS.len());
    for contract in expected {
        assert!(
            HOROSCOPE_SERVICE_DESCRIPTORS.iter().any(|descriptor| {
                descriptor.contracts.public_request_contract == contract
                    || descriptor.contracts.public_response_contract == contract
            }),
            "missing expected contract mapping for {contract}"
        );
    }
}

#[test]
fn common_and_public_contract_files_exist_and_parse() {
    let root = repo_root();
    let files = [
        root.join("contracts/common/request_context_common_v1.schema.json"),
        root.join("contracts/common/location_common_v1.schema.json"),
        root.join("contracts/common/birth_input_common_v1.schema.json"),
        root.join("contracts/public/natal_reading_request_v2.schema.json"),
        root.join("contracts/public/natal_reading_response_v2.schema.json"),
    ];

    for path in files {
        assert!(path.is_file(), "missing schema: {}", path.display());
        let raw = fs::read_to_string(&path).expect("read schema");
        let _: serde_json::Value = serde_json::from_str(&raw).expect("valid schema json");
    }
}

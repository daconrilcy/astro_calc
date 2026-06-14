use std::collections::HashSet;

#[test]
fn horoscope_service_codes_are_unique_in_shared_registry() {
    let mut seen = HashSet::new();
    for descriptor in astral_contracts::HOROSCOPE_SERVICE_DESCRIPTORS {
        assert!(
            seen.insert(descriptor.service_code),
            "duplicate service code: {}",
            descriptor.service_code
        );
    }
}

#[test]
fn service_descriptor_contract_mapping_stays_directionally_correct() {
    let period = astral_contracts::horoscope_service_descriptor(
        astral_contracts::HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    )
    .expect("descriptor");

    assert_eq!(
        period.contracts.calculator_request_contract,
        "horoscope_period_calculation_request"
    );
    assert_eq!(
        period.contracts.public_response_contract,
        "horoscope_period_response"
    );
}

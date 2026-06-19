use astral_calculator::application::ports::{
    HoroscopeBuilderCatalog, HoroscopePeriodProfile, HoroscopeScanProfileDefinition,
    HoroscopeServiceProfile, HoroscopeTimeSlotProfile,
};
use astral_calculator::features::horoscope::{
    build_horoscope_daily_calculation_request_from_public,
    build_horoscope_period_calculation_request_from_public,
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
};
use astral_calculator::shared::error::RuntimeError;
use async_trait::async_trait;
use serde_json::json;

#[derive(Clone)]
struct FakeHoroscopeBuilderCatalog {
    services: Vec<HoroscopeServiceProfile>,
    slots: Vec<HoroscopeTimeSlotProfile>,
    period_profiles: Vec<HoroscopePeriodProfile>,
    scan_profiles: Vec<HoroscopeScanProfileDefinition>,
}

impl FakeHoroscopeBuilderCatalog {
    fn seeded() -> Self {
        Self {
            services: vec![
                HoroscopeServiceProfile {
                    service_code: HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE.to_string(),
                    house_system_code: None,
                    period_profile_code: None,
                    scan_profile_code: None,
                },
                HoroscopeServiceProfile {
                    service_code: HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE.to_string(),
                    house_system_code: Some("placidus".to_string()),
                    period_profile_code: None,
                    scan_profile_code: None,
                },
                HoroscopeServiceProfile {
                    service_code: HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE.to_string(),
                    house_system_code: None,
                    period_profile_code: Some("next_7_days".to_string()),
                    scan_profile_code: Some("daily_noon_7_days".to_string()),
                },
            ],
            slots: vec![
                HoroscopeTimeSlotProfile {
                    service_code: HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE.to_string(),
                    slot_code: "all_day".to_string(),
                    start_local_time: "00:00".to_string(),
                    end_local_time: "23:59".to_string(),
                    reference_local_time: "12:00".to_string(),
                    sort_order: 1,
                },
                HoroscopeTimeSlotProfile {
                    service_code: HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE.to_string(),
                    slot_code: "morning".to_string(),
                    start_local_time: "08:00".to_string(),
                    end_local_time: "10:00".to_string(),
                    reference_local_time: "09:00".to_string(),
                    sort_order: 2,
                },
                HoroscopeTimeSlotProfile {
                    service_code: HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE.to_string(),
                    slot_code: "early".to_string(),
                    start_local_time: "06:00".to_string(),
                    end_local_time: "08:00".to_string(),
                    reference_local_time: "07:00".to_string(),
                    sort_order: 1,
                },
            ],
            period_profiles: vec![HoroscopePeriodProfile {
                period_profile_code: "next_7_days".to_string(),
                resolution_strategy: "anchor_forward_days".to_string(),
                duration_days: Some(7),
                week_offset: None,
                included_days: Some(json!([])),
                is_enabled: true,
                sort_order: 1,
            }],
            scan_profiles: vec![HoroscopeScanProfileDefinition {
                scan_profile_code: "daily_noon_7_days".to_string(),
                granularity: "daily".to_string(),
                reference_time_local: "12:00".to_string(),
                expected_snapshots_per_day: 1,
                is_enabled: true,
            }],
        }
    }
}

#[async_trait]
impl HoroscopeBuilderCatalog for FakeHoroscopeBuilderCatalog {
    async fn horoscope_service_profiles(
        &self,
    ) -> Result<Vec<HoroscopeServiceProfile>, RuntimeError> {
        Ok(self.services.clone())
    }

    async fn horoscope_time_slot_profiles(
        &self,
    ) -> Result<Vec<HoroscopeTimeSlotProfile>, RuntimeError> {
        Ok(self.slots.clone())
    }

    async fn astral_time_period_profiles(
        &self,
    ) -> Result<Vec<HoroscopePeriodProfile>, RuntimeError> {
        Ok(self.period_profiles.clone())
    }

    async fn horoscope_scan_profiles(
        &self,
    ) -> Result<Vec<HoroscopeScanProfileDefinition>, RuntimeError> {
        Ok(self.scan_profiles.clone())
    }
}

#[tokio::test]
async fn free_daily_builder_keeps_public_surface_minimal() {
    let repository = FakeHoroscopeBuilderCatalog::seeded();
    let request = build_horoscope_daily_calculation_request_from_public(
        &repository,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1",
            "location": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "label": "Paris"
            }
        }),
    )
    .await
    .expect("daily request");

    assert_eq!(request.contract_version, "horoscope_calculation_request");
    assert_eq!(
        request.service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
    );
    assert!(request.location.is_none());
    assert!(request.house_system_code.is_none());
    assert!(request.slot_profile_code.is_none());
    assert!(request.calculation_features.is_empty());
    assert_eq!(request.slots.len(), 1);
}

#[tokio::test]
async fn premium_daily_builder_requires_location_and_enables_local_features() {
    let repository = FakeHoroscopeBuilderCatalog::seeded();
    let err = build_horoscope_daily_calculation_request_from_public(
        &repository,
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1"
        }),
    )
    .await
    .expect_err("location required");
    assert_eq!(err, "HOROSCOPE_LOCATION_REQUIRED");

    let request = build_horoscope_daily_calculation_request_from_public(
        &repository,
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1",
            "location": {
                "latitude": 48.8566,
                "longitude": 2.3522,
                "label": "Paris"
            }
        }),
    )
    .await
    .expect("premium daily request");

    assert_eq!(request.slot_profile_code.as_deref(), Some("daily_2h_slots"));
    assert!(request.location.is_some());
    assert_eq!(request.house_system_code.as_deref(), Some("placidus"));
    assert_eq!(request.slots[0].slot_code, "early");
    assert_eq!(request.slots[1].slot_code, "morning");
    assert!(request
        .calculation_features
        .iter()
        .any(|feature| feature == "local_chart"));
}

#[tokio::test]
async fn daily_builder_rejects_invalid_timezone_and_service_code() {
    let repository = FakeHoroscopeBuilderCatalog::seeded();
    let invalid_timezone = build_horoscope_daily_calculation_request_from_public(
        &repository,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
        &json!({
            "date": "2026-06-14",
            "timezone": "Bad/Timezone",
            "chart_calculation_id": "chart-1"
        }),
    )
    .await
    .expect_err("timezone must fail");
    assert!(invalid_timezone.contains("timezone"));

    let invalid_service = build_horoscope_daily_calculation_request_from_public(
        &repository,
        "legacy_sync_daily",
        &json!({
            "date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-1"
        }),
    )
    .await
    .expect_err("service must fail");
    assert_eq!(invalid_service, "HOROSCOPE_SERVICE_NOT_IMPLEMENTED");
}

#[tokio::test]
async fn period_builder_creates_canonical_scan_plan_for_v2_service() {
    let repository = FakeHoroscopeBuilderCatalog::seeded();
    let request = build_horoscope_period_calculation_request_from_public(
        &repository,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &json!({
            "anchor_date": "2026-06-14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-7"
        }),
    )
    .await
    .expect("period request");

    assert_eq!(
        request.contract_version,
        "horoscope_period_calculation_request"
    );
    assert_eq!(
        request.service_code,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    assert_eq!(request.scan_plan.scan_profile_code, "daily_noon_7_days");
    assert_eq!(request.scan_plan.snapshot_count, 7);
    assert_eq!(request.scan_plan.snapshots.len(), 7);
    assert_eq!(
        request.period_resolution["included_dates"]
            .as_array()
            .expect("included_dates")
            .len(),
        7
    );
}

#[tokio::test]
async fn period_builder_rejects_invalid_anchor_date() {
    let repository = FakeHoroscopeBuilderCatalog::seeded();
    let err = build_horoscope_period_calculation_request_from_public(
        &repository,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
        &json!({
            "anchor_date": "2026/06/14",
            "timezone": "Europe/Paris",
            "chart_calculation_id": "chart-7"
        }),
    )
    .await
    .expect_err("anchor_date must fail");

    assert_eq!(err, "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED");
}

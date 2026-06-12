use std::fs;

use astral_time_window::{
    PeriodProfileDefinition, PeriodWindowError, PeriodWindowRequest, PeriodWindowResolver,
};
use jsonschema::JSONSchema;
use serde_json::{json, Value};

fn resolver() -> PeriodWindowResolver {
    PeriodWindowResolver::new(seed_profiles())
}

fn seed_profiles() -> Vec<PeriodProfileDefinition> {
    let raw = fs::read_to_string("../json_db/astral_time_period_profiles.json")
        .expect("time period profile seed should exist");
    let value: Value = serde_json::from_str(&raw).expect("time period profile seed should be JSON");
    serde_json::from_value(value["data"].clone()).expect("seed profiles should deserialize")
}

fn request(period_profile_code: &str, anchor_date: &str) -> PeriodWindowRequest {
    PeriodWindowRequest {
        period_profile_code: period_profile_code.to_string(),
        anchor_date: anchor_date.to_string(),
        timezone: "Europe/Paris".to_string(),
        custom_start_date: None,
        custom_end_date: None,
    }
}

fn validate_with_schema(value: &Value, schema_path: &str) -> Vec<String> {
    let raw = fs::read_to_string(schema_path).expect("schema should exist");
    let schema: Value = serde_json::from_str(&raw).expect("schema should be JSON");
    let compiled = JSONSchema::options()
        .compile(&schema)
        .expect("schema should compile");
    compiled
        .validate(value)
        .err()
        .map(|errors| errors.map(|error| error.to_string()).collect())
        .unwrap_or_default()
}

fn profile(period_profile_code: &str) -> PeriodProfileDefinition {
    seed_profiles()
        .into_iter()
        .find(|profile| profile.period_profile_code == period_profile_code)
        .unwrap_or_else(|| panic!("missing profile {period_profile_code}"))
}

#[test]
fn next_7_days_resolves_from_anchor_inclusive_to_end_exclusive() {
    let resolved = resolver()
        .resolve(&request("next_7_days", "2026-06-07"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-07 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-14 00:00:00"
    );
    assert_eq!(resolved.timezone, "Europe/Paris");
    assert_eq!(resolved.duration_days, 7);
    assert!(resolved.end_exclusive);
    assert!(resolved.included_days.is_empty());
}

#[test]
fn next_7_days_exposes_included_dates_from_resolved_window() {
    let resolved = resolver()
        .resolve(&request("next_7_days", "2026-06-07"))
        .expect("window should resolve");

    assert_eq!(
        resolved.included_dates(),
        [
            "2026-06-07",
            "2026-06-08",
            "2026-06-09",
            "2026-06-10",
            "2026-06-11",
            "2026-06-12",
            "2026-06-13"
        ]
    );
}

#[test]
fn iso_week_and_workweek_expose_included_dates_from_resolved_window() {
    let week = resolver()
        .resolve(&request("current_week_monday_sunday", "2026-06-03"))
        .expect("week should resolve");
    let workweek = resolver()
        .resolve(&request("current_workweek_monday_friday", "2026-06-07"))
        .expect("workweek should resolve");

    assert_eq!(
        week.included_dates(),
        [
            "2026-06-01",
            "2026-06-02",
            "2026-06-03",
            "2026-06-04",
            "2026-06-05",
            "2026-06-06",
            "2026-06-07"
        ]
    );
    assert_eq!(
        workweek.included_dates(),
        [
            "2026-06-01",
            "2026-06-02",
            "2026-06-03",
            "2026-06-04",
            "2026-06-05"
        ]
    );
}

#[test]
fn resolved_window_exposes_canonical_utc_boundaries() {
    let resolved = resolver()
        .resolve(&request("next_7_days", "2026-06-07"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_utc().unwrap(),
        "2026-06-06T22:00:00+00:00"
    );
    assert_eq!(
        resolved.end_datetime_utc().unwrap(),
        "2026-06-13T22:00:00+00:00"
    );
}

#[test]
fn iso_week_duration_comes_from_profile_definition() {
    let mut profile = profile("current_week_monday_sunday");
    profile.duration_days = Some(3);
    let resolver = PeriodWindowResolver::new([profile]);

    let resolved = resolver
        .resolve(&request("current_week_monday_sunday", "2026-06-03"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-01 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-04 00:00:00"
    );
    assert_eq!(resolved.duration_days, 3);
}

#[test]
fn invalid_included_day_from_profile_definition_is_rejected() {
    let mut profile = profile("current_workweek_monday_friday");
    profile.included_days.push("funday".into());
    let resolver = PeriodWindowResolver::new([profile]);

    let err = resolver
        .resolve(&request("current_workweek_monday_friday", "2026-06-07"))
        .expect_err("invalid profile should fail");

    assert_eq!(
        err,
        PeriodWindowError::InvalidProfileDefinition {
            profile_code: "current_workweek_monday_friday".into(),
            reason: "invalid included day funday".into()
        }
    );
}

#[test]
fn duplicate_included_day_from_profile_definition_is_rejected() {
    let mut profile = profile("current_workweek_monday_friday");
    profile.included_days.push("monday".into());
    let resolver = PeriodWindowResolver::new([profile]);

    let err = resolver
        .resolve(&request("current_workweek_monday_friday", "2026-06-07"))
        .expect_err("invalid profile should fail");

    assert_eq!(
        err,
        PeriodWindowError::InvalidProfileDefinition {
            profile_code: "current_workweek_monday_friday".into(),
            reason: "duplicate included day monday".into()
        }
    );
}

#[test]
fn current_workweek_from_sunday_resolves_to_same_iso_week_monday_to_saturday() {
    let resolved = resolver()
        .resolve(&request("current_workweek_monday_friday", "2026-06-07"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-01 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-06 00:00:00"
    );
    assert_eq!(resolved.duration_days, 5);
    assert_eq!(
        resolved.included_days,
        ["monday", "tuesday", "wednesday", "thursday", "friday"]
    );
}

#[test]
fn current_week_from_wednesday_resolves_monday_to_next_monday() {
    let resolved = resolver()
        .resolve(&request("current_week_monday_sunday", "2026-06-03"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-01 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-08 00:00:00"
    );
    assert_eq!(resolved.duration_days, 7);
    assert_eq!(
        resolved.included_days,
        [
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday"
        ]
    );
}

#[test]
fn next_workweek_from_monday_uses_following_monday() {
    let resolved = resolver()
        .resolve(&request("next_workweek_monday_friday", "2026-06-01"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-08 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-13 00:00:00"
    );
    assert_eq!(resolved.duration_days, 5);
}

#[test]
fn next_workweek_from_sunday_uses_following_monday() {
    let resolved = resolver()
        .resolve(&request("next_workweek_monday_friday", "2026-06-07"))
        .expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-08 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-13 00:00:00"
    );
}

#[test]
fn custom_date_range_is_inclusive_input_normalized_to_exclusive_end() {
    let mut request = request("custom_date_range", "2026-06-07");
    request.custom_start_date = Some("2026-06-10".into());
    request.custom_end_date = Some("2026-06-12".into());

    let resolved = resolver().resolve(&request).expect("window should resolve");

    assert_eq!(
        resolved.start_datetime_local.to_string(),
        "2026-06-10 00:00:00"
    );
    assert_eq!(
        resolved.end_datetime_local.to_string(),
        "2026-06-13 00:00:00"
    );
    assert_eq!(resolved.duration_days, 3);
    assert_eq!(
        resolved.included_dates(),
        ["2026-06-10", "2026-06-11", "2026-06-12"]
    );
}

#[test]
fn invalid_timezone_is_rejected() {
    let mut request = request("day", "2026-06-07");
    request.timezone = "Europe/Atlantis".into();

    let err = resolver()
        .resolve(&request)
        .expect_err("timezone should fail");

    assert_eq!(
        err,
        PeriodWindowError::InvalidTimezone("Europe/Atlantis".into())
    );
}

#[test]
fn invalid_anchor_date_is_rejected() {
    let err = resolver()
        .resolve(&request("day", "2026-99-07"))
        .expect_err("date should fail");

    assert_eq!(
        err,
        PeriodWindowError::InvalidDate {
            field: "anchor_date",
            value: "2026-99-07".into()
        }
    );
}

#[test]
fn unknown_profile_is_rejected() {
    let err = resolver()
        .resolve(&request("unknown", "2026-06-07"))
        .expect_err("profile should fail");

    assert_eq!(err, PeriodWindowError::UnknownProfile("unknown".into()));
}

#[test]
fn custom_date_range_requires_both_custom_dates() {
    let err = resolver()
        .resolve(&request("custom_date_range", "2026-06-07"))
        .expect_err("custom dates should fail");

    assert_eq!(err, PeriodWindowError::MissingCustomDateRange);
}

#[test]
fn custom_date_range_rejects_end_before_start() {
    let mut request = request("custom_date_range", "2026-06-07");
    request.custom_start_date = Some("2026-06-12".into());
    request.custom_end_date = Some("2026-06-10".into());

    let err = resolver()
        .resolve(&request)
        .expect_err("custom date order should fail");

    assert_eq!(err, PeriodWindowError::InvalidCustomDateRange);
}

#[test]
fn request_and_response_examples_match_public_schemas() {
    let request_value = json!({
        "period_profile_code": "next_7_days",
        "anchor_date": "2026-06-07",
        "timezone": "Europe/Paris"
    });
    let response_value = serde_json::to_value(
        resolver()
            .resolve(&serde_json::from_value(request_value.clone()).expect("request should parse"))
            .expect("window should resolve"),
    )
    .expect("response should serialize");

    let request_errors = validate_with_schema(
        &request_value,
        "../contracts/common/period_window_request_v1.schema.json",
    );
    let response_errors = validate_with_schema(
        &response_value,
        "../contracts/common/period_window_response_v1.schema.json",
    );

    assert!(
        request_errors.is_empty(),
        "request schema errors:\n{}",
        request_errors.join("\n")
    );
    assert!(
        response_errors.is_empty(),
        "response schema errors:\n{}",
        response_errors.join("\n")
    );
}

#[test]
fn custom_date_range_request_schema_requires_custom_dates() {
    let invalid = json!({
        "period_profile_code": "custom_date_range",
        "anchor_date": "2026-06-07",
        "timezone": "Europe/Paris"
    });
    let valid = json!({
        "period_profile_code": "custom_date_range",
        "anchor_date": "2026-06-07",
        "timezone": "Europe/Paris",
        "custom_start_date": "2026-06-10",
        "custom_end_date": "2026-06-12"
    });

    assert!(
        !validate_with_schema(
            &invalid,
            "../contracts/common/period_window_request_v1.schema.json"
        )
        .is_empty(),
        "schema should reject custom_date_range without custom dates"
    );
    assert!(
        validate_with_schema(
            &valid,
            "../contracts/common/period_window_request_v1.schema.json"
        )
        .is_empty(),
        "schema should accept complete custom_date_range"
    );
}

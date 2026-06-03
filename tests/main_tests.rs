use chrono::{TimeZone, Utc};

use astral_calculator::cli::{
    cli_options_from_args, output_mode_from_args, output_contract_from_env, root_output_dir,
    timestamped_output_filename, OutputContract, OutputMode,
};
use std::path::Path;

#[test]
fn output_mode_defaults_to_stdout() {
    assert_eq!(
        output_mode_from_args(Vec::<String>::new(), OutputMode::Stdout).unwrap(),
        OutputMode::Stdout
    );
}

#[test]
fn output_mode_accepts_file_flag() {
    assert_eq!(
        output_mode_from_args(["--file".to_string()], OutputMode::Stdout).unwrap(),
        OutputMode::File
    );
}

#[test]
fn output_mode_can_default_to_file_from_env() {
    assert_eq!(
        output_mode_from_args(Vec::<String>::new(), OutputMode::File).unwrap(),
        OutputMode::File
    );
}

#[test]
fn timestamped_output_filename_is_json_and_filesystem_safe() {
    let datetime = Utc.with_ymd_and_hms(2026, 6, 2, 12, 34, 56).unwrap();

    assert_eq!(
        timestamped_output_filename(datetime, OutputContract::AuditOnly),
        "basic_payload_20260602_123456.json"
    );
    assert_eq!(
        timestamped_output_filename(datetime, OutputContract::Engine),
        "astro_engine_response_20260602_123456.json"
    );
}

#[test]
fn cli_defaults_to_engine_contract() {
    assert_eq!(output_contract_from_env(), OutputContract::Engine);
}

#[test]
fn cli_accepts_audit_only_flag() {
    let options = cli_options_from_args(["--audit-only".to_string()], OutputMode::Stdout).unwrap();
    assert_eq!(options.output_contract, OutputContract::AuditOnly);
}

#[test]
fn cli_rejects_conflicting_contract_flags() {
    let result = cli_options_from_args(
        ["--engine".to_string(), "--audit-only".to_string()],
        OutputMode::Stdout,
    );
    assert!(result.is_err());
    let error = result.err().unwrap().to_string();
    assert!(error.contains("cannot use --engine and --audit-only"));
}

#[test]
fn root_output_dir_is_parent_of_crate_output_dir() {
    assert_eq!(
        root_output_dir(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate should have parent")
            .join("output")
    );
}

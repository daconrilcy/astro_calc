use chrono::{TimeZone, Utc};

use rust_sqlx_connection_test::cli::{
    output_mode_from_args, root_output_dir, timestamped_output_filename, OutputMode,
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
        timestamped_output_filename(datetime),
        "basic_payload_20260602_123456.json"
    );
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

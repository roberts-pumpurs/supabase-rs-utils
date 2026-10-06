//! Standalone CLI failure and credential handling, without a database dependency.

#![expect(
    clippy::unwrap_used,
    reason = "Fixture setup and process execution must succeed."
)]
#![expect(
    clippy::expect_used,
    reason = "Fixture cleanup failures should identify the failed operation."
)]
#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise the installed binary interface."
)]

use core::sync::atomic::{AtomicUsize, Ordering};
use std::path::PathBuf;
use std::process::{Command, Output};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rp-supabase-codegen-cli-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn snapshot(&self) -> PathBuf {
        self.0.join("schema.json")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove CLI test directory");
    }
}

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rp-supabase-codegen"));
    command.env_remove("DATABASE_URL");
    command.env_remove("RP_SUPABASE_CODEGEN_TEST_DATABASE_URL");
    command
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn assert_no_build_instructions(output: &Output) {
    for bytes in [&output.stdout, &output.stderr] {
        let output = text(bytes);
        assert!(
            !output.contains("cargo:"),
            "CLI must not emit Cargo instructions"
        );
        assert!(
            !output.contains("rerun-if-"),
            "CLI must not emit Cargo change detection"
        );
    }
}

#[test]
fn missing_subcommand_is_a_usage_error() {
    let output = cli().arg("snapshot").output().unwrap();
    assert_eq!(output.status.code(), Some(2_i32));
    assert_no_build_instructions(&output);
}

#[test]
fn check_requires_existing_snapshot_before_credentials() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    let output = cli()
        .args(["snapshot", "check", "--out"])
        .arg(&snapshot)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1_i32));
    assert!(output.stdout.is_empty());
    assert!(!snapshot.exists());
    assert_no_build_instructions(&output);
}

#[test]
fn check_rejects_unsupported_snapshot_before_connecting_without_writing() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    let contents = b"{\"version\":0,\"schemas\":[]}";
    std::fs::write(&snapshot, contents).unwrap();
    let output = cli()
        .args(["snapshot", "check", "--out"])
        .arg(&snapshot)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1_i32));
    assert_eq!(std::fs::read(&snapshot).unwrap(), contents);
    assert_no_build_instructions(&output);
}

#[test]
fn check_rejects_malformed_metadata_without_writing() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    let contents = format!(
        "{{\"version\":{},\"schemas\":null}}",
        rp_supabase_codegen::model::SNAPSHOT_VERSION
    );
    std::fs::write(&snapshot, &contents).unwrap();
    let output = cli()
        .args(["snapshot", "check", "--out"])
        .arg(&snapshot)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1_i32));
    assert_eq!(std::fs::read_to_string(&snapshot).unwrap(), contents);
    assert_no_build_instructions(&output);
}

#[test]
fn write_reports_selected_missing_environment_variable_without_creating_output() {
    let directory = TestDirectory::new();
    let snapshot = directory.snapshot();
    let output = cli()
        .args(["snapshot", "write", "--out"])
        .arg(&snapshot)
        .args([
            "--schema",
            "public",
            "--schema",
            "private",
            "--database-url-env",
            "RP_SUPABASE_CODEGEN_TEST_DATABASE_URL",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1_i32));
    assert!(!snapshot.exists());
    assert_no_build_instructions(&output);
}

#[test]
fn connection_failures_do_not_disclose_explicit_or_environment_credentials() {
    let directory = TestDirectory::new();
    let url = "postgresql://secret-user:secret-password@localhost:not-a-port/private-db";
    for explicit in [true, false] {
        let mut command = cli();
        command
            .args(["snapshot", "write", "--out"])
            .arg(directory.snapshot());
        if explicit {
            command.args(["--database-url", url]);
        } else {
            command.env("DATABASE_URL", url);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1_i32));
        let error = text(&output.stderr);
        for secret in ["secret-user", "secret-password", "private-db", url] {
            assert!(!error.contains(secret));
            assert!(!text(&output.stdout).contains(secret));
        }
        assert!(!directory.snapshot().exists());
        assert_no_build_instructions(&output);
    }
}

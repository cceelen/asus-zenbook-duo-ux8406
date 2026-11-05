//! The log of the KDE programs. The logger starts once for a process, so
//! each test runs this test program again, with the environment it needs,
//! and reads what it writes to standard error.

use std::process::Command;

use asus_ux8406_kde_common::daemon::start_logging;
use log::{debug, error, info, trace, warn};

/// Set in the process that writes the log.
const WRITER: &str = "KDE_COMMON_TEST_LOG_WRITER";

/// The part of the test program that writes the log; it does nothing where
/// the variable is not set.
#[test]
fn writer() {
    if std::env::var_os(WRITER).is_none() {
        return;
    }
    start_logging();
    error!(target: "asus_ux8406_test", "error");
    warn!(target: "asus_ux8406_test", "warn");
    info!(target: "asus_ux8406_test", "info");
    debug!(target: "asus_ux8406_test", "debug");
    trace!(target: "asus_ux8406_test", "trace");
    error!(target: "other_crate", "other error");
    warn!(target: "other_crate", "other warn");
    info!(target: "other_crate", "other info");
}

/// The lines that the writer writes, with these variables set and the
/// others not.
fn written(variables: &[(&str, &str)]) -> Vec<String> {
    let mut command = Command::new(std::env::current_exe().expect("test program"));

    command
        .args(["--exact", "writer", "--nocapture", "--test-threads=1"])
        .env(WRITER, "1")
        .env_remove("RUST_LOG")
        .env_remove("JOURNAL_STREAM")
        .envs(variables.iter().copied());

    let output = command.output().expect("the writer runs");

    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_programs_log_info_and_the_other_crates_warn() {
    assert_eq!(
        written(&[]),
        ["error", "warn", "info", "other error", "other warn"]
    );
}

#[test]
fn the_journal_gets_the_priority_at_the_start_of_each_line() {
    assert_eq!(
        written(&[
            ("JOURNAL_STREAM", "8:1"),
            ("RUST_LOG", "asus_ux8406_test=trace")
        ]),
        [
            "<3>error",
            "<4>warn",
            "<6>info",
            "<7>debug",
            "<7>trace",
            "<3>other error",
            "<4>other warn",
        ]
    );
}

#[test]
fn rust_log_sets_the_level() {
    assert_eq!(
        written(&[("RUST_LOG", "asus_ux8406_test=error")]),
        ["error", "other error", "other warn"]
    );
}

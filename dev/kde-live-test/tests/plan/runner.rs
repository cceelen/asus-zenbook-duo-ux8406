//! The programs of the machine.

use asus_ux8406_kde_live_test::runner::{Output, Runner, System};

#[test]
fn a_program_gives_its_code_and_output() {
    let output = System.run("sh", &["-c", "echo out; echo err >&2; exit 3"]);

    assert_eq!(output.code, Some(3));
    assert_eq!(output.stdout, "out\n");
    assert_eq!(output.stderr, "err\n");
    assert_eq!(output.text(), "out\nerr\n");
    assert!(!output.success());
    assert!(System.run("true", &[]).success());
}

#[test]
fn a_program_that_does_not_exist_has_no_code() {
    let output = System.run("/nonexistent/program", &[]);

    assert_eq!(output.code, None);
    assert!(output.stderr.contains("/nonexistent/program"));
    assert_eq!(Output::new(0, "x").stdout, "x");
}

#[test]
fn an_attached_program_gives_its_code() {
    assert_eq!(System.run_attached("sh", &["-c", "exit 3"]).code, Some(3));
    assert!(System.run_attached("true", &[]).success());
    assert_eq!(System.run_attached("true", &[]).text(), "");
}

#[test]
fn an_attached_program_that_does_not_exist_has_no_code() {
    let output = System.run_attached("/nonexistent/program", &[]);

    assert_eq!(output.code, None);
    assert!(output.stderr.contains("/nonexistent/program"));
}

#[test]
fn a_started_program_writes_its_log() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("log");
    let pid = System
        .start("sh", &["-c", "echo ready; echo bad >&2"], &log)
        .unwrap();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);

    assert!(pid > 0);
    while std::fs::read_to_string(&log).unwrap().lines().count() < 2 {
        assert!(std::time::Instant::now() < end);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn a_program_that_cannot_start_is_an_error() {
    let dir = tempfile::tempdir().unwrap();

    assert!(
        System
            .start("/nonexistent/program", &[], &dir.path().join("log"))
            .unwrap_err()
            .contains("/nonexistent/program")
    );
    assert!(
        System
            .start("true", &[], &dir.path().join("no/such/dir/log"))
            .is_err()
    );
}

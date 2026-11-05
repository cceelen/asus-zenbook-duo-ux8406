//! The program as a whole, on a machine that is not a live session, and the
//! run with a console of its own.

use std::process::Command;

use asus_ux8406_kde_live_test::cli::{Cli, execute_with};
use clap::Parser;

use crate::support::{FakeConsole, Fixture};

const PROGRAM: &str = env!("CARGO_BIN_EXE_ux8406-kde-live-test");

#[test]
fn the_program_refuses_an_installed_system() {
    let output = Command::new(PROGRAM).output().unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).contains("not a live session"));
}

#[test]
fn the_options_have_defaults() {
    let cli = Cli::parse_from(["x"]);

    assert_eq!(
        (cli.timeout, cli.dir.is_none(), cli.no_prompt, cli.pen),
        (None, true, false, false)
    );
    assert_eq!(cli.root, std::path::Path::new("/"));
    assert!(Cli::parse_from(["x", "--pen"]).pen);
    assert_eq!(Cli::parse_from(["x", "-t", "30"]).timeout, Some(30));
}

fn live_root() -> Fixture {
    let fixture = Fixture::new();

    fixture.file("/proc/cmdline", "rd.live.image");
    fixture.file("/proc/mounts", "overlay / overlay rw 0 0\n");
    fixture
}

fn arguments(fixture: &Fixture) -> Cli {
    Cli::parse_from([
        "x",
        "--root",
        &fixture.root.path().display().to_string(),
        "--dir",
        &fixture.dir.path().display().to_string(),
        "--timeout",
        "1",
        "--no-prompt",
    ])
}

#[test]
fn a_run_on_a_machine_that_is_not_a_ux8406_fails_in_the_preflight() {
    let fixture = live_root();
    let console = FakeConsole::new();

    // The real programs run: this is not a UX8406 and not a Wayland session
    // of the test, so the preflight stops the run with failed checks.
    fixture.file("/sys/class/dmi/id/product_name", "Other\n");
    assert_eq!(execute_with(&arguments(&fixture), &console), 1);

    let report = std::fs::read_to_string(fixture.dir.path().join("results/report.md")).unwrap();

    assert!(report.contains("| product name contains UX8406 | automatic | FAIL |"));
    assert!(console.has_said("Done:"));
}

#[test]
fn a_results_directory_that_cannot_be_made_stops_the_run() {
    let fixture = live_root();
    let console = FakeConsole::new();

    std::fs::write(fixture.dir.path().join("results"), "a file").unwrap();
    assert_eq!(execute_with(&arguments(&fixture), &console), 2);
    assert!(console.has_said("cannot make"));
}

#[test]
fn results_that_cannot_be_written_stop_the_run() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = live_root();
    let console = FakeConsole::new();
    let results = fixture.dir.path().join("results");

    // The directories are there, so the report opens; no write is possible.
    std::fs::create_dir_all(results.join("raw")).unwrap();
    std::fs::set_permissions(&results, std::fs::Permissions::from_mode(0o555)).unwrap();
    assert_eq!(execute_with(&arguments(&fixture), &console), 2);
    assert!(console.has_said("cannot write the results"));
    std::fs::set_permissions(&results, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn the_program_without_a_directory_uses_its_own() {
    let fixture = live_root();
    // A link next to the program (the same file system); a copy that is written
    // while other tests start programs can be busy when it is started.
    let home = tempfile::tempdir_in(std::path::Path::new(PROGRAM).parent().unwrap()).unwrap();
    let copy = home.path().join("ux8406-kde-live-test");

    fixture.file("/sys/class/dmi/id/product_name", "Other\n");
    std::fs::hard_link(PROGRAM, &copy).unwrap();

    let output = Command::new(&copy)
        .args([
            "--no-prompt",
            "--root",
            &fixture.root.path().display().to_string(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(home.path().join("results/report.md").is_file());
}

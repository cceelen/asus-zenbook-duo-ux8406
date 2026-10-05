//! The command line of the program.

use assert_cmd::Command;
use predicates::prelude::*;
use rstest::rstest;

use crate::world::{USAGE, World};

/// Command lines that must not be accepted.
#[rstest]
#[case(&["-H", "8x"])]
#[case(&["-H", "200"])]
#[case(&["-P", "0"])]
#[case(&["-t", "-1"])]
#[case(&["-t", "1h"])]
#[case(&["extra"])]
#[case(&["-t", "1", "x"])]
#[case(&["-x"])]
#[case(&["-H"])]
#[case(&["run", "restore"])]
#[case(&["-c", "/dev/null", "-c", "/dev/null"])]
fn refused(#[case] arguments: &[&str]) {
    let mut world = World::new();

    world.expect_refusal_with(USAGE, &arguments.join(" "), arguments);
    world.expect_log("error: ");
    world.expect_log("For more information, try '--help'.");
    world.expect_start("without those options");
}

/// Settings from the command line come after those of the file.
#[test]
fn settings() {
    let mut world = World::new();

    world.frozen = true;
    assert_eq!(
        world.run(&["-H", "90", "-P", "60", "-t", "1", "run"]),
        Some(0)
    );
    world.expect_log("hold_temp 90, step_up 2, max_offset 30, interval 1, settle 2");
    world.expect_log("probe_interval 60, ");
}

#[rstest]
#[case("-h")]
#[case("--help")]
fn help(#[case] argument: &str) {
    Command::cargo_bin("asus-ux8406-tcc-guard")
        .unwrap()
        .arg(argument)
        .assert()
        .success()
        .stderr("")
        .stdout(predicate::str::contains(
            "Usage: asus-ux8406-tcc-guard [OPTIONS] [COMMAND]",
        ))
        .stdout(predicate::str::contains("restore"))
        .stdout(predicate::str::contains("event_text"));
}

#[rstest]
#[case("-V")]
#[case("--version")]
fn version(#[case] argument: &str) {
    Command::cargo_bin("asus-ux8406-tcc-guard")
        .unwrap()
        .arg(argument)
        .assert()
        .success()
        .stderr("")
        .stdout(format!(
            "asus-ux8406-tcc-guard {}\n",
            env!("CARGO_PKG_VERSION")
        ));
}

/// Everything the guard says goes to the error stream, one plain line per
/// message: the time is the reader's to add.
#[test]
fn lines_are_plain_and_nothing_goes_to_the_output() {
    let world = World::new();

    Command::cargo_bin("asus-ux8406-tcc-guard")
        .unwrap()
        .arg("--root")
        .arg(world.machine.root())
        .args(["-t", "1"])
        .env_remove("JOURNAL_STREAM")
        .assert()
        .success()
        .stdout("")
        .stderr(predicate::str::starts_with(
            "package 0: armed: offset 10, CPU 95 C, tjmax 105\n",
        ))
        .stderr(predicate::str::ends_with(
            "package 0: stopping: offset untouched\n",
        ));
}

/// Under systemd every line carries its priority the way the journal reads
/// it.
#[test]
fn lines_for_the_journal_carry_their_priority() {
    let world = World::new();

    Command::cargo_bin("asus-ux8406-tcc-guard")
        .unwrap()
        .arg("--root")
        .arg(world.machine.root())
        .args(["-t", "1"])
        .env("JOURNAL_STREAM", "8:12345")
        .assert()
        .success()
        .stderr(predicate::str::starts_with(
            "<6>package 0: armed: offset 10",
        ));
    world.machine.remove(crate::common::OFFSET);
    Command::cargo_bin("asus-ux8406-tcc-guard")
        .unwrap()
        .arg("--root")
        .arg(world.machine.root())
        .env("JOURNAL_STREAM", "8:12345")
        .assert()
        .code(1)
        .stderr(predicate::str::starts_with("<3>"));
}

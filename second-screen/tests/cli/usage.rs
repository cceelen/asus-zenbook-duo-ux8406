//! The command line: usage, help and version.

use predicates::prelude::*;
use rstest::rstest;

use crate::common::{LOWER_STATUS, Sys, UNTOUCHED, docked};

#[rstest]
#[case(&[])]
#[case(&["--bogus", "dock"])]
#[case(&["dock", "brightness"])]
fn a_wrong_command_line_is_exit_status_1(docked: Sys, #[case] args: &[&str]) {
    docked
        .program()
        .args(args)
        .assert()
        .code(1)
        .stderr(predicate::str::starts_with(
            "usage: asus-ux8406-second-screen",
        ));
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
}

#[rstest]
fn help_and_version_print_and_touch_nothing(docked: Sys) {
    docked
        .program()
        .arg("--version")
        .assert()
        .success()
        .stdout(concat!(
            "asus-ux8406-second-screen ",
            env!("CARGO_PKG_VERSION"),
            "\n"
        ));
    docked
        .program()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "usage: asus-ux8406-second-screen",
        ));
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
}

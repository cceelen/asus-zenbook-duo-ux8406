//! The help and the version.

use assert_cmd::Command;
use asus_ux8406_keyboard_state::cli::USAGE;
use predicates::prelude::*;
use rstest::rstest;

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_asus-ux8406-keyboard-state"))
}

#[rstest]
#[case::short("-h")]
#[case::long("--help")]
fn help_prints_the_usage_and_succeeds(#[case] option: &str) {
    program()
        .arg(option)
        .assert()
        .success()
        .stdout(predicate::str::diff(format!("{USAGE}\n")))
        .stderr("");
}

#[rstest]
#[case::short("-V")]
#[case::long("--version")]
fn version_prints_the_name_and_the_version(#[case] option: &str) {
    program()
        .arg(option)
        .assert()
        .success()
        .stdout(format!(
            "asus-ux8406-keyboard-state {}\n",
            env!("CARGO_PKG_VERSION")
        ))
        .stderr("");
}

#[test]
fn the_usage_in_the_source_is_the_usage_printed() {
    let source = include_str!("../../src/main.rs");
    let documented: String = source
        .lines()
        .skip_while(|line| *line != "//! ```text")
        .skip(1)
        .take_while(|line| *line != "//! ```")
        .map(|line| line.strip_prefix("//! ").unwrap_or("").to_owned() + "\n")
        .collect();

    assert_eq!(documented.trim_end(), USAGE);
}

//! `sleep`, as systemd runs it through the hook before and after a sleep.

use std::fs;

use predicates::prelude::*;
use rstest::rstest;

use crate::common::{BRIGHT_HOTKEYS, Tree, USB, tree};

#[rstest]
#[case::before(&["sleep", "pre"])]
#[case::after(&["sleep", "post"])]
#[case::before_suspend(&["sleep", "pre", "suspend"])]
#[case::after_hibernate(&["sleep", "post", "hibernate"])]
fn without_a_keyboard_nothing_is_done(tree: Tree, #[case] arguments: &[&str]) {
    tree.write_state(BRIGHT_HOTKEYS);

    tree.program()
        .args(arguments)
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(tree.state().as_deref(), Some(BRIGHT_HOTKEYS));
}

/// A file is not a map, so the keyboard has no state map the program can
/// read; the node is a file, which hidraw cannot open.
#[rstest]
fn a_node_that_cannot_be_opened_is_reported_for_the_device(tree: Tree) {
    tree.pin(USB, &[1, 3]);
    tree.hidraw(USB, "hidraw3");
    fs::create_dir_all(tree.dev_root()).unwrap();
    fs::write(tree.dev_node("hidraw3"), "").unwrap();

    tree.program()
        .args(["sleep", "pre"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::starts_with(format!(
            "asus-ux8406-keyboard-state: {USB}: "
        )));
}

#[rstest]
fn a_device_without_a_node_is_reported(tree: Tree) {
    tree.pin(USB, &[1, 3]);

    tree.program()
        .args(["sleep", "pre"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::diff(format!(
            "asus-ux8406-keyboard-state: {USB}: no hidraw node\n"
        )));
}

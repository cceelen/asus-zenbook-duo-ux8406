//! `load`, as udev runs it when a HID device of the keyboard comes.

use predicates::prelude::*;
use rstest::rstest;

use crate::common::{BLUETOOTH, BRIGHT_HOTKEYS, Tree, USB, tree};

#[rstest]
fn without_a_state_nothing_is_printed(tree: Tree) {
    tree.program()
        .args(["load", USB])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[rstest]
fn the_written_state_is_printed_as_udev_properties(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);

    tree.program()
        .args(["load", BLUETOOTH])
        .assert()
        .success()
        .stdout(BRIGHT_HOTKEYS)
        .stderr("");
}

/// A file is not a map, so the other connection cannot be read here.
#[rstest]
fn a_pin_that_is_not_a_map_leaves_the_written_state(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);
    tree.pin(BLUETOOTH, &[1, 0]);

    tree.program()
        .args(["load", USB])
        .assert()
        .success()
        .stdout(BRIGHT_HOTKEYS)
        .stderr("");
}

#[rstest]
fn a_state_file_with_other_text_is_reported_and_nothing_printed(tree: Tree) {
    tree.write_state("LD_PRELOAD=/tmp/x.so\n");

    tree.program()
        .args(["load", USB])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::diff(format!(
            "asus-ux8406-keyboard-state: {}: not a keyboard state\n",
            tree.state_file().display()
        )));
}

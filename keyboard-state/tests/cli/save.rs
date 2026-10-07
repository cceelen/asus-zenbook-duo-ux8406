//! `save`, as udev runs it when a HID device of the keyboard goes.

use predicates::prelude::*;
use rstest::rstest;

use crate::common::{BRIGHT_HOTKEYS, Tree, USB, USB_OTHER_INTERFACE, tree};

#[rstest]
fn a_device_without_the_program_is_no_error(tree: Tree) {
    tree.program()
        .args(["save", USB_OTHER_INTERFACE])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(tree.state(), None);
}

/// A file is not a map: this is as far as the program gets without root and
/// a loaded program.
#[rstest]
fn a_pin_that_is_not_a_map_is_reported_and_nothing_written(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);
    let pin = tree.pin(USB, &[1, 0]);

    tree.program()
        .args(["save", USB])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::starts_with(format!(
            "asus-ux8406-keyboard-state: {}: ",
            pin.display()
        )));

    assert_eq!(tree.state().as_deref(), Some(BRIGHT_HOTKEYS));
}

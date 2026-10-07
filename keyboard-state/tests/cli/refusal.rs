//! Command lines that the program does not act on.

use predicates::prelude::*;
use rstest::rstest;

use crate::common::{BLUETOOTH, BRIGHT_HOTKEYS, Tree, USB, tree};
use asus_ux8406_keyboard_state::cli::USAGE;

#[rstest]
#[case::nothing(&[])]
#[case::no_device(&["save"])]
#[case::no_action(&[USB])]
#[case::unknown_action(&["restore", USB])]
#[case::another_device(&["save", "0003:056A:0357.0007"])]
#[case::a_path_for_a_device(&["load", "../../../etc/passwd"])]
#[case::two_devices(&["save", USB, BLUETOOTH])]
#[case::unknown_option(&["--force", "save", USB])]
fn the_usage_is_the_reply_and_nothing_is_done(tree: Tree, #[case] arguments: &[&str]) {
    tree.write_state(BRIGHT_HOTKEYS);

    tree.program()
        .args(arguments)
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::diff(format!("{USAGE}\n")));

    assert_eq!(tree.state().as_deref(), Some(BRIGHT_HOTKEYS));
}

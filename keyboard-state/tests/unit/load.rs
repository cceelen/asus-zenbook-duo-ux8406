//! What a connection that starts is given.

use asus_ux8406_keyboard_state::{Error, State, load};
use rstest::rstest;

use crate::common::{BLUETOOTH, BRIGHT_HOTKEYS, FileMaps, TABLET, Tree, USB, device, tree};

const BRIGHT: State = State {
    fn_lock: 0,
    backlight: 3,
};
const OFF_FUNCTION_KEYS: State = State {
    fn_lock: 1,
    backlight: 0,
};

fn loaded(tree: &Tree, name: &str) -> Result<Option<State>, Error> {
    load(&tree.bpffs(), &FileMaps, &device(name), &tree.state_file())
}

#[rstest]
fn without_a_connection_before_there_is_no_state(tree: Tree) {
    assert_eq!(loaded(&tree, USB).unwrap(), None);
}

#[rstest]
fn the_written_state_is_given(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);

    assert_eq!(loaded(&tree, USB).unwrap(), Some(BRIGHT));
}

#[rstest]
fn another_connection_that_is_still_there_comes_first(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);
    tree.pin(BLUETOOTH, &[1, 0]);

    assert_eq!(loaded(&tree, USB).unwrap(), Some(OFF_FUNCTION_KEYS));
}

#[rstest]
fn another_connection_is_enough_without_a_written_state(tree: Tree) {
    tree.pin(USB, &[1, 0]);

    assert_eq!(loaded(&tree, BLUETOOTH).unwrap(), Some(OFF_FUNCTION_KEYS));
}

#[rstest]
fn the_pins_of_the_device_itself_are_not_its_start(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);
    tree.pin(USB, &[1, 0]);

    assert_eq!(loaded(&tree, USB).unwrap(), Some(BRIGHT));
}

#[rstest]
fn another_device_is_not_a_connection_of_the_keyboard(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);
    tree.pin(TABLET, &[1, 0]);

    assert_eq!(loaded(&tree, USB).unwrap(), Some(BRIGHT));
}

#[rstest]
#[case::cannot_be_read(&[1, 0, 0])]
#[case::unknown_values(&[1, 200])]
fn a_bad_map_of_another_connection_leaves_the_written_state(tree: Tree, #[case] value: &[u8]) {
    tree.write_state(BRIGHT_HOTKEYS);
    tree.pin(BLUETOOTH, value);

    assert_eq!(loaded(&tree, USB).unwrap(), Some(BRIGHT));
}

#[rstest]
fn a_bad_map_and_no_written_state_give_no_state(tree: Tree) {
    tree.pin(BLUETOOTH, &[9, 9]);

    assert_eq!(loaded(&tree, USB).unwrap(), None);
}

#[rstest]
fn a_state_file_with_other_text_is_refused(tree: Tree) {
    tree.write_state("ASUS_UX8406_KBD_BACKLIGHT=3\nID_INPUT_KEYBOARD=0\n");

    let error = loaded(&tree, USB).unwrap_err();

    assert!(matches!(error, Error::BadState(path) if path == tree.state_file()));
}

#[rstest]
fn a_state_file_that_cannot_be_read_is_reported(tree: Tree) {
    std::fs::create_dir(tree.state_file()).unwrap();

    let error = loaded(&tree, USB).unwrap_err();

    assert!(matches!(error, Error::Io { path, .. } if path == tree.state_file()));
}

//! What is written when a connection ends.

use asus_ux8406_keyboard_state::{Error, State, save};
use rstest::rstest;

use crate::common::{BRIGHT_HOTKEYS, FileMaps, Tree, USB, USB_OTHER_INTERFACE, device, tree};

#[rstest]
fn the_state_of_the_device_is_written(tree: Tree) {
    tree.pin(USB, &[0, 3]);

    let saved = save(&tree.bpffs(), &FileMaps, &device(USB), &tree.state_file()).unwrap();

    assert_eq!(
        saved,
        Some(State {
            fn_lock: 0,
            backlight: 3
        })
    );
    assert_eq!(tree.state().as_deref(), Some(BRIGHT_HOTKEYS));
}

#[rstest]
fn a_newer_state_replaces_the_one_written(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);
    tree.pin(USB, &[1, 0]);

    save(&tree.bpffs(), &FileMaps, &device(USB), &tree.state_file()).unwrap();

    assert_eq!(
        tree.state().as_deref(),
        Some("ASUS_UX8406_KBD_BACKLIGHT=0\nASUS_UX8406_KBD_FN_LOCK=1\n")
    );
}

#[rstest]
fn a_device_without_the_program_writes_nothing(tree: Tree) {
    tree.pin(USB, &[0, 3]);

    let saved = save(
        &tree.bpffs(),
        &FileMaps,
        &device(USB_OTHER_INTERFACE),
        &tree.state_file(),
    )
    .unwrap();

    assert_eq!(saved, None);
    assert_eq!(tree.state(), None);
}

#[rstest]
fn a_device_without_the_program_leaves_the_written_state(tree: Tree) {
    tree.write_state(BRIGHT_HOTKEYS);

    save(
        &tree.bpffs(),
        &FileMaps,
        &device(USB_OTHER_INTERFACE),
        &tree.state_file(),
    )
    .unwrap();

    assert_eq!(tree.state().as_deref(), Some(BRIGHT_HOTKEYS));
}

#[rstest]
#[case::backlight_above_the_brightest(&[0, 9])]
#[case::function_row_above_its_modes(&[7, 0])]
fn a_map_with_unknown_values_is_refused_and_nothing_written(tree: Tree, #[case] value: &[u8]) {
    tree.write_state(BRIGHT_HOTKEYS);
    let pin = tree.pin(USB, value);

    let error = save(&tree.bpffs(), &FileMaps, &device(USB), &tree.state_file()).unwrap_err();

    assert!(matches!(error, Error::BadState(path) if path == pin));
    assert_eq!(tree.state().as_deref(), Some(BRIGHT_HOTKEYS));
}

#[rstest]
fn a_map_that_cannot_be_read_is_reported_and_nothing_written(tree: Tree) {
    let pin = tree.pin(USB, &[0, 3, 0]);

    let error = save(&tree.bpffs(), &FileMaps, &device(USB), &tree.state_file()).unwrap_err();

    assert!(matches!(error, Error::Map { path, .. } if path == pin));
    assert_eq!(tree.state(), None);
}

#[rstest]
fn a_state_file_that_cannot_be_written_is_reported(tree: Tree) {
    tree.pin(USB, &[0, 3]);
    let file = tree.state_file().join("below-a-missing-directory");

    let error = save(&tree.bpffs(), &FileMaps, &device(USB), &file).unwrap_err();

    assert!(matches!(error, Error::Io { path, .. } if path == file));
}

#[rstest]
fn nothing_but_the_state_file_is_left_behind(tree: Tree) {
    tree.pin(USB, &[0, 3]);

    save(&tree.bpffs(), &FileMaps, &device(USB), &tree.state_file()).unwrap();

    let names: Vec<_> = std::fs::read_dir(tree.state_file().parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

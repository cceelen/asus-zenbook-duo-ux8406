//! Where the state maps of the keyboard's devices are looked for.

use std::path::PathBuf;

use rstest::rstest;

use crate::common::{BLUETOOTH, TABLET, Tree, USB, USB_OTHER_INTERFACE, device, tree};

/// No map.
const NONE: [PathBuf; 0] = [];

#[rstest]
fn the_map_of_a_device_is_below_its_directory(tree: Tree) {
    let pin = tree.pin(USB, &[0, 0]);

    assert_eq!(tree.bpffs().map_of(&device(USB)), Some(pin));
}

#[rstest]
fn a_device_without_pins_has_no_map(tree: Tree) {
    tree.pin(USB, &[0, 0]);

    assert_eq!(tree.bpffs().map_of(&device(USB_OTHER_INTERFACE)), None);
}

#[rstest]
fn a_device_with_another_program_has_no_map(tree: Tree) {
    tree.pin_other_program(USB);

    assert_eq!(tree.bpffs().map_of(&device(USB)), None);
}

#[rstest]
fn without_a_bpf_file_system_there_is_no_map(tree: Tree) {
    assert_eq!(tree.bpffs().map_of(&device(USB)), None);
    assert_eq!(tree.bpffs().maps_of_others(&device(USB)), NONE);
}

#[rstest]
fn the_maps_of_others_leave_out_the_device_itself(tree: Tree) {
    tree.pin(USB, &[0, 0]);
    let bluetooth = tree.pin(BLUETOOTH, &[0, 0]);

    assert_eq!(tree.bpffs().maps_of_others(&device(USB)), [bluetooth]);
}

#[rstest]
fn the_maps_of_others_are_those_of_the_keyboard_only(tree: Tree) {
    tree.pin(TABLET, &[0, 0]);
    tree.pin_other_program(BLUETOOTH);

    assert_eq!(tree.bpffs().maps_of_others(&device(USB)), NONE);
}

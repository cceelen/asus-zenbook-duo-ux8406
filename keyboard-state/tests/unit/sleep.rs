//! The backlight for the sleep of the system.

use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use asus_ux8406_keyboard_state::sleep::{LEVEL_INDEX, REPORT_SIZE, report};
use asus_ux8406_keyboard_state::{Error, HidrawSender, Phase, ReportSender, sleep};
use rstest::rstest;

use crate::common::{
    BLUETOOTH, FakeSender, FileMaps, TABLET, Tree, USB, USB_OTHER_INTERFACE, tree,
};

/// The report that sets the backlight to `level`, as the program sends it.
fn report_for(level: u8) -> Vec<u8> {
    let mut bytes = vec![0x5a, 0xba, 0xc5, 0xc4, level];

    bytes.resize(16, 0);
    bytes
}

/// Run `sleep` on the tree with `sender`.
fn run(tree: &Tree, sender: &FakeSender, phase: Phase) -> Result<(), Error> {
    sleep(
        &tree.bpffs(),
        &FileMaps,
        sender,
        &tree.sysfs_root(),
        &tree.dev_root(),
        phase,
    )
}

/// What the sender got.
fn sent(sender: &FakeSender) -> Vec<(PathBuf, Vec<u8>)> {
    sender.sent.borrow().clone()
}

#[rstest]
#[case::off(0)]
#[case::dim(1)]
#[case::bright(3)]
fn the_report_is_the_one_of_the_program(#[case] level: u8) {
    assert_eq!(report(level).to_vec(), report_for(level));
    assert_eq!(report(level).len(), REPORT_SIZE);
    assert_eq!(report(level)[LEVEL_INDEX], level);
}

#[rstest]
fn before_the_sleep_the_backlight_goes_off(tree: Tree) {
    tree.pin(USB, &[1, 3]);
    tree.pin(BLUETOOTH, &[0, 2]);
    tree.hidraw(USB, "hidraw3");
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender::default();

    run(&tree, &sender, Phase::Pre).unwrap();

    assert_eq!(
        sent(&sender),
        [
            (tree.dev_node("hidraw3"), report_for(0)),
            (tree.dev_node("hidraw5"), report_for(0)),
        ]
    );
}

#[rstest]
fn after_the_sleep_the_backlight_has_the_level_of_the_map(tree: Tree) {
    tree.pin(USB, &[1, 3]);
    tree.pin(BLUETOOTH, &[0, 2]);
    tree.hidraw(USB, "hidraw3");
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender::default();

    run(&tree, &sender, Phase::Post).unwrap();

    assert_eq!(
        sent(&sender),
        [
            (tree.dev_node("hidraw3"), report_for(3)),
            (tree.dev_node("hidraw5"), report_for(2)),
        ]
    );
}

#[rstest]
fn the_map_is_not_changed(tree: Tree) {
    let pin = tree.pin(USB, &[1, 3]);
    tree.hidraw(USB, "hidraw3");

    run(&tree, &FakeSender::default(), Phase::Pre).unwrap();

    assert_eq!(std::fs::read(pin).unwrap(), [1, 3]);
}

#[rstest]
fn the_first_node_of_a_device_is_used(tree: Tree) {
    tree.pin(USB, &[0, 1]);
    tree.hidraw(USB, "hidraw7");
    tree.hidraw(USB, "hidraw4");
    let sender = FakeSender::default();

    run(&tree, &sender, Phase::Post).unwrap();

    assert_eq!(sent(&sender), [(tree.dev_node("hidraw4"), report_for(1))]);
}

#[rstest]
#[case::before(Phase::Pre)]
#[case::after(Phase::Post)]
fn without_a_device_nothing_is_done(tree: Tree, #[case] phase: Phase) {
    let sender = FakeSender::default();

    run(&tree, &sender, phase).unwrap();

    assert_eq!(sent(&sender), []);
}

#[rstest]
fn devices_without_the_program_are_left_alone(tree: Tree) {
    tree.pin(USB, &[0, 3]);
    tree.hidraw(USB, "hidraw3");
    tree.hidraw(USB_OTHER_INTERFACE, "hidraw4");
    tree.pin(TABLET, &[0, 3]);
    tree.hidraw(TABLET, "hidraw9");
    tree.pin_other_program(BLUETOOTH);
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender::default();

    run(&tree, &sender, Phase::Pre).unwrap();

    assert_eq!(sent(&sender), [(tree.dev_node("hidraw3"), report_for(0))]);
}

#[rstest]
fn a_device_without_a_node_is_an_error_and_the_others_are_set(tree: Tree) {
    tree.pin(USB, &[0, 3]);
    tree.pin(BLUETOOTH, &[0, 2]);
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender::default();

    let error = run(&tree, &sender, Phase::Post).unwrap_err();

    assert!(matches!(&error, Error::NoHidraw(device) if device.to_string() == USB));
    assert_eq!(error.to_string(), format!("{USB}: no hidraw node"));
    assert_eq!(sent(&sender), [(tree.dev_node("hidraw5"), report_for(2))]);
}

#[rstest]
fn a_device_with_an_empty_node_directory_has_no_node(tree: Tree) {
    tree.pin(USB, &[0, 3]);
    std::fs::create_dir_all(
        tree.sysfs_root()
            .join("bus/hid/devices")
            .join(USB)
            .join("hidraw"),
    )
    .unwrap();

    let error = run(&tree, &FakeSender::default(), Phase::Pre).unwrap_err();

    assert!(matches!(error, Error::NoHidraw(_)));
}

#[rstest]
fn a_sender_error_names_the_device_and_the_others_are_set(tree: Tree) {
    tree.pin(USB, &[0, 3]);
    tree.pin(BLUETOOTH, &[0, 2]);
    tree.hidraw(USB, "hidraw3");
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender {
        failing: vec![tree.dev_node("hidraw3")],
        ..FakeSender::default()
    };

    let error = run(&tree, &sender, Phase::Pre).unwrap_err();

    assert!(matches!(&error, Error::Send { device, .. } if device.to_string() == USB));
    assert_eq!(error.to_string(), format!("{USB}: the node refused"));
    assert_eq!(sent(&sender), [(tree.dev_node("hidraw5"), report_for(0))]);
}

#[rstest]
fn the_first_of_two_errors_is_the_result(tree: Tree) {
    tree.pin(USB, &[0, 3]);
    tree.pin(BLUETOOTH, &[0, 2]);
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender {
        failing: vec![tree.dev_node("hidraw5")],
        ..FakeSender::default()
    };

    let error = run(&tree, &sender, Phase::Pre).unwrap_err();

    assert!(matches!(error, Error::NoHidraw(_)));
}

#[rstest]
fn after_the_sleep_a_map_that_cannot_be_read_is_an_error(tree: Tree) {
    let pin = tree.pin(USB, &[0]);
    tree.pin(BLUETOOTH, &[0, 2]);
    tree.hidraw(USB, "hidraw3");
    tree.hidraw(BLUETOOTH, "hidraw5");
    let sender = FakeSender::default();

    let error = run(&tree, &sender, Phase::Post).unwrap_err();

    assert!(matches!(error, Error::Map { path, .. } if path == pin));
    assert_eq!(sent(&sender), [(tree.dev_node("hidraw5"), report_for(2))]);
}

#[rstest]
#[case::backlight(&[0, 4])]
#[case::function_row(&[2, 1])]
fn after_the_sleep_a_value_that_is_not_known_is_an_error(tree: Tree, #[case] value: &[u8]) {
    let pin = tree.pin(USB, value);
    tree.hidraw(USB, "hidraw3");
    let sender = FakeSender::default();

    let error = run(&tree, &sender, Phase::Post).unwrap_err();

    assert!(matches!(error, Error::BadState(path) if path == pin));
    assert_eq!(sent(&sender), []);
}

#[rstest]
fn before_the_sleep_the_map_is_not_read(tree: Tree) {
    tree.pin(USB, &[9]);
    tree.hidraw(USB, "hidraw3");
    let sender = FakeSender::default();

    run(&tree, &sender, Phase::Pre).unwrap();

    assert_eq!(sent(&sender), [(tree.dev_node("hidraw3"), report_for(0))]);
}

#[rstest]
fn a_node_that_is_not_there_is_an_error(tree: Tree) {
    let error = HidrawSender
        .send(&tree.dev_node("hidraw0"), &report(0))
        .unwrap_err();

    assert_ne!(error, "");
}

#[rstest]
fn a_node_that_is_a_plain_file_is_an_error(tree: Tree) {
    std::fs::create_dir_all(tree.dev_root()).unwrap();
    std::fs::write(tree.dev_node("hidraw0"), "").unwrap();

    let error = HidrawSender
        .send(&tree.dev_node("hidraw0"), &report(0))
        .unwrap_err();

    assert_ne!(error, "");
}

#[test]
fn a_node_with_a_null_byte_in_its_name_is_an_error() {
    let node = std::ffi::OsStr::from_bytes(b"/dev/hidraw\0");

    assert!(
        HidrawSender
            .send(std::path::Path::new(node), &report(0))
            .is_err()
    );
}

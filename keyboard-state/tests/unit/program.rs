//! That this crate, the HID-BPF program and the udev rule agree on names and
//! values. Each of the three is read by a different program at run time, and
//! none of them reports a name that the others do not know.

use asus_ux8406_keyboard_state::pins::STATE_MAP;
use asus_ux8406_keyboard_state::state::{
    BACKLIGHT_PROPERTY, FN_LOCK_PROPERTY, MAX_BACKLIGHT, MAX_FN_LOCK,
};
use rstest::rstest;

/// The source of the HID-BPF program.
const PROGRAM: &str =
    include_str!("../../../keyboard-bpf/src/ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.c");
/// The udev rule, before the build fills in the helper's directory.
const RULES: &str = include_str!("../../udev/80-asus-ux8406-keyboard-state.rules.in");
/// The hwdb entries that have udev-hid-bpf load the program.
const HWDB: &str = include_str!("../../../keyboard-bpf/udev/82-hid-bpf-asus-ux8406.hwdb");

/// The program without line breaks and runs of spaces.
fn program() -> String {
    PROGRAM.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[rstest]
#[case::backlight(BACKLIGHT_PROPERTY)]
#[case::function_row(FN_LOCK_PROPERTY)]
fn the_program_has_a_variable_for_each_property(#[case] property: &str) {
    assert!(program().contains(&format!("char UDEV_PROP_{property}[")));
}

#[test]
fn the_program_names_the_map_as_this_crate_does() {
    assert!(program().contains(&format!("}} {STATE_MAP} SEC(\".maps\");")));
}

#[test]
fn the_value_of_the_map_is_function_row_then_backlight() {
    assert!(program().contains("struct KbdState { __u8 fn_lock; __u8 backlight; };"));
}

#[test]
fn the_largest_values_are_those_of_the_program() {
    assert!(program().contains(&format!("kBacklightOn = {MAX_BACKLIGHT},")));
    assert!(program().contains(&format!("kFnLockFunctionKeys = {MAX_FN_LOCK},")));
}

/// The devices of the hwdb, as the kernel names them: `0003:0B05:1BF2.`.
fn loaded_for() -> Vec<String> {
    HWDB.lines()
        .filter_map(|line| line.strip_prefix("hid-bpf:hid:b"))
        .map(|ids| {
            let (bus, rest) = ids.split_once('g').unwrap();
            let (_, rest) = rest.split_once('v').unwrap();
            let (vendor, product) = rest.split_once('p').unwrap();

            format!("{bus}:{}:{}.", &vendor[4..], &product[4..])
        })
        .collect()
}

#[test]
fn the_rule_and_this_crate_know_every_device_the_program_is_loaded_for() {
    let devices = loaded_for();

    assert_eq!(devices.len(), 3);
    for device in devices {
        assert!(
            RULES.contains(&format!("{device}*")),
            "{device} in the rule"
        );
        assert!(
            format!("{device}0001")
                .parse::<asus_ux8406_keyboard_state::Device>()
                .is_ok()
        );
    }
}

#[rstest]
#[case::load(
    "ACTION==\"add\", IMPORT{program}=\"@LIBEXECDIR@/asus-ux8406-keyboard-state load %k\""
)]
#[case::save("ACTION==\"remove\", RUN+=\"@LIBEXECDIR@/asus-ux8406-keyboard-state save %k\"")]
fn the_rule_runs_the_program_with_the_name_of_the_device(#[case] rule: &str) {
    assert!(RULES.lines().any(|line| line == rule));
}

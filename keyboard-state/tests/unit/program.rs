//! That this crate, the HID-BPF program and the udev rule agree on names and
//! values. Each of the three is read by a different program at run time, and
//! none of them reports a name that the others do not know.

use asus_ux8406_keyboard_state::pins::STATE_MAP;
use asus_ux8406_keyboard_state::sleep::{
    BACKLIGHT_COMMAND, BACKLIGHT_PARAMETER_1, BACKLIGHT_PARAMETER_2, LEVEL_INDEX, REPORT_ID,
    REPORT_SIZE,
};
use asus_ux8406_keyboard_state::state::{
    BACKLIGHT_PROPERTY, FN_LOCK_PROPERTY, MAX_BACKLIGHT, MAX_FN_LOCK,
};
use rstest::rstest;

/// The source of the HID-BPF program.
const PROGRAM: &str =
    include_str!("../../../keyboard-bpf/src/ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.c");
/// The udev rule, before the build fills in the helper's directory.
const RULES: &str = include_str!("../../udev/80-asus-ux8406-keyboard-state.rules.in");
/// The hook of systemd, before the build fills in the helper's directory.
const HOOK: &str = include_str!("../../systemd/asus-ux8406-keyboard-state.sleep.in");
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

    assert_eq!(devices.len(), 4);
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

#[rstest]
#[case::report_id("kHotkeyReportId", usize::from(REPORT_ID))]
#[case::command("kBacklightCommand", usize::from(BACKLIGHT_COMMAND))]
#[case::first_parameter("kBacklightParameter1", usize::from(BACKLIGHT_PARAMETER_1))]
#[case::second_parameter("kBacklightParameter2", usize::from(BACKLIGHT_PARAMETER_2))]
#[case::place_of_the_level("kBacklightLevelIndex", LEVEL_INDEX)]
#[case::size("kFeatureReportSize", REPORT_SIZE)]
fn the_report_is_that_of_the_program(#[case] name: &str, #[case] value: usize) {
    let hexadecimal = format!("{name} = {value:#x},");
    let decimal = format!("{name} = {value},");
    let program = program();

    assert!(
        program.contains(&hexadecimal) || program.contains(&decimal),
        "{name} = {value} in the program"
    );
}

#[test]
fn the_program_fills_the_report_in_the_order_of_this_crate() {
    assert!(program().contains(
        "kHotkeyReportId, kBacklightCommand, kBacklightParameter1, kBacklightParameter2, kBacklightOff,"
    ));
}

#[test]
fn the_hook_runs_the_helper_with_the_words_of_systemd() {
    assert!(
        HOOK.lines()
            .any(|line| line == "exec @LIBEXECDIR@/asus-ux8406-keyboard-state sleep \"$@\"")
    );
    assert!(HOOK.starts_with("#!/bin/sh\n"));
}

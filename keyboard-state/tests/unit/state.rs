//! The state as the value of the map and as udev properties.

use std::path::Path;

use asus_ux8406_keyboard_state::{Error, State};
use rstest::rstest;

use crate::common::BRIGHT_HOTKEYS;

const SOURCE: &str = "/somewhere/state";

fn is_bad_state(error: &Error) -> bool {
    matches!(error, Error::BadState(path) if path == Path::new(SOURCE))
}

#[rstest]
#[case::off_hotkeys([0, 0])]
#[case::bright_function_keys([1, 3])]
#[case::dim_hotkeys([0, 1])]
fn the_values_of_the_map_are_function_row_then_backlight(#[case] value: [u8; 2]) {
    let state = State::from_map(Path::new(SOURCE), value).unwrap();

    assert_eq!([state.fn_lock, state.backlight], value);
}

#[rstest]
#[case::backlight_above_the_brightest([0, 4])]
#[case::function_row_above_its_modes([2, 0])]
#[case::both([255, 255])]
fn values_the_keyboard_does_not_know_are_refused(#[case] value: [u8; 2]) {
    let error = State::from_map(Path::new(SOURCE), value).unwrap_err();

    assert!(is_bad_state(&error));
}

#[test]
fn the_properties_are_those_the_program_reads() {
    let state = State {
        fn_lock: 0,
        backlight: 3,
    };

    assert_eq!(state.properties(), BRIGHT_HOTKEYS);
}

#[rstest]
#[case::off_hotkeys(State { fn_lock: 0, backlight: 0 })]
#[case::bright_function_keys(State { fn_lock: 1, backlight: 3 })]
#[case::middle_hotkeys(State { fn_lock: 0, backlight: 2 })]
fn the_properties_give_the_state_back(#[case] state: State) {
    let read = State::from_properties(Path::new(SOURCE), &state.properties()).unwrap();

    assert_eq!(read, state);
}

#[rstest]
#[case::nothing("")]
#[case::one_property("ASUS_UX8406_KBD_BACKLIGHT=3\n")]
#[case::other_order("ASUS_UX8406_KBD_FN_LOCK=0\nASUS_UX8406_KBD_BACKLIGHT=3\n")]
#[case::a_third_line("ASUS_UX8406_KBD_BACKLIGHT=3\nASUS_UX8406_KBD_FN_LOCK=0\nACTION=remove\n")]
#[case::another_property("ASUS_UX8406_KBD_BACKLIGHT=3\nDEVPATH=0\n")]
#[case::two_digits("ASUS_UX8406_KBD_BACKLIGHT=03\nASUS_UX8406_KBD_FN_LOCK=0\n")]
#[case::a_sign("ASUS_UX8406_KBD_BACKLIGHT=+\nASUS_UX8406_KBD_FN_LOCK=0\n")]
#[case::no_value("ASUS_UX8406_KBD_BACKLIGHT=\nASUS_UX8406_KBD_FN_LOCK=0\n")]
#[case::a_space("ASUS_UX8406_KBD_BACKLIGHT= 3\nASUS_UX8406_KBD_FN_LOCK=0\n")]
#[case::backlight_above_the_brightest("ASUS_UX8406_KBD_BACKLIGHT=4\nASUS_UX8406_KBD_FN_LOCK=0\n")]
#[case::function_row_above_its_modes("ASUS_UX8406_KBD_BACKLIGHT=3\nASUS_UX8406_KBD_FN_LOCK=2\n")]
fn any_other_text_is_refused(#[case] text: &str) {
    let error = State::from_properties(Path::new(SOURCE), text).unwrap_err();

    assert!(is_bad_state(&error));
}

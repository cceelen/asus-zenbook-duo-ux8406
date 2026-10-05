//! Bringing the lower panel's connector in line with the keyboard.

use asus_ux8406_second_screen::{Detection, Error, Mode, release_panel, sync_panel};
use rstest::rstest;

use crate::common::{
    LOWER_STATUS, Sys, UNTOUCHED, UPPER_STATUS, docked, keyboard_on_cable, panels_only,
    upper_panel_only,
};

#[rstest]
fn docking_forces_the_lower_panel_off(docked: Sys) {
    let outcome = sync_panel(&docked.sysfs(), Mode::Apply).unwrap();

    assert_eq!(outcome.detection, Detection::ForcedOff);
    assert_eq!(outcome.status_files, [docked.path(LOWER_STATUS)]);
    assert_eq!(docked.read(LOWER_STATUS), "off");
    assert_eq!(docked.read(UPPER_STATUS), UNTOUCHED);
}

#[rstest]
fn without_the_keyboard_the_panel_is_left_to_detection(panels_only: Sys) {
    let outcome = sync_panel(&panels_only.sysfs(), Mode::Apply).unwrap();

    assert_eq!(outcome.detection, Detection::Automatic);
    assert_eq!(panels_only.read(LOWER_STATUS), "detect");
    assert_eq!(panels_only.read(UPPER_STATUS), UNTOUCHED);
}

#[rstest]
fn releasing_hands_a_docked_panel_back_to_detection(docked: Sys) {
    sync_panel(&docked.sysfs(), Mode::Apply).unwrap();
    assert_eq!(docked.read(LOWER_STATUS), "off");
    let outcome = release_panel(&docked.sysfs(), Mode::Apply).unwrap();

    assert_eq!(outcome.detection, Detection::Automatic);
    assert_eq!(docked.read(LOWER_STATUS), "detect");
    assert_eq!(docked.read(UPPER_STATUS), UNTOUCHED);
}

#[rstest]
fn releasing_on_another_model_is_refused(panels_only: Sys) {
    panels_only.make_other_model();

    assert!(matches!(
        release_panel(&panels_only.sysfs(), Mode::Apply),
        Err(Error::UnsupportedModel)
    ));
    assert_eq!(panels_only.read(LOWER_STATUS), UNTOUCHED);
}

#[rstest]
fn the_cable_leaves_the_panel_on(keyboard_on_cable: Sys) {
    sync_panel(&keyboard_on_cable.sysfs(), Mode::Apply).unwrap();
    assert_eq!(keyboard_on_cable.read(LOWER_STATUS), "detect");
}

#[rstest]
fn a_dry_run_decides_but_writes_nothing(docked: Sys) {
    let outcome = sync_panel(&docked.sysfs(), Mode::DryRun).unwrap();

    assert_eq!(outcome.detection, Detection::ForcedOff);
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
}

#[rstest]
fn another_model_is_refused_before_anything_is_written(docked: Sys) {
    docked.make_other_model();

    assert!(matches!(
        sync_panel(&docked.sysfs(), Mode::Apply),
        Err(Error::UnsupportedModel)
    ));
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
}

#[rstest]
fn a_machine_without_the_lower_panel_is_an_error(upper_panel_only: Sys) {
    assert!(matches!(
        sync_panel(&upper_panel_only.sysfs(), Mode::Apply),
        Err(Error::NoLowerPanel)
    ));
    assert_eq!(upper_panel_only.read(UPPER_STATUS), UNTOUCHED);
}

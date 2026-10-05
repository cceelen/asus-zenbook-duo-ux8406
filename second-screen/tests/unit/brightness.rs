//! Giving the lower panel the brightness of the upper one.

use asus_ux8406_second_screen::{Error, Mode, Panel, sync_brightness};
use rstest::rstest;

use crate::common::{
    LOWER_BRIGHTNESS, SCREENPAD_BRIGHTNESS, Sys, UPPER_BRIGHTNESS, docked, screenpad_only,
};

#[rstest]
fn the_lower_panel_gets_the_brightness_of_the_upper_one(docked: Sys) {
    let brightness = sync_brightness(&docked.sysfs(), Mode::Apply).unwrap();

    assert_eq!(brightness.value, 253);
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "253");
    assert_eq!(docked.read(UPPER_BRIGHTNESS), "253\n");
    assert_eq!(docked.read(SCREENPAD_BRIGHTNESS), "40\n");
}

#[rstest]
fn a_brightness_dry_run_decides_but_writes_nothing(docked: Sys) {
    docked.write(UPPER_BRIGHTNESS, "23\n");
    let brightness = sync_brightness(&docked.sysfs(), Mode::DryRun).unwrap();

    assert_eq!(brightness.value, 23);
    assert!(
        brightness
            .file
            .ends_with("card1-eDP-2-backlight/brightness")
    );
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

#[rstest]
fn a_brightness_above_its_maximum_is_refused(docked: Sys) {
    docked.write(UPPER_BRIGHTNESS, "401\n");

    assert!(matches!(
        sync_brightness(&docked.sysfs(), Mode::Apply),
        Err(Error::BadBrightness(_))
    ));
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

#[rstest]
fn brightness_on_another_model_is_refused(docked: Sys) {
    docked.make_other_model();

    assert!(matches!(
        sync_brightness(&docked.sysfs(), Mode::Apply),
        Err(Error::UnsupportedModel)
    ));
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

#[rstest]
fn a_machine_without_backlights_is_an_error(screenpad_only: Sys) {
    assert!(matches!(
        sync_brightness(&screenpad_only.sysfs(), Mode::Apply),
        Err(Error::NoBacklight(Panel::Upper))
    ));
}

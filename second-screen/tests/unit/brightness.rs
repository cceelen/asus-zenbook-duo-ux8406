//! Giving the lower panel the brightness of the upper one.

use asus_ux8406_second_screen::{Error, Mode, Panel, sync_brightness};
use rstest::rstest;

use crate::common::{
    LOWER_BRIGHTNESS, SCREENPAD_BRIGHTNESS, Sys, UPPER_BRIGHTNESS, docked, max_of, read_only,
    screenpad_only,
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

#[rstest]
fn a_tree_without_the_lower_backlight_is_an_error(docked: Sys) {
    std::fs::remove_file(docked.path("class/backlight/card1-eDP-2-backlight")).unwrap();

    assert!(matches!(
        sync_brightness(&docked.sysfs(), Mode::Apply),
        Err(Error::NoBacklight(Panel::Lower))
    ));
}

#[rstest]
#[case::the_brightness_of_the_upper_panel(UPPER_BRIGHTNESS)]
fn what_cannot_be_read_stops_the_brightness(docked: Sys, #[case] file: &str) {
    docked.write(file, "not a number\n");

    assert!(matches!(
        sync_brightness(&docked.sysfs(), Mode::Apply),
        Err(Error::BadBrightness(_))
    ));
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

#[rstest]
fn the_maximum_of_the_upper_panel_must_be_a_number(docked: Sys) {
    docked.write(&max_of(UPPER_BRIGHTNESS), "x\n");

    assert!(matches!(
        sync_brightness(&docked.sysfs(), Mode::Apply),
        Err(Error::BadBrightness(_))
    ));
}

#[rstest]
fn the_maximum_of_the_lower_panel_must_be_a_number(docked: Sys) {
    docked.write(&max_of(LOWER_BRIGHTNESS), "x\n");

    assert!(matches!(
        sync_brightness(&docked.sysfs(), Mode::Apply),
        Err(Error::BadBrightness(_))
    ));
}

#[rstest]
fn a_lower_brightness_that_cannot_be_written_is_an_error(docked: Sys) {
    read_only(&docked, LOWER_BRIGHTNESS);

    let error = sync_brightness(&docked.sysfs(), Mode::Apply).unwrap_err();

    assert!(matches!(error, Error::Io { path, .. } if path == docked.path(LOWER_BRIGHTNESS)));
}

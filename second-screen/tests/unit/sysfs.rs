//! What asus-ux8406-second-screen reads from and writes to sysfs.

use std::fs;

use asus_ux8406_second_screen::sysfs::{ATTRIBUTE_LIMIT, read_attribute, write_detection};
use asus_ux8406_second_screen::{Detection, Error, Panel};
use rstest::rstest;

use crate::common::{
    LOWER_STATUS, Sys, UNTOUCHED, UPPER_BRIGHTNESS, docked, empty, panels_only, screenpad_only,
    upper_panel_only,
};

#[test]
fn detection_follows_the_dock_state() {
    assert_eq!(Detection::for_keyboard(true), Detection::ForcedOff);
    assert_eq!(Detection::for_keyboard(false), Detection::Automatic);
    assert_eq!(Detection::ForcedOff.to_string(), "off");
    assert_eq!(Detection::Automatic.to_string(), "detect");
}

#[rstest]
fn the_model_is_read_from_the_product_name(panels_only: Sys, docked: Sys, empty: Sys) {
    docked.make_other_model();

    assert!(panels_only.sysfs().is_supported_model());
    assert!(!docked.sysfs().is_supported_model());
    assert!(!empty.sysfs().is_supported_model());
}

#[rstest]
fn the_keyboard_on_the_dock_port_is_docked(docked: Sys) {
    assert!(docked.sysfs().keyboard_docked());
}

#[test]
fn the_keyboard_on_the_cable_is_not_docked() {
    let sys = Sys::from_fixture("ux8406ca-keyboard-on-cable");

    assert!(!sys.sysfs().keyboard_docked());
}

#[test]
fn another_device_on_the_dock_port_is_not_the_keyboard() {
    let product = Sys::from_fixture("usb-other-product-on-dock-port");
    let vendor = Sys::from_fixture("usb-other-vendor-on-dock-port");

    assert!(!product.sysfs().keyboard_docked());
    assert!(!vendor.sysfs().keyboard_docked());
}

#[rstest]
fn a_tree_without_usb_is_not_docked(empty: Sys) {
    assert!(!empty.sysfs().keyboard_docked());
}

#[rstest]
fn an_attribute_is_its_first_line(empty: Sys) {
    empty.write("a", "0b05\nmore\n");
    empty.write("empty", "");

    assert_eq!(read_attribute(&empty.path("a")).as_deref(), Some("0b05"));
    assert_eq!(read_attribute(&empty.path("empty")).as_deref(), Some(""));
    assert_eq!(read_attribute(&empty.path("missing")), None);
}

#[rstest]
fn an_attribute_longer_than_the_limit_is_refused(empty: Sys) {
    empty.write("fits", &"x".repeat(ATTRIBUTE_LIMIT));
    empty.write("long", &"x".repeat(ATTRIBUTE_LIMIT + 1));
    fs::write(empty.path("binary"), [0xff, 0xfe]).unwrap();

    assert!(read_attribute(&empty.path("fits")).is_some());
    assert_eq!(read_attribute(&empty.path("long")), None);
    assert_eq!(read_attribute(&empty.path("binary")), None);
}

#[rstest]
fn each_panel_has_the_backlight_below_its_connector(docked: Sys) {
    docked.write(UPPER_BRIGHTNESS, "182\n");
    let upper = docked.sysfs().backlight(Panel::Upper).unwrap();
    let lower = docked.sysfs().backlight(Panel::Lower).unwrap();

    assert_eq!(upper.brightness().unwrap(), 182);
    assert_eq!(upper.max_brightness().unwrap(), 400);
    assert_eq!(lower.brightness().unwrap(), 170);
    assert!(
        upper
            .brightness_file()
            .ends_with("card1-eDP-1/intel_backlight/brightness")
    );
    assert!(
        lower
            .brightness_file()
            .ends_with("card1-eDP-2/card1-eDP-2-backlight/brightness")
    );
}

#[rstest]
fn a_brightness_is_written_as_a_number(docked: Sys) {
    let lower = docked.sysfs().backlight(Panel::Lower).unwrap();

    lower.set_brightness(182).unwrap();
    assert_eq!(fs::read_to_string(lower.brightness_file()).unwrap(), "182");
    assert_eq!(lower.brightness().unwrap(), 182);
}

#[rstest]
fn a_panel_without_a_backlight_is_an_error(empty: Sys, screenpad_only: Sys) {
    assert!(matches!(
        empty.sysfs().backlight(Panel::Lower),
        Err(Error::Io { .. })
    ));
    assert!(matches!(
        screenpad_only.sysfs().backlight(Panel::Lower),
        Err(Error::NoBacklight(Panel::Lower))
    ));
}

#[rstest]
fn a_brightness_that_is_not_a_number_is_an_error(docked: Sys) {
    let upper = docked.sysfs().backlight(Panel::Upper).unwrap();

    fs::write(upper.brightness_file(), "bright\n").unwrap();
    assert!(matches!(upper.brightness(), Err(Error::BadBrightness(_))));
    fs::write(upper.brightness_file(), "-1\n").unwrap();
    assert!(matches!(upper.brightness(), Err(Error::BadBrightness(_))));
}

#[rstest]
fn only_the_lower_panel_is_listed(panels_only: Sys) {
    assert_eq!(
        panels_only.sysfs().lower_panel_status_files().unwrap(),
        [panels_only.path(LOWER_STATUS)]
    );
}

#[rstest]
fn a_tree_without_the_lower_panel_is_an_error(upper_panel_only: Sys, empty: Sys) {
    assert!(matches!(
        upper_panel_only.sysfs().lower_panel_status_files(),
        Err(Error::NoLowerPanel)
    ));
    assert!(matches!(
        empty.sysfs().lower_panel_status_files(),
        Err(Error::Io { .. })
    ));
}

#[rstest]
fn the_detection_is_written_as_its_word(panels_only: Sys) {
    let status = panels_only.path(LOWER_STATUS);

    assert_eq!(panels_only.read(LOWER_STATUS), UNTOUCHED);
    write_detection(&status, Detection::ForcedOff).unwrap();
    assert_eq!(fs::read_to_string(&status).unwrap(), "off");
    write_detection(&status, Detection::Automatic).unwrap();
    assert_eq!(fs::read_to_string(&status).unwrap(), "detect");
}

#[rstest]
fn writing_to_a_missing_attribute_is_an_error(empty: Sys) {
    assert!(matches!(
        write_detection(&empty.path("nowhere/status"), Detection::ForcedOff),
        Err(Error::Io { .. })
    ));
}

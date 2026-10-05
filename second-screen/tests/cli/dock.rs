//! `asus-ux8406-second-screen dock`.

use rstest::rstest;

use crate::common::{
    DOCKED_KEYBOARD_PRODUCT, LOWER_BRIGHTNESS, LOWER_STATUS, PRODUCT_NAME, SISTER_MODEL, Sys,
    docked,
};

#[rstest]
fn docking_takes_the_panel_away_and_leaves_its_brightness(docked: Sys) {
    docked.program().arg("dock").assert().success();

    assert_eq!(docked.read(LOWER_STATUS), "off");
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

#[rstest]
fn undocking_brings_the_panel_back_at_the_upper_brightness(docked: Sys) {
    docked.program().arg("dock").assert().success();
    docked.undock();
    docked.program().arg("dock").assert().success();

    assert_eq!(docked.read(LOWER_STATUS), "detect");
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "253");
}

#[rstest]
fn the_other_ux8406_and_its_keyboard_count_too(docked: Sys) {
    docked.write(PRODUCT_NAME, SISTER_MODEL);
    docked.write(DOCKED_KEYBOARD_PRODUCT, "1b2c\n");

    docked.program().arg("dock").assert().success();

    assert_eq!(docked.read(LOWER_STATUS), "off");
}

//! `asus-ux8406-second-screen release`.

use rstest::rstest;

use crate::common::{LOWER_BRIGHTNESS, LOWER_STATUS, Sys, docked};

#[rstest]
fn releasing_brings_a_docked_panel_back_without_touching_its_brightness(docked: Sys) {
    docked.program().arg("dock").assert().success();
    assert_eq!(docked.read(LOWER_STATUS), "off");
    docked.program().arg("release").assert().success();

    assert_eq!(docked.read(LOWER_STATUS), "detect");
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

//! `asus-ux8406-second-screen brightness`.

use rstest::rstest;

use crate::common::{LOWER_BRIGHTNESS, LOWER_STATUS, Sys, UNTOUCHED, docked, read_only};

#[rstest]
fn brightness_follows_the_upper_panel_whether_docked_or_not(docked: Sys) {
    docked.program().arg("brightness").assert().success();

    assert_eq!(docked.read(LOWER_BRIGHTNESS), "253");
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
}

#[rstest]
fn a_brightness_that_cannot_be_written_fails_the_dock_command_too(docked: Sys) {
    docked.undock();
    read_only(&docked, LOWER_BRIGHTNESS);
    docked
        .program()
        .arg("dock")
        .assert()
        .failure()
        .stderr(predicates::str::contains("asus-ux8406-second-screen: "));
}

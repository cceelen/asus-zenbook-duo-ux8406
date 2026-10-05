//! `asus-ux8406-second-screen brightness`.

use rstest::rstest;

use crate::common::{LOWER_BRIGHTNESS, LOWER_STATUS, Sys, UNTOUCHED, docked};

#[rstest]
fn brightness_follows_the_upper_panel_whether_docked_or_not(docked: Sys) {
    docked.program().arg("brightness").assert().success();

    assert_eq!(docked.read(LOWER_BRIGHTNESS), "253");
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
}

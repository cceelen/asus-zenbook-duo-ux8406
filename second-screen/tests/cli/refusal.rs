//! A machine that is not a UX8406.

use rstest::rstest;

use crate::common::{LOWER_BRIGHTNESS, LOWER_STATUS, Sys, UNTOUCHED, docked};

#[rstest]
fn a_refusal_is_exit_status_1(
    docked: Sys,
    #[values("dock", "brightness", "release")] action: &str,
) {
    docked.write("class/dmi/id/product_name", "Some Other Laptop\n");

    docked.program().arg(action).assert().code(1);
    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

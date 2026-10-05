//! `asus-ux8406-second-screen -n`.

use predicates::prelude::*;
use rstest::rstest;

use crate::common::{LOWER_BRIGHTNESS, LOWER_STATUS, Sys, UNTOUCHED, docked};

#[rstest]
fn a_dry_run_says_what_it_would_write_and_writes_nothing(docked: Sys) {
    docked.undock();
    docked
        .program()
        .args(["-n", "dock"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("would write \"detect\"")
                .and(predicate::str::contains("would write \"253\"")),
        );

    assert_eq!(docked.read(LOWER_STATUS), UNTOUCHED);
    assert_eq!(docked.read(LOWER_BRIGHTNESS), "170\n");
}

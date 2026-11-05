//! The global shortcut "Toggle auto-rotate", registered with kglobalacceld.

use std::time::Duration;

use asus_ux8406_kde_fakes::{Accel, Bus, Style};

use crate::{Session, panels};

const COMPONENT: &str = "plasma-builtin-screen-rotation";
const ACTION: &str = "ToggleAutoRotate";

fn flag(session: &Session, name: &str) -> bool {
    bool::try_from(session.property(name)).expect("flag")
}

#[test]
fn the_action_is_registered_with_no_key_and_no_default_key() {
    let session = Session::new(panels(), Some("normal"));
    let accel = Accel::new(session.bus.address());
    let program = session.start();

    program.wait_for("the registration", || !accel.keys().is_empty());
    assert_eq!(
        accel.registered(),
        [[
            COMPONENT,
            ACTION,
            "Auto-rotate screens",
            "Toggle auto-rotate"
        ]
        .map(str::to_owned)
        .to_vec()]
    );
    program.wait_for("both calls", || accel.keys().len() == 2);

    let flags: Vec<(u32, usize)> = accel.keys().iter().map(|(_, f, n)| (*f, *n)).collect();

    assert_eq!(flags, [(2, 0), (8, 0)]);
}

#[test]
fn the_shortcut_switches_the_rotation_off_and_on_and_the_journal_says_so() {
    let session = Session::new(panels(), Some("normal"));
    let accel = Accel::new(session.bus.address());
    let program = session.start();

    program.wait_for("the registration", || accel.keys().len() == 2);
    accel.press(COMPONENT, ACTION);
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
    program.wait_log("rotation off (shortcut)");
    accel.press(COMPONENT, ACTION);
    program.wait_for("the switch on", || flag(&session, "Enabled"));
    program.wait_log("rotation on (shortcut)");
}

#[test]
fn another_action_or_component_does_not_switch_anything() {
    let session = Session::new(panels(), Some("normal"));
    let accel = Accel::new(session.bus.address());
    let program = session.start();

    program.wait_for("the registration", || accel.keys().len() == 2);
    accel.press(COMPONENT, "Other");
    accel.press("another-component", ACTION);
    std::thread::sleep(Duration::from_millis(400));
    assert!(flag(&session, "Enabled"));
}

#[test]
fn without_kglobalacceld_the_program_starts_and_registers_when_it_comes() {
    let session = Session::new(panels(), Some("normal"));
    let mut accel = Accel::absent(session.bus.address());
    let program = session.start();

    program.wait_log("global shortcut: not registered");
    accel.start();
    program.wait_for("the registration", || accel.keys().len() == 2);
}

#[test]
fn a_kglobalacceld_that_starts_again_gets_the_action_again() {
    let session = Session::new(panels(), Some("normal"));
    let mut accel = Accel::new(session.bus.address());
    let program = session.start();

    program.wait_for("the registration", || accel.keys().len() == 2);
    accel.stop();
    accel.start();
    program.wait_for("the registration again", || accel.keys().len() == 2);
    accel.press(COMPONENT, ACTION);
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
}

#[test]
fn a_bus_that_takes_no_match_rule_leaves_the_shortcut_out_and_says_so() {
    let session = Session::on(Bus::with_match_rules(2), Style::Registry, panels(), None);
    let program = session.start();

    program.wait_log("no global shortcut: ");
}

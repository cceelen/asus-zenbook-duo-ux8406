//! What the applet in the system tray shows and asks: the switch, or the
//! two buttons where no sensor gives the orientation.

use asus_ux8406_kde_fakes::Output;

use crate::{LOWER, Session, UPPER, panels};

fn flag(session: &Session, name: &str) -> bool {
    bool::try_from(session.property(name)).expect("flag")
}

#[test]
fn with_a_sensor_the_applet_shows_the_switch() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("the switch", || session.control() == "switch");
    assert!(flag(&session, "Enabled"));

    session.call("SetEnabled", &(false,)).expect("call");
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
}

#[test]
fn a_toggle_flips_the_switch_and_the_journal_says_who_asked() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.call("Toggle", &()).expect("call");
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
    program.wait_log("rotation off (Toggle)");
    assert!(session.config().contains("enabled = false"));

    session.call("Toggle", &()).expect("call");
    program.wait_for("the switch on", || flag(&session, "Enabled"));
    program.wait_log("rotation on (Toggle)");

    session.call("SetEnabled", &(false,)).expect("call");
    program.wait_log("rotation off (SetEnabled)");
}

#[test]
fn a_request_for_the_state_that_holds_is_not_a_change() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.call("SetEnabled", &(true,)).expect("call");
    session.call("Toggle", &()).expect("call");
    program.wait_log("rotation off (Toggle)");
    assert!(!program.logged("rotation on"), "{:?}", program.log());
}

#[test]
fn a_configuration_without_the_switch_leaves_the_rotation_on() {
    let session = Session::new(panels(), Some("normal"));

    std::fs::write(session.file(), "[policies]\n").expect("configuration");

    let _program = session.start();

    assert!(flag(&session, "Enabled"));
}

#[test]
fn without_a_sensor_the_buttons_turn_the_screens() {
    let session = Session::new(panels(), None);
    let program = session.start();

    assert_eq!(session.control(), "buttons");
    assert!(flag(&session, "CanTurnClockwise"));
    assert!(flag(&session, "CanTurnCounterclockwise"));

    session.call("Turn", &("clockwise",)).expect("call");
    session.wait_places(&program, (0, 0, 3), (1440, 0, 3));
    // Right side up: no further turn clockwise.
    program.wait_for("the end of the turns", || {
        !flag(&session, "CanTurnClockwise")
    });
    session.call("Turn", &("clockwise",)).expect("call");

    session.call("Turn", &("counterclockwise",)).expect("call");
    session.wait_places(&program, (0, 0, 0), (0, 1440, 0));
    session.call("Turn", &("counterclockwise",)).expect("call");
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    // Left side up: no further turn counterclockwise.
    program.wait_for("the end of the turns", || {
        !flag(&session, "CanTurnCounterclockwise")
    });
}

#[test]
fn a_turn_needs_a_direction() {
    let session = Session::new(panels(), None);
    let _program = session.start();
    let error = session.call("Turn", &("sideways",)).expect_err("refused");

    assert!(
        error.to_string().contains("no direction: sideways"),
        "{error}"
    );
}

#[test]
fn with_a_sensor_the_buttons_do_nothing() {
    let session = Session::new(panels(), Some("normal"));
    let _program = session.start();

    session.call("Turn", &("clockwise",)).expect("call");
    std::thread::sleep(crate::turns::SETTLED);
    assert_eq!(session.place(UPPER), (0, 0, 0));
}

#[test]
fn without_a_built_in_screen_there_is_no_control() {
    let session = Session::new(
        vec![Output::monitor("DP-1", 2560, 1440, 0, 0)],
        Some("left-up"),
    );
    let _program = session.start();

    assert_eq!(session.control(), "none");
    std::thread::sleep(crate::turns::SETTLED);
    assert_eq!(session.compositor.output("DP-1").transform, 0);
}

#[test]
fn the_applet_can_end_the_program() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    // The program can end before the answer is sent.
    let _ = session.call("Quit", &());

    let (status, _) = program.end();

    assert!(status.success());
}

#[test]
fn after_quit_the_program_changes_no_policy() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("never", || session.compositor.output(LOWER).policy == 0);
    // The program can end before the answer is sent.
    let _ = session.call("Quit", &());
    session.compositor.set_policy(UPPER, 2);
    session.compositor.set_policy(LOWER, 2);

    let (status, log) = program.end();

    assert!(status.success());
    assert_eq!(session.compositor.output(UPPER).policy, 2, "{log:?}");
    assert_eq!(session.compositor.output(LOWER).policy, 2, "{log:?}");
}

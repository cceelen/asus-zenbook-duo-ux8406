//! Each built-in screen turned by hand, and what that does to the rotation.

use asus_ux8406_kde_fakes::Output;

use crate::{LOWER, Session, UPPER, panels};

type Listed = Vec<(String, String, String)>;

fn listed(session: &Session) -> Listed {
    Listed::try_from(session.property("Screens")).expect("a list")
}

fn flag(session: &Session, name: &str) -> bool {
    bool::try_from(session.property(name)).expect("flag")
}

fn lower_panel(y: i32) -> Output {
    // With the policy of the program already: it sets none first.
    Output {
        priority: 1,
        policy: 0,
        ..Output::panel(LOWER, 0, y)
    }
}

fn entry(connector: &str, name: &str, orientation: &str) -> (String, String, String) {
    (
        connector.to_owned(),
        name.to_owned(),
        orientation.to_owned(),
    )
}

#[test]
fn the_screens_are_listed_with_their_names_and_orientations() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("the screens", || listed(&session).len() == 2);
    assert_eq!(
        listed(&session),
        [
            entry(UPPER, "Upper screen", "normal"),
            entry(LOWER, "Lower screen", "normal")
        ]
    );
}

#[test]
fn one_screen_is_the_built_in_screen_and_a_mirrored_one_has_no_orientation() {
    let mut mirrored = Output::panel(UPPER, 0, 0);

    mirrored.transform = 4;

    let session = Session::new(vec![mirrored], None);
    let program = session.start();

    program.wait_for("the screens", || !listed(&session).is_empty());
    assert_eq!(listed(&session), [entry(UPPER, "Built-in screen", "")]);
}

#[test]
fn the_lower_screen_turned_upside_down_holds_the_rotation_for_the_session() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("the claim", || session.sensor().claims() == (1, 0));
    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(LOWER) == (0, 1440, 2));
    assert_eq!(session.place(UPPER), (0, 0, 0));
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
    program.wait_log("eDP-2: turned to bottom-up by hand (TurnScreen)");
    program.wait_log("rotation off (TurnScreen)");
    program.wait_for("the release", || session.sensor().claims() == (1, 1));
    program.wait_for("the list", || {
        listed(&session).contains(&entry(LOWER, "Lower screen", "bottom-up"))
    });
    // The saved setting does not change.
    assert!(!session.config().contains("enabled = false"));

    // The sensor does not undo the turn.
    session.sensor().turn("left-up");
    std::thread::sleep(crate::turns::SETTLED);
    assert_eq!(session.place(LOWER), (0, 1440, 2));
    assert_eq!(session.place(UPPER), (0, 0, 0));
}

#[test]
fn switching_on_hands_all_screens_back_to_the_sensor() {
    let session = Session::new(panels(), Some("left-up"));
    let program = session.start();

    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
    session.call("SetEnabled", &(true,)).expect("call");
    program.wait_log("rotation on (SetEnabled)");
    program.wait_log("SetEnabled: the screens go back under the sensor");
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    assert!(flag(&session, "Enabled"));
}

#[test]
fn a_toggle_resumes_too_and_switching_off_when_held_changes_nothing() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session
        .call("TurnScreen", &(UPPER, "left-up"))
        .expect("call");
    program.wait_for("the switch off", || !flag(&session, "Enabled"));
    session.call("SetEnabled", &(false,)).expect("call");
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!flag(&session, "Enabled"));
    session.call("Toggle", &()).expect("call");
    program.wait_for("the switch on", || flag(&session, "Enabled"));
    program.wait_log("rotation on (Toggle)");
    session.wait_places(&program, (0, 0, 0), (0, 1440, 0));
}

#[test]
fn a_screen_that_is_off_and_comes_back_is_turned_again() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(LOWER) == (0, 1440, 2));
    session.compositor.unplug(LOWER);
    program.wait_for("one screen", || listed(&session).len() == 1);
    session.compositor.plug(lower_panel(1440));
    program.wait_log("eDP-2: turned to bottom-up again (the monitors changed)");
    program.wait_for("the screen turned again", || {
        session.place(LOWER) == (0, 1440, 2)
    });
    assert_eq!(session.place(UPPER), (0, 0, 0));
}

#[test]
fn a_turn_that_kwin_refuses_is_not_held() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("never", || session.compositor.output(LOWER).policy == 0);
    session.compositor.refuse_next("no");
    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_log("KWin did not take the turn: no");
    program.wait_for("the switch on again", || flag(&session, "Enabled"));
    assert_eq!(session.place(LOWER), (0, 1440, 0));
}

#[test]
fn a_turn_again_that_kwin_refuses_drops_the_hold_of_that_screen() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(LOWER) == (0, 1440, 2));
    session.compositor.unplug(LOWER);
    program.wait_for("one screen", || listed(&session).len() == 1);
    session.compositor.refuse_next("no");
    session.compositor.plug(lower_panel(1440));
    program.wait_log("eDP-2 is not held any more");
    assert_eq!(session.place(LOWER), (0, 1440, 0));
}

#[test]
fn the_last_screen_that_is_not_held_any_more_hands_the_screens_to_the_sensor() {
    let session = Session::new(vec![Output::panel(UPPER, 0, 0)], Some("normal"));
    let program = session.start();

    session
        .call("TurnScreen", &(UPPER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(UPPER) == (0, 0, 2));
    session.compositor.unplug(UPPER);
    program.wait_for("no screen", || listed(&session).is_empty());
    session.compositor.refuse_next("no");
    session.compositor.plug(Output {
        policy: 0,
        ..Output::panel(UPPER, 0, 0)
    });
    program.wait_log("eDP-1 is not held any more");
    program.wait_for("the switch on", || flag(&session, "Enabled"));
}

#[test]
fn an_orientation_that_is_none_and_a_screen_that_is_not_there_are_refused() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();
    let error = session
        .call("TurnScreen", &(LOWER, "sideways"))
        .expect_err("refused");

    assert!(error.to_string().contains("no orientation: sideways"));
    session
        .call("TurnScreen", &("DP-9", "left-up"))
        .expect("call");
    program.wait_log("DP-9: cannot be turned to left-up now");
    assert!(flag(&session, "Enabled"));
}

#[test]
fn a_second_turn_while_the_first_waits_for_kwin_is_refused() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("never", || session.compositor.output(LOWER).policy == 0);
    session.compositor.ignore_next_apply();
    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_log("eDP-2: turned to bottom-up by hand");
    session
        .call("TurnScreen", &(UPPER, "left-up"))
        .expect("call");
    program.wait_log("eDP-1: cannot be turned to left-up now");
}

#[test]
fn the_buttons_turn_all_screens_together_and_end_the_hold() {
    let session = Session::new(panels(), None);
    let program = session.start();

    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(LOWER) == (0, 1440, 2));
    session.call("Turn", &("clockwise",)).expect("call");
    session.wait_places(&program, (0, 0, 3), (1440, 0, 3));
}

#[test]
fn two_held_screens_that_the_compositor_resets_are_turned_again_one_after_the_other() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the lower screen turned", || {
        session.place(LOWER) == (0, 1440, 2)
    });
    session
        .call("TurnScreen", &(UPPER, "left-up"))
        .expect("call");
    program.wait_for("the upper screen turned", || session.place(UPPER).2 == 1);
    session.compositor.set_transform(UPPER, 0);
    session.compositor.set_transform(LOWER, 0);
    program.wait_for("both turned again", || {
        session.place(UPPER).2 == 1 && session.place(LOWER).2 == 2
    });
}

#[test]
fn after_a_screen_turned_upside_down_both_buttons_of_90_degrees_still_work() {
    let session = Session::new(panels(), None);
    let program = session.start();

    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(LOWER) == (0, 1440, 2));
    program.wait_for("both buttons", || {
        flag(&session, "CanTurnClockwise") && flag(&session, "CanTurnCounterclockwise")
    });
    session.call("Turn", &("clockwise",)).expect("call");
    session.wait_places(&program, (0, 0, 3), (1440, 0, 3));
}

#[test]
fn a_screen_that_comes_back_after_quit_is_not_turned_again() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session
        .call("TurnScreen", &(LOWER, "bottom-up"))
        .expect("call");
    program.wait_for("the screen turned", || session.place(LOWER) == (0, 1440, 2));
    session.compositor.unplug(LOWER);
    program.wait_for("one screen", || listed(&session).len() == 1);
    // The program can end before it answers.
    let _ = session.call("Quit", &());
    session.compositor.plug(lower_panel(1440));

    let (status, log) = program.end();

    assert!(status.success());
    assert_eq!(session.place(LOWER), (0, 1440, 0), "{log:?}");
}

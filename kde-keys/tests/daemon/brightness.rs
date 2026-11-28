//! The lower screen has the brightness of the upper screen.

use asus_ux8406_kde_fakes::{Compositor, Output};

use crate::{LOWER, Session, UPPER, panels, touch};

fn lower_follows(compositor: &Compositor, level: u32) {
    compositor.wait_for("the lower brightness", |c| {
        c.output(LOWER).brightness == level
    });
}

#[test]
fn the_lower_screen_follows_the_upper_screen() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();
    let compositor = &session.compositor;

    compositor.set_brightness(UPPER, 3000);
    lower_follows(compositor, 3000);
    program.wait_log("eDP-2: brightness 3000, as eDP-1");

    // The change of the lower screen is no reason for another one.
    compositor.set_brightness(UPPER, 3000);
    compositor.set_brightness(UPPER, 7000);
    lower_follows(compositor, 7000);
    assert_eq!(
        compositor.requests(),
        [
            format!("{LOWER} brightness 3000"),
            "apply".to_owned(),
            format!("{LOWER} brightness 7000"),
            "apply".to_owned(),
        ]
    );
}

#[test]
fn equal_screens_get_no_request() {
    let session = Session::new(panels(), Some(touch()));
    let _program = session.start();

    session.compositor.set_brightness(UPPER, 5000);
    session.compositor.set_brightness(LOWER, 5000);
    session
        .compositor
        .wait_for("the brightness", |c| c.output(LOWER).brightness == 5000);
    assert_eq!(session.compositor.requests(), Vec::<String>::new());
}

#[test]
fn a_lower_screen_that_comes_gets_the_brightness_of_the_upper_screen() {
    let session = Session::new(vec![Output::panel(UPPER, 0, 0)], Some(touch()));
    let program = session.start();

    session.compositor.plug(Output {
        brightness: 9000,
        ..Output::panel(LOWER, 0, 1440)
    });
    session.compositor.wait_for("the lower brightness", |c| {
        c.outputs()
            .iter()
            .any(|output| output.name == LOWER && output.brightness == 5000)
    });
    program.wait_log("eDP-2: brightness 5000, as eDP-1");
}

#[test]
fn a_lower_screen_that_returns_gets_the_brightness_of_the_upper_screen() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();
    let compositor = &session.compositor;

    session.press();
    compositor.wait_for("the lower panel off", |c| !c.output(LOWER).enabled);

    // KWin keeps no change of an output that is off.
    compositor.set_brightness(UPPER, 2000);
    session.press();
    lower_follows(compositor, 2000);
    program.wait_log("eDP-2: brightness 2000, as eDP-1");
    assert!(compositor.output(LOWER).enabled);
}

#[test]
fn a_lower_screen_that_is_off_is_left_alone() {
    let session = Session::new(
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                enabled: false,
                ..Output::panel(LOWER, 0, 1440)
            },
        ],
        Some(touch()),
    );
    let _program = session.start();

    session.compositor.set_brightness(UPPER, 2000);
    session
        .compositor
        .wait_for("the brightness", |c| c.output(UPPER).brightness == 2000);
    assert_eq!(session.compositor.requests(), Vec::<String>::new());
    assert_eq!(session.compositor.output(LOWER).brightness, 5000);
}

#[test]
fn a_lower_screen_without_the_capability_is_left_alone() {
    let session = Session::new(
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                can_set_brightness: false,
                ..Output::panel(LOWER, 0, 1440)
            },
        ],
        Some(touch()),
    );
    let _program = session.start();

    session.compositor.set_brightness(UPPER, 2000);
    session
        .compositor
        .wait_for("the brightness", |c| c.output(UPPER).brightness == 2000);
    assert_eq!(session.compositor.requests(), Vec::<String>::new());
}

#[test]
fn a_brightness_that_kwin_refuses_is_logged() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();

    session.compositor.refuse_next("no");
    session.compositor.set_brightness(UPPER, 2000);
    program.wait_log("KWin did not take the brightness: no");
    assert_eq!(
        session.compositor.requests(),
        [format!("{LOWER} brightness 2000"), "apply".to_owned()]
    );
}

#[test]
fn no_second_request_waits_for_the_answer() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();
    let compositor = &session.compositor;

    compositor.ignore_next_apply();
    compositor.set_brightness(UPPER, 2000);
    program.wait_log("eDP-2: brightness 2000, as eDP-1");
    compositor.set_brightness(UPPER, 3000);
    compositor.wait_for("the upper brightness", |c| {
        c.output(UPPER).brightness == 3000
    });
    program.wait_for("the program has read it", || {
        compositor.requests() == [format!("{LOWER} brightness 2000"), "apply".to_owned()]
    });
}

#[test]
fn a_change_while_kwin_holds_the_answer_is_not_lost() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();
    let compositor = &session.compositor;

    compositor.hold_next_apply();
    compositor.set_brightness(UPPER, 2000);
    program.wait_log("eDP-2: brightness 2000, as eDP-1");
    compositor.set_brightness(UPPER, 3000);
    compositor.wait_for("the upper brightness", |c| {
        c.output(UPPER).brightness == 3000
    });
    compositor.answer_held();
    lower_follows(compositor, 3000);
    program.wait_log("eDP-2: brightness 3000, as eDP-1");
}

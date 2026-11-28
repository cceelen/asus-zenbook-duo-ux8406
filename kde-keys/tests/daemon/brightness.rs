//! The lower screen has the brightness of the upper screen.

use asus_ux8406_kde_fakes::input::Device;
use asus_ux8406_kde_fakes::{Compositor, Output};

use crate::{LOWER, Session, UPPER, panels, touch};

fn lower_follows(compositor: &Compositor, level: u32) {
    compositor.wait_for("the lower brightness", |c| {
        c.outputs()
            .iter()
            .any(|output| output.name == LOWER && output.brightness == level)
    });
}

/// The requests that set the brightness of the lower screen once.
fn follow_requests(level: u32) -> Vec<String> {
    vec![format!("{LOWER} brightness {level}"), "apply".to_owned()]
}

/// Take the lower screen away and put this one in its place.
fn replace_lower(compositor: &Compositor, lower: Output) {
    compositor.unplug(LOWER);
    compositor.plug(lower);
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
    let program = session.start();
    let compositor = &session.compositor;

    compositor.set_brightness(UPPER, 5000);
    compositor.set_brightness(LOWER, 5000);
    compositor.set_brightness(UPPER, 2000);
    lower_follows(compositor, 2000);
    program.wait_log("eDP-2: brightness 2000, as eDP-1");
    assert_eq!(compositor.requests(), follow_requests(2000));
}

#[test]
fn the_lower_screen_that_differs_at_the_start_follows() {
    let session = Session::new(
        vec![
            Output {
                brightness: 3000,
                priority: 1,
                ..Output::panel(UPPER, 0, 0)
            },
            Output::panel(LOWER, 0, 1440),
        ],
        Some(touch()),
    );
    let program = session.start();

    lower_follows(&session.compositor, 3000);
    program.wait_log("eDP-2: brightness 3000, as eDP-1");
    assert_eq!(session.compositor.requests(), follow_requests(3000));
}

#[test]
fn a_lower_screen_that_alone_changes_is_set_back() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();

    session.compositor.set_brightness(LOWER, 8000);
    lower_follows(&session.compositor, 5000);
    program.wait_log("eDP-2: brightness 5000, as eDP-1");
    assert_eq!(session.compositor.requests(), follow_requests(5000));
}

#[test]
fn a_lower_screen_that_comes_gets_the_brightness_of_the_upper_screen() {
    let session = Session::new(vec![Output::panel(UPPER, 0, 0)], Some(touch()));
    let program = session.start();

    session.compositor.plug(Output {
        brightness: 9000,
        ..Output::panel(LOWER, 0, 1440)
    });
    lower_follows(&session.compositor, 5000);
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
    let program = session.start();
    let compositor = &session.compositor;

    compositor.set_brightness(UPPER, 2000);
    replace_lower(compositor, Output::panel(LOWER, 0, 1440));
    lower_follows(compositor, 2000);
    program.wait_log("eDP-2: brightness 2000, as eDP-1");
    assert_eq!(compositor.requests(), follow_requests(2000));
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
    let program = session.start();
    let compositor = &session.compositor;

    compositor.set_brightness(UPPER, 2000);
    replace_lower(compositor, Output::panel(LOWER, 0, 1440));
    lower_follows(compositor, 2000);
    program.wait_log("eDP-2: brightness 2000, as eDP-1");
    assert_eq!(compositor.requests(), follow_requests(2000));
}

#[test]
fn a_brightness_that_kwin_refuses_is_logged_and_not_sent_again() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();
    let compositor = &session.compositor;

    compositor.refuse_next("no");
    compositor.set_brightness(UPPER, 2000);
    program.wait_log("KWin did not take the brightness: no");

    // The program handles a later command only after the answer, and a
    // second request would have left before it.
    session.input().add(Device::pen("event20", 0x4448));
    program.wait_for("the pen on its panel", || {
        session.input().output_name("event20") == LOWER
    });
    assert_eq!(compositor.requests(), follow_requests(2000));

    // The next change of the outputs is the next try.
    compositor.plug(Output::monitor("DP-1", 1920, 1080, 2304, 0));
    lower_follows(compositor, 2000);
    assert_eq!(
        compositor.requests(),
        [follow_requests(2000), follow_requests(2000)].concat()
    );
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
    assert_eq!(
        compositor.requests(),
        [follow_requests(2000), follow_requests(3000)].concat()
    );
}

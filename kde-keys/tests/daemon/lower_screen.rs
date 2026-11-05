//! The key right of F12: the lower screen off and on again.

use asus_ux8406_kde_fakes::Output;

use crate::{LOWER, Session, UPPER, panels, touch};

#[test]
fn the_key_switches_the_lower_screen_off_and_on() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();
    let compositor = &session.compositor;

    session.press();
    compositor.wait_for("the lower panel off", |c| !c.output(LOWER).enabled);
    assert_eq!(
        compositor.requests(),
        [
            format!("{UPPER} position 0 0"),
            format!("{LOWER} enable 0"),
            "apply".to_owned(),
        ]
    );

    // The remembered layout comes back.
    session.press();
    compositor.wait_for("the lower panel on", |c| c.output(LOWER).enabled);
    assert_eq!(
        (compositor.output(LOWER).x, compositor.output(LOWER).y),
        (0, 1440)
    );

    let (status, _) = program.stop();

    assert!(status.success());
}

#[test]
fn without_a_remembered_layout_the_lower_screen_goes_below() {
    let session = Session::new(
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                enabled: false,
                scale: 1.0,
                ..Output::panel(LOWER, 0, 0)
            },
        ],
        Some(touch()),
    );
    let program = session.start();

    session.press();
    session
        .compositor
        .wait_for("the lower panel on", |c| c.output(LOWER).enabled);

    let lower = session.compositor.output(LOWER);

    // At the scale of the upper panel, below it.
    assert_eq!((lower.x, lower.y, lower.scale), (0, 1440, 1.25));
    drop(program);
}

#[test]
fn with_the_keyboard_on_the_lower_screen_the_key_does_nothing() {
    let session = Session::new(vec![Output::panel(UPPER, 0, 0)], Some(touch()));
    let program = session.start();

    session.press();
    program.wait_log("nothing to do");
    assert_eq!(session.compositor.requests(), Vec::<String>::new());
}

#[test]
fn a_layout_that_kwin_refuses_is_not_remembered() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();

    session.compositor.refuse_next("outputs overlap");
    session.press();
    program.wait_log("KWin did not take the layout: outputs overlap");
    assert!(session.compositor.output(LOWER).enabled);

    // The next press tries again.
    session.press();
    session
        .compositor
        .wait_for("the lower panel off", |c| !c.output(LOWER).enabled);
}

#[test]
fn the_program_ends_with_kwin() {
    let session = Session::new(panels(), Some(touch()));
    let program = session.start();

    session.compositor.kick();

    let (status, log) = program.end();

    assert_eq!(status.code(), Some(1));
    assert!(log.iter().any(|line| line.contains("Wayland")), "{log:?}");
}

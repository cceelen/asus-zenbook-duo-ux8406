//! The mode of an output that changes while the client runs.

use asus_ux8406_kde_common::geometry::Size;
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use rstest::rstest;

use crate::{STYLES, UPPER, session, until};

const SMALLER: Size = Size {
    width: 1920,
    height: 1200,
};

fn size_of_upper(session: &asus_ux8406_kde_common::output::Session) -> Option<Size> {
    session
        .outputs
        .state()
        .connected
        .get(UPPER)
        .copied()
        .flatten()
}

#[rstest]
fn a_new_current_mode_gives_the_new_size(#[values(STYLES[0], STYLES[1])] style: Style) {
    let compositor = Compositor::new(style, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);

    compositor.resize(UPPER, SMALLER.width, SMALLER.height);
    until(&mut session, "the new size", |s| {
        size_of_upper(s) == Some(SMALLER)
    });
}

#[rstest]
fn a_new_preferred_mode_gives_the_new_size_when_there_is_no_current_mode(
    #[values(STYLES[0], STYLES[1])] style: Style,
) {
    let compositor = Compositor::new(
        style,
        vec![Output {
            has_current_mode: false,
            ..Output::panel(UPPER, 0, 0)
        }],
    );
    let mut session = session(&compositor);

    compositor.resize(UPPER, SMALLER.width, SMALLER.height);
    until(&mut session, "the new size", |s| {
        size_of_upper(s) == Some(SMALLER)
    });
}

#[test]
fn a_mode_that_goes_is_released_where_the_protocol_has_the_request() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);

    compositor.resize(UPPER, SMALLER.width, SMALLER.height);
    until(&mut session, "the mode released", |_| {
        compositor.mode_releases() == 1
    });
}

#[test]
fn a_mode_that_goes_is_not_released_where_the_protocol_has_no_such_request() {
    let compositor = Compositor::new(Style::Globals, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);

    compositor.resize(UPPER, SMALLER.width, SMALLER.height);
    until(&mut session, "the new size", |s| {
        size_of_upper(s) == Some(SMALLER)
    });
    assert_eq!(compositor.mode_releases(), 0);
}

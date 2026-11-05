//! Outputs that come and go while the client runs.

use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use rstest::rstest;

use crate::{LOWER, STYLES, UPPER, changed, panels, session, until};

#[rstest]
fn an_output_that_goes_is_no_longer_connected(#[values(STYLES[0], STYLES[1])] style: Style) {
    let compositor = Compositor::new(style, panels());
    let mut session = session(&compositor);

    compositor.unplug(LOWER);
    until(&mut session, "the lower panel gone", |s| {
        !s.outputs.state().connected.contains_key(LOWER)
    });
    assert!(changed(&session.take_events()));
    assert_eq!(session.outputs.state().logical.len(), 1);
}

#[rstest]
fn an_output_that_comes_is_described(#[values(STYLES[0], STYLES[1])] style: Style) {
    let compositor = Compositor::new(style, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);

    compositor.plug(Output::panel(LOWER, 0, 1440));
    until(&mut session, "the lower panel back", |s| {
        s.outputs.state().logical.len() == 2
    });
    assert!(changed(&session.take_events()));
}

#[test]
fn other_globals_and_the_end_of_the_registry_change_nothing() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);
    let before = session.outputs.state();

    compositor.add_global();
    compositor.finish();
    compositor.set_policy(UPPER, 0);
    // The policy change comes after the two: once it is in, they are too.
    until(&mut session, "the policy", |s| {
        s.outputs.policies()[0].1 == asus_ux8406_kde_common::output::Policy::Never
    });
    assert_eq!(session.outputs.state(), before);
}

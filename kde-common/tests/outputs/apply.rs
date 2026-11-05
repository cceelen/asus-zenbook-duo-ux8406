//! Layouts given to KWin: the requests, and KWin's answer.

use asus_ux8406_kde_common::geometry::Monitor;
use asus_ux8406_kde_common::output::Event;
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use rstest::rstest;

use crate::{LOWER, STYLES, UPPER, apply, changed, panels, session, until};

fn monitor(connector: &str, x: i32, y: i32, transform: u32, scale: f64) -> Monitor {
    Monitor {
        x,
        y,
        scale,
        transform,
        primary: false,
        connectors: vec![connector.to_owned()],
    }
}

#[rstest]
fn an_output_left_out_goes_off(#[values(STYLES[0], STYLES[1])] style: Style) {
    let compositor = Compositor::new(style, panels());
    let mut session = session(&compositor);

    assert_eq!(
        apply(&mut session, &[monitor(UPPER, 0, 0, 0, 1.25)]),
        Ok(())
    );
    assert_eq!(
        compositor.requests(),
        [
            format!("{UPPER} position 0 0"),
            format!("{LOWER} enable 0"),
            "apply".to_owned()
        ]
    );
    assert!(!compositor.output(LOWER).enabled);
    until(&mut session, "the lower panel off", |s| {
        s.outputs.state().logical.len() == 1
    });
    assert!(changed(&session.take_events()));
}

#[test]
fn an_output_put_back_goes_on_in_its_mode() {
    let compositor = Compositor::new(
        Style::Registry,
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                enabled: false,
                ..Output::panel(LOWER, 0, 0)
            },
        ],
    );
    let mut session = session(&compositor);
    let layout = [
        monitor(UPPER, 0, 0, 0, 1.25),
        monitor(LOWER, 0, 1440, 0, 1.25),
    ];

    assert_eq!(apply(&mut session, &layout), Ok(()));
    assert_eq!(
        compositor.requests()[..3],
        [
            format!("{UPPER} position 0 0"),
            format!("{LOWER} enable 1"),
            format!("{LOWER} mode"),
        ]
    );
    assert_eq!(compositor.output(LOWER).y, 1440);
}

#[test]
fn transform_and_scale_are_sent_where_they_change() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);
    let layout = [
        monitor(UPPER, 1440, 0, 1, 1.25),
        monitor(LOWER, 0, 0, 1, 1.5),
    ];

    assert_eq!(apply(&mut session, &layout), Ok(()));
    assert_eq!(
        compositor.requests(),
        [
            format!("{UPPER} position 1440 0"),
            format!("{UPPER} transform 1"),
            format!("{LOWER} position 0 0"),
            format!("{LOWER} transform 1"),
            format!("{LOWER} scale 1.5"),
            "apply".to_owned(),
        ]
    );
    assert_eq!(compositor.output(LOWER).scale, 1.5);
}

#[test]
fn a_replica_and_an_output_that_is_off_are_left_alone() {
    let compositor = Compositor::new(
        Style::Registry,
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                replication_source: format!("uuid-{UPPER}"),
                ..Output::panel(LOWER, 0, 0)
            },
            Output {
                enabled: false,
                ..Output::monitor("DP-1", 1920, 1080, 0, 0)
            },
        ],
    );
    let mut session = session(&compositor);
    let mirrored = Monitor {
        connectors: vec![UPPER.to_owned(), LOWER.to_owned()],
        ..monitor(UPPER, 0, 0, 0, 1.25)
    };

    assert_eq!(apply(&mut session, &[mirrored]), Ok(()));
    assert_eq!(
        compositor.requests(),
        [format!("{UPPER} position 0 0"), "apply".to_owned()]
    );
}

#[rstest]
#[case::with_a_reason(Style::Registry, "outputs overlap")]
#[case::kwin_6_6(Style::Globals, "outputs overlap")]
// Before version 12 KWin gives no reason.
#[case::without_a_reason(Style::Old, "")]
fn a_refused_layout_comes_back_as_failed(#[case] style: Style, #[case] reason: &str) {
    let compositor = Compositor::new(style, panels());
    let mut session = session(&compositor);

    compositor.refuse_next("outputs overlap");
    assert_eq!(
        apply(&mut session, &[monitor(UPPER, 0, 0, 0, 1.25)]),
        Err(reason.to_owned())
    );
    assert!(compositor.output(LOWER).enabled);
}

#[test]
fn the_answers_come_as_events_with_their_numbers() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);
    let layout = [monitor(UPPER, 0, 0, 0, 1.25)];

    compositor.refuse_next("no");

    let first = session.apply(&layout);
    let second = session.apply(&layout);

    session.flush().expect("flush");
    // Wait for the second one; the answer to the first one stays for
    // take_events.
    assert_eq!(crate::wait(&mut session, second).expect("answer"), Ok(()));
    assert_ne!(first, second);
    assert!(
        session
            .take_events()
            .contains(&Event::Failed(first, "no".to_owned()))
    );
}

#[test]
fn an_answer_that_is_granted_to_an_earlier_layout_stays_too() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);
    let layout = [monitor(UPPER, 0, 0, 0, 1.25)];
    let first = session.apply(&layout);
    let second = session.apply(&layout);

    session.flush().expect("flush");
    assert_eq!(crate::wait(&mut session, second).expect("answer"), Ok(()));
    assert!(session.take_events().contains(&Event::Applied(first)));
}

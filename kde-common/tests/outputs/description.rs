//! What the client makes of the outputs that KWin describes.

use asus_ux8406_kde_common::geometry::{Monitor, Size};
use asus_ux8406_kde_common::output::is_builtin;
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use rstest::rstest;

use crate::{LOWER, UPPER, panels, session};

const PANEL: Size = Size {
    width: 2880,
    height: 1800,
};

#[rstest]
fn two_panels_are_two_built_in_monitors(
    #[values(Style::Registry, Style::Globals, Style::Old)] style: Style,
) {
    let compositor = Compositor::new(style, panels());
    let state = session(&compositor).outputs.state();

    assert!(state.logical_mode);
    assert_eq!(state.builtin, [UPPER, LOWER]);
    assert_eq!(state.connected.get(UPPER), Some(&Some(PANEL)));
    assert_eq!(
        state.logical,
        [
            Monitor {
                x: 0,
                y: 0,
                scale: 1.25,
                transform: 0,
                // KWin 5 has no priority: no output is the primary.
                primary: style != Style::Old,
                connectors: vec![UPPER.to_owned()],
            },
            Monitor {
                x: 0,
                y: 1440,
                scale: 1.25,
                transform: 0,
                primary: false,
                connectors: vec![LOWER.to_owned()],
            },
        ]
    );
}

#[test]
fn an_output_that_is_off_is_connected_but_not_shown() {
    let compositor = Compositor::new(
        Style::Registry,
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                enabled: false,
                ..Output::panel(LOWER, 0, 1440)
            },
        ],
    );
    let state = session(&compositor).outputs.state();

    assert_eq!(state.connected.get(LOWER), Some(&Some(PANEL)));
    assert_eq!(state.logical.len(), 1);
}

#[test]
fn without_a_current_mode_the_preferred_one_counts() {
    let compositor = Compositor::new(
        Style::Registry,
        vec![Output {
            has_current_mode: false,
            ..Output::panel(UPPER, 0, 0)
        }],
    );

    assert_eq!(
        session(&compositor).outputs.state().connected.get(UPPER),
        Some(&Some(PANEL))
    );
}

#[test]
fn a_replica_is_in_the_monitor_of_its_source() {
    let compositor = Compositor::new(
        Style::Registry,
        vec![
            Output::panel(UPPER, 0, 0),
            Output {
                replication_source: format!("uuid-{UPPER}"),
                ..Output::panel(LOWER, 0, 0)
            },
            Output {
                replication_source: "uuid-of-nothing".to_owned(),
                ..Output::monitor("DP-1", 1920, 1080, 2304, 0)
            },
        ],
    );
    let state = session(&compositor).outputs.state();

    assert_eq!(state.logical[0].connectors, [UPPER, LOWER]);
    // A source that is not there: the replica is a monitor by itself.
    assert_eq!(state.logical[1].connectors, ["DP-1"]);
    assert_eq!((state.logical[1].x, state.logical[1].scale), (0, 1.0));
}

#[test]
fn a_turned_output_has_its_transform() {
    let compositor = Compositor::new(
        Style::Globals,
        vec![Output {
            transform: 3,
            ..Output::panel(UPPER, 0, 0)
        }],
    );

    assert_eq!(session(&compositor).outputs.state().logical[0].transform, 3);
}

#[rstest]
#[case("eDP-1", true)]
#[case("LVDS-1", true)]
#[case("DSI-1", true)]
#[case("DP-1", false)]
#[case("HDMI-A-1", false)]
fn built_in_connectors(#[case] connector: &str, #[case] builtin: bool) {
    assert_eq!(is_builtin(connector), builtin);
}

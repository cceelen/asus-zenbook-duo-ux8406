//! The brightness of the outputs.

use asus_ux8406_kde_common::brightness::Panel;
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use rstest::rstest;

use crate::{LOWER, UPPER, panels, session, until, wait};

#[rstest]
fn the_brightness_of_each_output_is_read(#[values(Style::Registry, Style::Globals)] style: Style) {
    let compositor = Compositor::new(
        style,
        vec![
            Output {
                brightness: 3000,
                ..Output::panel(UPPER, 0, 0)
            },
            Output {
                enabled: false,
                ..Output::panel(LOWER, 0, 1440)
            },
        ],
    );
    let session = session(&compositor);

    assert_eq!(
        session.outputs.panel(UPPER),
        Some(Panel {
            enabled: true,
            level: Some(3000)
        })
    );
    assert_eq!(
        session.outputs.panel(LOWER),
        Some(Panel {
            enabled: false,
            level: Some(5000)
        })
    );
    assert_eq!(session.outputs.panel("DP-1"), None);
}

#[test]
fn an_output_that_cannot_set_its_brightness_has_none() {
    let compositor = Compositor::new(
        Style::Registry,
        vec![Output {
            can_set_brightness: false,
            ..Output::panel(UPPER, 0, 0)
        }],
    );

    assert_eq!(
        session(&compositor).outputs.panel(UPPER),
        Some(Panel {
            enabled: true,
            level: None
        })
    );
}

#[test]
fn an_old_kwin_tells_no_brightness() {
    let compositor = Compositor::new(Style::Old, panels());
    let mut session = session(&compositor);

    assert_eq!(session.outputs.panel(UPPER).and_then(|p| p.level), None);
    assert_eq!(session.set_brightness(LOWER, 3000), None);
}

#[test]
fn a_brightness_is_set_on_the_output_named() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);
    let number = session
        .set_brightness(LOWER, 3000)
        .expect("KWin 6.8 sets the brightness");

    session.flush().expect("flush");
    assert_eq!(wait(&mut session, number).expect("answer"), Ok(()));
    assert_eq!(
        compositor.requests(),
        [format!("{LOWER} brightness 3000"), "apply".to_owned()]
    );
    until(&mut session, "the new brightness", |s| {
        s.outputs.panel(LOWER).and_then(|p| p.level) == Some(3000)
    });
    assert_eq!(compositor.output(UPPER).brightness, 5000);
}

#[test]
fn the_brightness_of_an_unknown_output_is_not_set() {
    let compositor = Compositor::new(Style::Registry, panels());

    assert_eq!(session(&compositor).set_brightness("DP-1", 3000), None);
}

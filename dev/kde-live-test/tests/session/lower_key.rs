//! The key right of F12, and the keyboard on the lower screen.

use asus_ux8406_kde_fakes::Output;
use asus_ux8406_kde_live_test::guided::{keyboard_on_screen, lower_key};

use crate::support::services::{KeysService, LOWER, Scene, UPPER, panels};
use crate::support::{Fixture, detail, expect, rows, with_hands};

const PRESS: &str = "Press the key right of F12. The lower screen should go";

fn lower(y: i32) -> Output {
    Output {
        priority: 1,
        ..Output::panel(LOWER, 0, y)
    }
}

fn open(fixture: &Fixture) -> asus_ux8406_kde_common::output::Session {
    fixture.ctx().outputs().unwrap()
}

#[test]
fn three_presses_switch_the_lower_screen_off_on_and_off() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let keys = KeysService::new(scene.bus.address(), {
        let compositor = std::sync::Arc::clone(compositor);

        move || compositor.plug(lower(1440))
    });
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![
            ("go off", Box::new(|| compositor.unplug(LOWER))),
            ("go on", Box::new(|| compositor.plug(lower(1440)))),
            ("go off", Box::new(|| compositor.unplug(LOWER))),
        ],
        || {
            lower_key(&mut ctx, &mut session);
        },
    );
    // The first "go off" is also waited for by the second step: the steps
    // are told apart by their order, so the texts are only a guide.
    for number in 1..=3 {
        expect(
            &ctx,
            &format!("key right of F12, press {number} of 3"),
            "OK",
        );
    }
    assert_eq!(*keys.toggles.lock().unwrap(), 1);
    assert!(
        fixture
            .console
            .has_said("switched the lower screen on again: done")
    );
}

#[test]
fn a_screen_that_comes_back_elsewhere_fails() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![
            ("go off", Box::new(|| compositor.unplug(LOWER))),
            ("go on", Box::new(|| compositor.plug(lower(1500)))),
        ],
        || {
            lower_key(&mut ctx, &mut session);
        },
    );
    expect(&ctx, "key right of F12, press 2 of 3", "FAIL");
    assert!(detail(&ctx, "key right of F12, press 2 of 3").contains("not in the same place"));
}

#[test]
fn the_upper_screen_that_goes_off_too_fails() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![
            ("go off", Box::new(|| compositor.unplug(LOWER))),
            ("go on", Box::new(|| compositor.plug(lower(1440)))),
            (
                "go off",
                Box::new(|| {
                    compositor.unplug(LOWER);
                    compositor.unplug(UPPER);
                }),
            ),
        ],
        || {
            lower_key(&mut ctx, &mut session);
        },
    );
    expect(&ctx, "key right of F12, press 3 of 3", "FAIL");
    assert!(
        detail(&ctx, "key right of F12, press 3 of 3").contains("the upper screen went off too")
    );
}

#[test]
fn no_press_in_time_and_a_skip_stop_the_sequence() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    lower_key(&mut ctx, &mut session);
    expect(&ctx, "key right of F12, press 1 of 3", "FAIL");
    assert_eq!(rows(&ctx).len(), 1);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    fixture.console.skip_at("should go off");
    lower_key(&mut ctx, &mut session);
    expect(&ctx, "key right of F12, press 1 of 3", "SKIP");
}

#[test]
fn a_screen_that_stays_off_when_the_tool_cannot_switch_it_on_is_said() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![("go off", Box::new(|| compositor.unplug(LOWER)))],
        || {
            fixture.console.skip_at("go on");
            lower_key(&mut ctx, &mut session);
        },
    );
    expect(&ctx, "key right of F12, press 2 of 3", "SKIP");
    assert!(fixture.console.has_said("it did not work, press the key"));
}

#[test]
fn a_compositor_that_goes_away_is_reported() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![("go off", Box::new(|| scene.compositor.kick()))],
        || {
            lower_key(&mut ctx, &mut session);
        },
    );
    assert!(detail(&ctx, "key right of F12, press 1 of 3").contains("lost the connection"));
    let _ = PRESS;
}

#[test]
fn the_keyboard_on_the_screen_removes_it_and_off_the_screen_brings_it_back() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![
            (
                "Put the keyboard on the lower screen",
                Box::new(|| compositor.unplug(LOWER)),
            ),
            (
                "Take the keyboard off",
                Box::new(|| compositor.plug(lower(1440))),
            ),
        ],
        || {
            keyboard_on_screen(&mut ctx, &mut session);
        },
    );
    expect(
        &ctx,
        "keyboard on the lower screen: the screen goes away",
        "OK",
    );
    expect(
        &ctx,
        "the key does nothing with the keyboard on the screen",
        "OK",
    );
    expect(
        &ctx,
        "keyboard off the lower screen: screen and layout come back",
        "OK",
    );
}

#[test]
fn a_key_that_brings_the_screen_back_and_another_layout_fail() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![
            (
                "Put the keyboard on the lower screen",
                Box::new(|| compositor.unplug(LOWER)),
            ),
            (
                "Press the key right of F12 once",
                Box::new(|| compositor.plug(lower(1500))),
            ),
        ],
        || {
            keyboard_on_screen(&mut ctx, &mut session);
        },
    );
    expect(
        &ctx,
        "the key does nothing with the keyboard on the screen",
        "FAIL",
    );
    expect(
        &ctx,
        "keyboard off the lower screen: screen and layout come back",
        "FAIL",
    );
    assert!(
        detail(
            &ctx,
            "keyboard off the lower screen: screen and layout come back"
        )
        .contains("layout before")
    );
}

#[test]
fn a_connection_that_ends_while_the_tool_watches_is_reported() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![
            (
                "Put the keyboard on the lower screen",
                Box::new(|| compositor.unplug(LOWER)),
            ),
            (
                "Press the key right of F12 once",
                Box::new(|| compositor.kick()),
            ),
        ],
        || {
            keyboard_on_screen(&mut ctx, &mut session);
        },
    );
    assert!(
        detail(&ctx, "the key does nothing with the keyboard on the screen").contains("Wayland")
    );
}

#[test]
fn a_screen_that_stays_is_a_failure() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    keyboard_on_screen(&mut ctx, &mut session);
    expect(
        &ctx,
        "keyboard on the lower screen: the screen goes away",
        "FAIL",
    );
    assert_eq!(rows(&ctx).len(), 1);
}

#[test]
fn a_key_press_that_does_not_bring_the_screen_back_is_said() {
    let scene = Scene::new(panels());
    let compositor = &scene.compositor;
    let keys = KeysService::new(scene.bus.address(), || {});
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = open(&fixture);

    with_hands(
        &fixture.console,
        vec![("go off", Box::new(|| compositor.unplug(LOWER)))],
        || {
            fixture.console.skip_at("go on");
            lower_key(&mut ctx, &mut session);
        },
    );
    assert_eq!(*keys.toggles.lock().unwrap(), 1);
    assert!(fixture.console.has_said("it did not work, press the key"));
}

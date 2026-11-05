//! A screen turned by hand: the program holds it, and Auto-rotate shows off.

use std::sync::Arc;

use asus_ux8406_kde_fakes::{Compositor, Output};
use asus_ux8406_kde_live_test::hand;

use crate::support::services::{LOWER, RotationService, Scene, UPPER, panels};
use crate::support::{detail, expect, with_hands};

const TURNED: &str = "the lower screen turned by hand (TurnScreen)";
const BACK: &str = "switching on hands the screens back to the sensor";

fn lower_panel(transform: u32, y: i32) -> Output {
    Output {
        priority: 1,
        policy: 0,
        transform,
        ..Output::panel(LOWER, 0, y)
    }
}

/// The program as the check needs it: a turn sets the transform, and
/// switching on gives both screens the transform of the laptop.
fn program(scene: &Scene, transform: u32) -> RotationService {
    let service = RotationService::new(scene.bus.address());
    let compositor: Arc<Compositor> = Arc::clone(&scene.compositor);

    service.on_call(move |method, connector, orientation| {
        if method == "TurnScreen" {
            compositor.set_transform(connector, if orientation == "bottom-up" { 2 } else { 0 });
        } else {
            compositor.set_transform(UPPER, transform);
            compositor.set_transform(LOWER, transform);
        }
    });
    service
}

#[test]
fn a_turn_by_hand_holds_and_switching_on_hands_the_screens_back() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let service = program(&scene, 0);
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    service.set_enabled(false);
    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "OK");
    expect(&ctx, BACK, "OK");
    assert!(detail(&ctx, TURNED).contains("Enabled is false"));
    assert!(detail(&ctx, BACK).contains("both screens have transform 0"));
}

#[test]
fn a_sensor_without_an_orientation_is_no_fault() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("undefined");
    let service = program(&scene, 0);
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    service.set_enabled(false);
    hand::automatic(&mut ctx);
    expect(&ctx, BACK, "OK");
    assert!(detail(&ctx, BACK).contains("gives no orientation"));
}

#[test]
fn a_turn_that_does_nothing_and_a_program_that_is_not_there_fail() {
    let scene = Scene::new(panels());
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    service.set_enabled(false);
    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "FAIL");
    assert!(detail(&ctx, TURNED).contains("did not turn to transform 2"));

    let dark = Scene::new(panels());
    let fixture = dark.fixture();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "FAIL");
    expect(&ctx, BACK, "FAIL");
}

#[test]
fn a_turn_that_changes_the_upper_screen_or_leaves_a_gap_or_keeps_the_switch_on_fails() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let service = RotationService::new(scene.bus.address());
    let compositor = Arc::clone(&scene.compositor);

    service.on_call(move |_, _, _| {
        compositor.set_transform(UPPER, 1);
        compositor.set_transform(LOWER, 2);
    });
    service.set_enabled(false);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "FAIL");
    assert!(detail(&ctx, TURNED).contains("eDP-1 changed"));

    let gap = Scene::new(panels());
    let service = RotationService::new(gap.bus.address());
    let compositor = Arc::clone(&gap.compositor);

    service.on_call(move |_, _, _| {
        compositor.unplug(LOWER);
        compositor.plug(lower_panel(2, 5000));
    });
    service.set_enabled(false);

    let fixture = gap.fixture();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "FAIL");
    assert!(detail(&ctx, TURNED).contains("gap or overlap"));

    let on = Scene::new(panels());
    let _service = program(&on, 0);
    let fixture = on.fixture();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "FAIL");
    assert!(detail(&ctx, TURNED).contains("Enabled stayed true"));
}

#[test]
fn a_switch_that_does_nothing_or_screens_that_do_not_follow_fail() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let service = program(&scene, 0);
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    service.set_enabled(false);
    hand::automatic(&mut ctx);
    expect(&ctx, BACK, "FAIL");
    assert!(detail(&ctx, BACK).contains("asks for transform 1"));

    let stuck = Scene::new(panels());
    let service = program(&stuck, 0);
    let fixture = stuck.fixture();
    let mut ctx = fixture.ctx();

    service.set_enabled(false);
    service.ignore_set_enabled();
    hand::automatic(&mut ctx);
    expect(&ctx, BACK, "FAIL");
    assert!(detail(&ctx, BACK).contains("Enabled stayed false"));
}

#[test]
fn a_screen_that_is_off_skips_the_check() {
    let scene = Scene::new(panels());

    scene.compositor.unplug(LOWER);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "SKIP");
}

#[test]
fn an_upper_screen_that_is_off_skips_the_check_too() {
    let scene = Scene::new(panels());

    scene.compositor.unplug(UPPER);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "SKIP");
}

#[test]
fn without_a_compositor_the_check_is_a_failed_row() {
    let fixture = crate::support::Fixture::new();
    let mut ctx = fixture.ctx();

    hand::automatic(&mut ctx);
    expect(&ctx, TURNED, "FAIL");
}

type Step<'a> = (&'a str, Box<dyn FnOnce() + Send + 'a>);

fn steps<'a>(
    scene: &'a Scene,
    service: &'a RotationService,
    sensor_transform: u32,
) -> Vec<Step<'a>> {
    vec![
        (
            "turn the LOWER screen (the one next to the keyboard) upside down",
            Box::new(|| scene.compositor.set_transform(LOWER, 2)),
        ),
        (
            "Put the keyboard on the lower screen",
            Box::new(|| scene.compositor.unplug(LOWER)),
        ),
        (
            "Take the keyboard off the lower screen",
            Box::new(|| scene.compositor.plug(lower_panel(2, 1440))),
        ),
        (
            "Switch Auto-rotate on",
            Box::new(move || {
                service.set_enabled(true);
                scene.compositor.set_transform(LOWER, sensor_transform);
            }),
        ),
    ]
}

#[test]
fn the_applet_turns_the_lower_screen_and_the_keyboard_does_not_undo_it() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    service.set_enabled(false);
    with_hands(&fixture.console, steps(&scene, &service, 0), || {
        hand::guided(&mut ctx, &mut session);
    });
    expect(&ctx, "applet: turn the lower screen upside down", "OK");
    expect(
        &ctx,
        "applet: the keyboard goes on and off a held screen",
        "OK",
    );
    expect(
        &ctx,
        "applet: Auto-rotate on hands the screens back to the sensor",
        "OK",
    );
}

#[test]
fn a_lower_screen_that_does_not_turn_skips_the_rest_and_the_tool_switches_on() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    service.set_enabled(false);
    fixture.console.skip_at("upside down");
    hand::guided(&mut ctx, &mut session);
    expect(&ctx, "applet: turn the lower screen upside down", "SKIP");
    expect(
        &ctx,
        "applet: the keyboard goes on and off a held screen",
        "SKIP",
    );
    expect(
        &ctx,
        "applet: Auto-rotate on hands the screens back to the sensor",
        "SKIP",
    );
    assert_eq!(fixture.runner.calls().len(), 0);
}

#[test]
fn an_upper_screen_that_turned_too_is_a_failure_and_the_rest_is_skipped() {
    let scene = Scene::new(panels());
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    service.set_enabled(false);
    with_hands(
        &fixture.console,
        vec![(
            "upside down",
            Box::new(|| {
                scene.compositor.set_transform(UPPER, 1);
                scene.compositor.set_transform(LOWER, 2);
            }) as Box<dyn FnOnce() + Send>,
        )],
        || hand::guided(&mut ctx, &mut session),
    );
    expect(&ctx, "applet: turn the lower screen upside down", "FAIL");
    expect(
        &ctx,
        "applet: the keyboard goes on and off a held screen",
        "SKIP",
    );
}

#[test]
fn a_keyboard_that_does_not_go_on_the_screen_skips_that_check() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    service.set_enabled(false);
    fixture
        .console
        .skip_at("Put the keyboard on the lower screen");
    with_hands(
        &fixture.console,
        vec![
            steps(&scene, &service, 0).remove(0),
            steps(&scene, &service, 0).remove(3),
        ],
        || hand::guided(&mut ctx, &mut session),
    );
    expect(
        &ctx,
        "applet: the keyboard goes on and off a held screen",
        "SKIP",
    );
    expect(
        &ctx,
        "applet: Auto-rotate on hands the screens back to the sensor",
        "OK",
    );
}

#[test]
fn a_screen_that_comes_back_upright_after_the_keyboard_fails() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();
    let mut all = steps(&scene, &service, 0);

    all[2] = (
        "Take the keyboard off the lower screen",
        Box::new(|| scene.compositor.plug(lower_panel(0, 1440))),
    );
    service.set_enabled(false);
    fixture.console.skip_at("Switch Auto-rotate on");
    with_hands(&fixture.console, all.into_iter().take(3).collect(), || {
        hand::guided(&mut ctx, &mut session);
    });
    expect(
        &ctx,
        "applet: the keyboard goes on and off a held screen",
        "FAIL",
    );
    expect(
        &ctx,
        "applet: Auto-rotate on hands the screens back to the sensor",
        "SKIP",
    );
}

#[test]
fn a_layout_with_a_gap_after_the_turn_is_a_failure() {
    let scene = Scene::new(panels());
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    service.set_enabled(false);
    with_hands(
        &fixture.console,
        vec![(
            "upside down",
            Box::new(|| {
                scene.compositor.unplug(LOWER);
                scene.compositor.plug(lower_panel(2, 5000));
            }) as Box<dyn FnOnce() + Send>,
        )],
        || hand::guided(&mut ctx, &mut session),
    );
    expect(&ctx, "applet: turn the lower screen upside down", "FAIL");
    assert!(detail(&ctx, "applet: turn the lower screen upside down").contains("not valid"));
}

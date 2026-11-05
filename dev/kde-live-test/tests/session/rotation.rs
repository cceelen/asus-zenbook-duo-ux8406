//! The four turns of the laptop, with both panels and with one.

use asus_ux8406_kde_common::geometry::Monitor;
use asus_ux8406_kde_common::output::Session;
use asus_ux8406_kde_live_test::guided::rotation;

use crate::support::services::{LOWER, RotationService, Scene, UPPER, panels};
use crate::support::{detail, expect, with_hands};

fn monitor(name: &str, x: i32, transform: u32, primary: bool) -> Monitor {
    Monitor {
        x,
        y: if transform == 0 && name == LOWER {
            1440
        } else {
            0
        },
        scale: 1.25,
        transform,
        primary,
        connectors: vec![name.to_owned()],
    }
}

/// The layouts that the program of the rotation would give.
fn layout(transform: u32, gap: i32) -> Vec<Monitor> {
    match transform {
        0 => vec![monitor(UPPER, 0, 0, false), monitor(LOWER, 0, 0, true)],
        1 => vec![
            monitor(LOWER, 0, 1, true),
            monitor(UPPER, 1440 + gap, 1, false),
        ],
        2 => vec![
            Monitor {
                y: 1440,
                ..monitor(UPPER, 0, 2, false)
            },
            monitor(LOWER, 0, 2, true),
        ],
        _ => vec![
            monitor(UPPER, 0, 3, false),
            monitor(LOWER, 1440 + gap, 3, true),
        ],
    }
}

fn turn(
    scene: &Scene,
    sensor: &asus_ux8406_kde_fakes::Sensor,
    orientation: &str,
    layout: Option<Vec<Monitor>>,
) {
    if let Some(layout) = layout {
        let connection = wayland_client::Connection::from_socket(
            std::os::unix::net::UnixStream::connect(scene.compositor.socket()).unwrap(),
        )
        .unwrap();
        let mut session = Session::with(connection).unwrap();
        let number = session.apply(&layout);

        session.flush().unwrap();
        session.wait(number).unwrap().unwrap();
    }
    sensor.turn(orientation);
}

type Steps<'a> = Vec<(&'a str, Box<dyn FnOnce() + Send + 'a>)>;

/// The layout with the upper panel only.
fn upper_only(transform: u32) -> Vec<Monitor> {
    vec![monitor(UPPER, 0, transform, true)]
}

/// The four turns with both panels, or with the upper panel only.
fn turns<'a>(
    scene: &'a Scene,
    sensor: &'a asus_ux8406_kde_fakes::Sensor,
    gap: i32,
    both: bool,
) -> Steps<'a> {
    let layout = move |transform: u32| {
        if both {
            layout(transform, if transform == 0 { 0 } else { gap })
        } else {
            upper_only(transform)
        }
    };

    vec![
        (
            "LEFT side is up",
            Box::new(move || turn(scene, sensor, "left-up", Some(layout(1)))),
        ),
        (
            "RIGHT side is up",
            Box::new(move || turn(scene, sensor, "right-up", Some(layout(3)))),
        ),
        (
            "upright again",
            Box::new(move || turn(scene, sensor, "normal", Some(layout(0)))),
        ),
        (
            "upside down",
            Box::new(move || turn(scene, sensor, "bottom-up", Some(layout(2)))),
        ),
    ]
}

/// The lower screen goes off, and the laptop stands upright.
fn switch_lower_off<'a>(scene: &'a Scene, sensor: &'a asus_ux8406_kde_fakes::Sensor) -> Steps<'a> {
    vec![
        (
            "Hold the laptop upright for the run",
            Box::new(move || turn(scene, sensor, "normal", Some(layout(0, 0)))),
        ),
        (
            "The lower screen should go off",
            Box::new(move || scene.compositor.unplug(LOWER)),
        ),
    ]
}

fn both_runs<'a>(
    scene: &'a Scene,
    sensor: &'a asus_ux8406_kde_fakes::Sensor,
    gap: i32,
) -> Steps<'a> {
    let mut steps = turns(scene, sensor, gap, true);

    steps.extend(switch_lower_off(scene, sensor));
    steps.extend(turns(scene, sensor, gap, false));
    steps
}

const NAMES: [&str; 4] = ["left-up", "right-up", "normal", "bottom-up"];

#[test]
fn the_panels_turn_with_the_laptop_and_come_back_with_both_panels_and_with_one() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    with_hands(&fixture.console, both_runs(&scene, &sensor, 0), || {
        rotation(&mut ctx, &mut session);
    });
    for label in ["both panels", "upper panel only"] {
        for name in NAMES {
            expect(&ctx, &format!("rotation, {label}: {name}"), "OK");
        }
    }
    assert!(detail(&ctx, "rotation, both panels: left-up").contains("eDP-1 and eDP-2 turned"));
    assert!(detail(&ctx, "rotation, upper panel only: left-up").contains("eDP-1 turned"));
    assert!(
        fixture
            .console
            .has_said("Put the keyboard on the cable and the laptop upright")
    );
}

#[test]
fn a_lower_screen_that_is_off_at_the_start_is_switched_on_first() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");

    scene.compositor.unplug(LOWER);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();
    let mut steps: Steps<'_> = vec![(
        "The lower screen should go on",
        Box::new(|| {
            scene.compositor.plug(asus_ux8406_kde_fakes::Output {
                priority: 1,
                ..asus_ux8406_kde_fakes::Output::panel(LOWER, 0, 1440)
            });
        }),
    )];

    steps.extend(turns(&scene, &sensor, 0, true));
    steps.extend(switch_lower_off(&scene, &sensor));
    steps.extend(turns(&scene, &sensor, 0, false));
    with_hands(&fixture.console, steps, || rotation(&mut ctx, &mut session));
    expect(&ctx, "rotation, both panels: left-up", "OK");
    expect(&ctx, "rotation, upper panel only: bottom-up", "OK");
}

#[test]
fn a_lower_screen_that_does_not_go_on_skips_the_run_with_both_panels() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");

    scene.compositor.unplug(LOWER);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    fixture.console.skip_at("should go on");
    with_hands(&fixture.console, turns(&scene, &sensor, 0, false), || {
        rotation(&mut ctx, &mut session);
    });
    expect(&ctx, "rotation, both panels", "SKIP");
    expect(&ctx, "rotation, upper panel only: left-up", "OK");
}

#[test]
fn a_gap_between_the_turned_panels_fails() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    fixture
        .console
        .skip_at("Hold the laptop upright for the run");
    with_hands(&fixture.console, turns(&scene, &sensor, 200, true), || {
        rotation(&mut ctx, &mut session);
    });
    expect(&ctx, "rotation, both panels: left-up", "FAIL");
    assert!(detail(&ctx, "rotation, both panels: left-up").contains("gap or overlap"));
}

#[test]
fn a_layout_that_does_not_come_back_fails() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();
    let mut steps = turns(&scene, &sensor, 0, true);
    let moved = {
        let mut other = layout(0, 0);

        other[0].y = 1440;
        other[1].y = 0;
        other
    };

    steps[2] = (
        "upright again",
        Box::new(|| turn(&scene, &sensor, "normal", Some(moved))),
    );
    fixture
        .console
        .skip_at("Hold the laptop upright for the run");
    with_hands(&fixture.console, steps, || rotation(&mut ctx, &mut session));
    expect(&ctx, "rotation, both panels: normal", "FAIL");
    assert!(detail(&ctx, "rotation, both panels: normal").contains("not the layout from before"));
}

#[test]
fn panels_that_do_not_turn_fail_and_a_sensor_that_is_silent_fails() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();
    let steps: Steps<'_> = vec![(
        "LEFT side is up",
        Box::new(|| turn(&scene, &sensor, "left-up", None)),
    )];

    fixture
        .console
        .skip_at("Hold the laptop upright for the run");
    with_hands(&fixture.console, steps, || rotation(&mut ctx, &mut session));
    expect(&ctx, "rotation, both panels: left-up", "FAIL");
    assert!(detail(&ctx, "rotation, both panels: left-up").contains("did not turn to left-up"));
    expect(&ctx, "rotation, both panels: right-up", "FAIL");
    assert!(
        detail(&ctx, "rotation, both panels: right-up").contains("did not report this orientation")
    );
}

#[test]
fn an_upside_down_turn_that_does_not_turn_the_panels_fails() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();
    let steps: Steps<'_> = vec![(
        "upside down",
        Box::new(|| turn(&scene, &sensor, "bottom-up", None)),
    )];

    fixture.console.skip_at("LEFT side");
    fixture
        .console
        .skip_at("Hold the laptop upright for the run");
    with_hands(&fixture.console, steps, || rotation(&mut ctx, &mut session));
    expect(&ctx, "rotation, both panels: bottom-up", "FAIL");
    assert!(detail(&ctx, "rotation, both panels: bottom-up").contains("did not turn to bottom-up"));
}

#[test]
fn an_upside_down_layout_with_the_lower_panel_below_fails() {
    let scene = Scene::new(panels());
    let sensor = scene.sensor("normal");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();
    let wrong = {
        let mut other = layout(2, 0);

        other[0].y = 0;
        other[1].y = 1440;
        other
    };
    let steps: Steps<'_> = vec![(
        "upside down",
        Box::new(|| turn(&scene, &sensor, "bottom-up", Some(wrong))),
    )];

    fixture.console.skip_at("LEFT side");
    fixture
        .console
        .skip_at("Hold the laptop upright for the run");
    with_hands(&fixture.console, steps, || rotation(&mut ctx, &mut session));
    expect(&ctx, "rotation, both panels: bottom-up", "FAIL");
    assert!(
        detail(&ctx, "rotation, both panels: bottom-up").contains("the lower panel is not on top")
    );
}

#[test]
fn a_laptop_that_is_not_upright_at_the_start_is_a_skip() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation, both panels", "SKIP");
}

#[test]
fn no_system_bus_fails() {
    let scene = Scene::new(panels());
    let mut fixture = scene.fixture();
    let mut session = fixture.ctx().outputs().unwrap();

    fixture.vars.insert(
        "DBUS_SYSTEM_BUS_ADDRESS".to_owned(),
        "unix:path=/nonexistent/bus".to_owned(),
    );

    let mut ctx = fixture.ctx();

    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation", "FAIL");
}

fn skipped_rotation(scene: &Scene) -> (bool, Vec<String>) {
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation, both panels", "SKIP");
    (
        crate::support::rows(&ctx)
            .iter()
            .any(|row| row.0 == "rotation was off"),
        crate::support::rows(&ctx)
            .into_iter()
            .map(|row| row.2)
            .collect(),
    )
}

#[test]
fn a_rotation_that_was_off_is_switched_on_and_the_tester_is_told() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    service.set_enabled(false);
    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation was off", "OK");
    assert!(
        detail(&ctx, "rotation was off").contains("switched on for the test"),
        "{}",
        detail(&ctx, "rotation was off")
    );
}

#[test]
fn a_rotation_that_is_on_is_left_alone() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let _service = RotationService::new(scene.bus.address());
    let (said, results) = skipped_rotation(&scene);

    assert!(!said);
    assert_eq!(results, ["SKIP"]);
}

#[test]
fn a_program_that_is_not_there_is_left_alone() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let (said, _) = skipped_rotation(&scene);

    assert!(!said);
}

#[test]
fn a_program_that_does_not_switch_on_or_refuses_fails_the_row() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let service = RotationService::new(scene.bus.address());

    service.set_enabled(false);
    service.ignore_set_enabled();

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let mut session = ctx.outputs().unwrap();

    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation was off", "FAIL");
    assert!(detail(&ctx, "rotation was off").contains("stayed false"));

    service.fail_set_enabled();

    let mut ctx = fixture.ctx();

    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation was off", "FAIL");
    assert!(detail(&ctx, "rotation was off").contains("could not switch the rotation on"));
}

#[test]
fn without_a_session_bus_the_rotation_is_not_looked_at() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("left-up");
    let mut fixture = scene.fixture();

    fixture.vars.insert(
        "DBUS_SESSION_BUS_ADDRESS".to_owned(),
        "unix:path=/nonexistent/bus".to_owned(),
    );

    let mut ctx = fixture.ctx();
    let mut session = scene.fixture().ctx().outputs().unwrap();

    rotation(&mut ctx, &mut session);
    expect(&ctx, "rotation, both panels", "SKIP");
}

//! The touchscreens and pens: their mapping in KWin, and a touch on each
//! panel.

use std::io::Write;

use asus_ux8406_kde_fakes::input::Device;
use asus_ux8406_kde_fakes::{Bus, Input};
use asus_ux8406_kde_live_test::auto;
use asus_ux8406_kde_live_test::guided;

use crate::support::services::{Scene, panels};
use crate::support::{Fixture, detail, expect, rows, with_hands};

const MAPPING: &str = "ELAN touch and pen devices act on their panels";
const LOG: &str = "results/raw/libinput-debug-events.log";
const START: &str = "sudo -n stdbuf -oL libinput debug-events --device /dev/input/event7";

#[test]
fn each_controller_acts_on_its_own_panel() {
    let scene = Scene::new(panels());

    scene.map_touch();

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    auto::touch(&mut ctx);
    expect(&ctx, MAPPING, "OK");
    assert!(detail(&ctx, MAPPING).contains("event9 (04f3:4448) on \"eDP-2\""));
}

#[test]
fn the_mapping_that_comes_a_moment_later_is_waited_for() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();
    let bus = scene.bus.address().to_owned();
    let helper = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));

        let connection = zbus::blocking::connection::Builder::address(bus.as_str())
            .and_then(zbus::blocking::connection::Builder::build)
            .unwrap();

        for (event, panel) in [
            ("event7", "eDP-1"),
            ("event8", "eDP-1"),
            ("event9", "eDP-2"),
        ] {
            connection
                .call_method(
                    Some("org.kde.KWin"),
                    format!("/org/kde/KWin/InputDevice/{event}"),
                    Some("org.freedesktop.DBus.Properties"),
                    "Set",
                    &(
                        "org.kde.KWin.InputDevice",
                        "outputName",
                        zbus::zvariant::Value::from(panel),
                    ),
                )
                .unwrap();
        }
    });

    auto::touch(&mut ctx);
    helper.join().unwrap();
    expect(&ctx, MAPPING, "OK");
}

#[test]
fn devices_without_a_panel_or_on_the_wrong_one_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    auto::touch(&mut ctx);
    expect(&ctx, MAPPING, "FAIL");
    assert!(detail(&ctx, MAPPING).contains("do not act on eDP-1 and eDP-2"));
}

#[test]
fn both_controllers_on_the_upper_panel_fail() {
    let scene = Scene::new(panels());

    scene.map_touch_to(["eDP-1", "eDP-1", "eDP-1"]);

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    auto::touch(&mut ctx);
    expect(&ctx, MAPPING, "FAIL");
    assert!(detail(&ctx, MAPPING).contains("do not act on eDP-1 and eDP-2"));
}

#[test]
fn controllers_that_share_a_panel_or_split_fail() {
    let scene = Scene::new(panels());

    scene.map_touch();
    // The pen of the upper controller acts on the lower panel.
    scene
        .bus
        .connect()
        .call_method(
            Some("org.kde.KWin"),
            "/org/kde/KWin/InputDevice/event8",
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(
                "org.kde.KWin.InputDevice",
                "outputName",
                zbus::zvariant::Value::from("eDP-2"),
            ),
        )
        .unwrap();

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    auto::touch(&mut ctx);
    expect(&ctx, MAPPING, "FAIL");
    assert!(detail(&ctx, MAPPING).contains("one controller act on different panels"));
}

#[test]
fn no_elan_device_and_no_kwin_fail() {
    let bus = Bus::new();
    let _input = Input::new(
        bus.address(),
        vec![Device {
            sys_name: "event1".to_owned(),
            vendor: 1,
            touch: true,
            ..Device::default()
        }],
    );
    let fixture = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());
    let mut ctx = fixture.ctx();

    auto::touch(&mut ctx);
    assert!(detail(&ctx, MAPPING).contains("no ELAN touchscreen"));

    let none = Fixture::new()
        .var("DBUS_SESSION_BUS_ADDRESS", bus.address())
        .var("XDG_RUNTIME_DIR", "/nonexistent");
    let dark = Bus::new();
    let mut ctx = none.ctx();

    ctx.vars.insert(
        "DBUS_SESSION_BUS_ADDRESS".to_owned(),
        dark.address().to_owned(),
    );
    auto::touch(&mut ctx);
    assert!(detail(&ctx, MAPPING).contains("KWin's input devices"));
}

fn append(path: &std::path::Path, text: &str) -> impl FnOnce() + Send {
    let path = path.to_owned();
    let text = text.to_owned();

    move || {
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }
}

fn libinput(scene: &Scene) -> Fixture {
    scene.map_touch();

    let fixture = scene.fixture();

    // The first lines of libinput: the devices.
    fixture.runner.log(START, "-event7   DEVICE_ADDED\n");
    fixture
}

#[test]
fn a_touch_and_a_pen_on_each_panel_come_from_the_controller_of_the_panel() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);
    let mut ctx = fixture.ctx();
    let log = fixture.dir.path().join(LOG);

    with_hands(
        &fixture.console,
        vec![
            (
                "LOWER screen (the one next to the keyboard) with a finger",
                Box::new(append(&log, "event9   TOUCH_DOWN  +1.0s\n")),
            ),
            (
                "LOWER screen (the one next to the keyboard) with the pen",
                Box::new(append(&log, "event9   TABLET_TOOL_PROXIMITY\n")),
            ),
            (
                "UPPER screen (the one with the camera) with a finger",
                Box::new(append(&log, "event7   TOUCH_DOWN\n")),
            ),
            (
                "UPPER screen (the one with the camera) with the pen",
                Box::new(append(&log, "event8   TABLET_TOOL_AXIS\n")),
            ),
        ],
        || {
            guided::touch(&mut ctx);
        },
    );
    for check in [
        "finger on the lower screen",
        "pen on the lower screen",
        "finger on the upper screen",
        "pen on the upper screen",
    ] {
        expect(&ctx, check, "OK");
    }
    assert!(
        detail(&ctx, "finger on the lower screen")
            .contains("TOUCH_DOWN from event9, which KWin maps to eDP-2")
    );
    assert!(
        fixture
            .runner
            .called("sudo -n pkill -f libinput debug-events")
    );
    assert!(
        fixture.runner.starts()[0]
            .contains("--device /dev/input/event8 --device /dev/input/event9")
    );
}

#[test]
fn an_event_from_the_wrong_controller_or_an_unknown_one_fails() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);
    let mut ctx = fixture.ctx();
    let log = fixture.dir.path().join(LOG);

    with_hands(
        &fixture.console,
        vec![
            (
                "LOWER screen (the one next to the keyboard) with a finger",
                Box::new(append(&log, "event7   TOUCH_DOWN\n")),
            ),
            (
                "That was the upper screen. Touch the LOWER screen",
                Box::new(append(&log, "event8   TOUCH_DOWN\n")),
            ),
            (
                "LOWER screen (the one next to the keyboard) with the pen",
                Box::new(append(&log, "event99   TABLET_TOOL_TIP\n")),
            ),
            (
                "UPPER screen (the one with the camera) with a finger",
                Box::new(append(&log, "event7   TOUCH_UP\nevent7   TOUCH_DOWN\n")),
            ),
            (
                "UPPER screen (the one with the camera) with the pen",
                Box::new(append(&log, "event8   TABLET_TOOL_AXIS\n")),
            ),
        ],
        || {
            guided::touch(&mut ctx);
        },
    );
    expect(&ctx, "finger on the lower screen", "FAIL");
    assert!(detail(&ctx, "finger on the lower screen").contains("maps to \"eDP-1\", not eDP-2"));
    assert!(
        detail(&ctx, "finger on the lower screen")
            .contains("TOUCH_DOWN from event7; then TOUCH_DOWN from event8")
    );
    assert!(detail(&ctx, "pen on the lower screen").contains("event99, which KWin does not list"));
    expect(&ctx, "finger on the upper screen", "OK");
    expect(&ctx, "pen on the upper screen", "OK");
}

#[test]
fn no_event_in_time_and_a_skip_are_told_apart() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);
    let mut ctx = fixture.ctx();

    fixture.runner.log(START, "");
    fixture
        .console
        .skip_at("LOWER screen (the one next to the keyboard) with the pen");
    guided::touch(&mut ctx);
    expect(&ctx, "finger on the lower screen", "FAIL");
    assert!(
        detail(&ctx, "finger on the lower screen").contains("no event from the ELAN controllers")
    );
    expect(&ctx, "pen on the lower screen", "SKIP");
}

#[test]
fn a_log_that_stays_empty_does_not_block_the_start() {
    let scene = Scene::new(panels());

    scene.map_touch();

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    fixture.console.skip_at("Touch");
    guided::touch(&mut ctx);
    assert_eq!(rows(&ctx).len(), 4);
}

#[test]
fn without_a_pen_only_a_finger_is_asked_for() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);
    let mut ctx = fixture.ctx();
    let log = fixture.dir.path().join(LOG);

    ctx.pen = false;
    with_hands(
        &fixture.console,
        vec![
            (
                "LOWER screen (the one next to the keyboard) with a finger",
                Box::new(append(&log, "event9   TOUCH_DOWN\n")),
            ),
            (
                "UPPER screen (the one with the camera) with a finger",
                Box::new(append(&log, "event7   TOUCH_DOWN\n")),
            ),
        ],
        || {
            guided::touch(&mut ctx);
        },
    );
    expect(&ctx, "finger on the lower screen", "OK");
    expect(&ctx, "finger on the upper screen", "OK");
    expect(&ctx, "pen on each screen", "SKIP");
    assert!(detail(&ctx, "pen on each screen").contains("--pen"));
    assert!(!fixture.console.has_said("with the pen"));
    assert_eq!(rows(&ctx).len(), 3);
}

#[test]
fn no_device_no_kwin_and_no_libinput_are_reported() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);

    fixture.runner.refuse("sudo");

    let mut ctx = fixture.ctx();

    guided::touch(&mut ctx);
    assert!(detail(&ctx, "touch and pen").contains("cannot start"));

    let bus = Bus::new();
    let _input = Input::new(bus.address(), vec![]);
    let empty = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());
    let mut ctx = empty.ctx();

    guided::touch(&mut ctx);
    expect(&ctx, "touch and pen", "SKIP");

    let dark = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    let mut ctx = dark.ctx();

    guided::touch(&mut ctx);
    expect(&ctx, "touch and pen", "FAIL");
}

#[test]
fn a_touch_on_the_other_screen_is_asked_for_again_once() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);
    let mut ctx = fixture.ctx();
    let log = fixture.dir.path().join(LOG);

    with_hands(
        &fixture.console,
        vec![
            (
                "LOWER screen (the one next to the keyboard) with a finger",
                Box::new(append(&log, "event7   TOUCH_DOWN\n")),
            ),
            (
                "That was the upper screen. Touch the LOWER screen, the one next to the keyboard.",
                Box::new(append(&log, "event9   TOUCH_DOWN\n")),
            ),
            (
                "UPPER screen (the one with the camera) with a finger",
                Box::new(append(&log, "event9   TOUCH_DOWN\n")),
            ),
            (
                "That was the lower screen. Touch the UPPER screen, the one with the camera.",
                Box::new(append(&log, "event7   TOUCH_DOWN\n")),
            ),
        ],
        || {
            guided::touch(&mut ctx);
        },
    );
    expect(&ctx, "finger on the lower screen", "OK");
    expect(&ctx, "finger on the upper screen", "OK");
    assert!(
        detail(&ctx, "finger on the lower screen")
            .contains("the first touch was on the other screen")
    );
}

#[test]
fn a_second_touch_that_does_not_come_is_a_missing_event() {
    let scene = Scene::new(panels());
    let fixture = libinput(&scene);
    let mut ctx = fixture.ctx();
    let log = fixture.dir.path().join(LOG);

    fixture.console.skip_at("That was the upper screen");
    with_hands(
        &fixture.console,
        vec![(
            "LOWER screen (the one next to the keyboard) with a finger",
            Box::new(append(&log, "event7   TOUCH_DOWN\n")),
        )],
        || {
            guided::touch(&mut ctx);
        },
    );
    expect(&ctx, "finger on the lower screen", "SKIP");
}

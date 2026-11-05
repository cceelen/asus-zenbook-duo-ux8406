//! Services that fail in ways the fakes do not: a compositor that hangs up,
//! KWin without its input devices, scripts that KWin refuses later.

use std::os::unix::net::UnixListener;

use asus_ux8406_kde_fakes::Bus;
use asus_ux8406_kde_live_test::deps::runtime;
use asus_ux8406_kde_live_test::guided;
use asus_ux8406_kde_live_test::windows::Windows;

use crate::support::services::{Scene, panels, scripting_only};
use crate::support::{Fixture, detail, expect, rows};

/// A socket where something accepts and hangs up.
fn hangup() -> (tempfile::TempDir, std::thread::JoinHandle<()>) {
    let dir = tempfile::tempdir().unwrap();
    let listener = UnixListener::bind(dir.path().join("wayland-dead")).unwrap();
    let thread = std::thread::spawn(move || {
        for _ in 0..4 {
            let _ = listener.accept();
        }
    });

    (dir, thread)
}

#[test]
fn a_compositor_that_hangs_up_is_reported_by_the_registry_and_by_the_outputs() {
    let (dir, _thread) = hangup();
    let fixture = Fixture::new().var(
        "WAYLAND_DISPLAY",
        &dir.path().join("wayland-dead").display().to_string(),
    );
    let mut ctx = fixture.ctx();

    assert!(ctx.outputs().is_err());
    runtime(&mut ctx);
    assert!(detail(&ctx, "KWin Wayland output protocols").contains("Wayland registry"));
}

#[test]
fn kwin_without_input_devices_is_named() {
    let bus = Bus::new();
    let _kwin = scripting_only(bus.address());
    let fixture = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    expect(&ctx, "KWin scripting and input device interfaces", "FAIL");
    assert!(
        detail(&ctx, "KWin scripting and input device interfaces")
            .contains("input device interface")
    );
}

#[test]
fn a_bus_without_kwin_fails_the_scripting() {
    let bus = Bus::new();
    let fixture = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    assert!(
        detail(&ctx, "KWin scripting and input device interfaces").contains("scripting interface")
    );
}

#[test]
fn scripts_that_kwin_refuses_after_the_start_are_errors() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let windows = Windows::start(&fixture.ctx()).unwrap();

    scene.scripts.lock().unwrap().refuse = true;
    assert!(
        windows
            .place(1, "eDP-1")
            .unwrap_err()
            .contains("did not load the script")
    );
}

#[test]
fn a_bus_without_kwin_cannot_load_the_watch_script() {
    let bus = Bus::new();
    let fixture = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());

    assert!(
        Windows::start(&fixture.ctx())
            .err()
            .unwrap()
            .contains("loadScript")
    );
}

#[test]
fn reports_that_are_not_a_process_and_an_output_are_ignored() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let windows = Windows::start(&fixture.ctx()).unwrap();
    let bus = scene.bus.connect();
    let report = |line: &str| {
        bus.call_method(
            Some(asus_ux8406_kde_live_test::windows::SERVICE),
            asus_ux8406_kde_live_test::windows::PATH,
            Some(asus_ux8406_kde_live_test::windows::SERVICE),
            "Report",
            &(line,),
        )
        .unwrap();
    };

    report("nonsense");
    report("x eDP-1");
    report("42 eDP-2");
    assert_eq!(windows.output_of(42).as_deref(), Some("eDP-2"));
    assert_eq!(windows.output_of(0), None);
}

#[test]
fn without_a_compositor_the_guided_checks_say_so_and_the_keys_are_still_asked() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    fixture.runner.rule("pgrep", 1, "");
    fixture.console.skip_at(">>>");
    guided::run(&mut ctx);
    expect(&ctx, "guided checks", "FAIL");
    expect(&ctx, "F11 opens the emoji selector", "SKIP");
    expect(&ctx, "F12 opens System Settings", "SKIP");
    expect(&ctx, "terminal window above the others", "OK");
    assert_eq!(rows(&ctx).len(), 4);
}

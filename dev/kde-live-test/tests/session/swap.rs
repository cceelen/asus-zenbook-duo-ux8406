//! F8, with two test windows, and the service the scripts report to.

use std::sync::atomic::Ordering;

use asus_ux8406_kde_live_test::guided::swap;
use asus_ux8406_kde_live_test::windows::{WATCH, Windows};

use crate::support::services::{FakeWindows, Scene, panels};
use crate::support::{Fixture, detail, expect, with_hands};

const CHECK: &str = "F8 swaps the windows of the two screens";

#[test]
fn f8_swaps_two_windows_that_the_tool_placed() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let windows = FakeWindows::install(&scene, &fixture.runner);
    let mut ctx = fixture.ctx();

    with_hands(
        &fixture.console,
        vec![("Press F8", Box::new(|| windows.swap()))],
        || {
            swap(&mut ctx);
        },
    );
    expect(&ctx, CHECK, "OK");
    expect(&ctx, "the test window of process 1000 closed", "OK");
    assert!(detail(&ctx, "the test window of process 1001 closed").contains("close it by hand"));
    assert_eq!(
        fixture.runner.starts(),
        [
            "konsole --nofork -p tabtitle=UX8406-TEST-A -e sleep 3600",
            "konsole --nofork -p tabtitle=UX8406-TEST-B -e sleep 3600"
        ]
    );
    assert!(fixture.runner.called("kill 1000"));
    assert!(fixture.runner.called("kill 1001"));
    assert!(
        scene
            .scripts
            .lock()
            .unwrap()
            .loaded
            .iter()
            .all(|(name, _)| name != "ux8406-live-test-watch")
    );
}

#[test]
fn windows_that_do_not_swap_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let _windows = FakeWindows::install(&scene, &fixture.runner);
    let mut ctx = fixture.ctx();

    swap(&mut ctx);
    expect(&ctx, CHECK, "FAIL");
    assert!(detail(&ctx, CHECK).contains("did not swap"));
}

#[test]
fn a_skip_at_the_prompt_is_a_skip() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let _windows = FakeWindows::install(&scene, &fixture.runner);
    let mut ctx = fixture.ctx();

    fixture.console.skip_at("Press F8");
    swap(&mut ctx);
    expect(&ctx, CHECK, "SKIP");
}

#[test]
fn windows_that_kwin_does_not_report_or_does_not_move_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    swap(&mut ctx);
    assert!(detail(&ctx, CHECK).contains("did not report the two test windows"));

    let windows = FakeWindows::install(&scene, &fixture.runner);
    let mut ctx = fixture.ctx();

    windows.obey.store(false, Ordering::SeqCst);
    // Both windows start on the upper screen: B never reaches the lower one.
    swap(&mut ctx);
    assert!(detail(&ctx, CHECK).contains("did not go to their screens"));

    // Both windows start on the lower screen: A never reaches the upper one.
    let fixture = scene.fixture();
    let windows = FakeWindows::install(&scene, &fixture.runner);
    let mut ctx = fixture.ctx();

    windows.obey.store(false, Ordering::SeqCst);
    *windows.opens_on.lock().unwrap() = "eDP-2".to_owned();
    swap(&mut ctx);
    assert!(detail(&ctx, CHECK).contains("did not go to their screens"));
}

#[test]
fn a_terminal_that_does_not_start_or_a_script_that_kwin_refuses_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    fixture
        .runner
        .refuse("konsole --nofork -p tabtitle=UX8406-TEST-B");
    swap(&mut ctx);
    assert!(detail(&ctx, CHECK).contains("konsole: cannot start"));
    assert_eq!(fixture.runner.starts().len(), 1);
    assert!(fixture.runner.called("kill 1000"));

    scene.scripts.lock().unwrap().refuse = true;

    let mut ctx = fixture.ctx();

    swap(&mut ctx);
    assert!(detail(&ctx, CHECK).contains("did not load the script"));
}

#[test]
fn the_service_exists_once_and_needs_a_bus_and_a_writable_directory() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let ctx = fixture.ctx();
    let first = Windows::start(&ctx).unwrap();

    assert_eq!(first.output_of(1), None);
    assert!(
        Windows::start(&ctx)
            .err()
            .unwrap()
            .contains("service io.github.cceelen.Ux8406LiveTest")
    );
    assert!(WATCH.contains("windowAdded"));

    let dark = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");

    assert!(Windows::start(&dark.ctx()).is_err());

    // A results directory that is gone: the script cannot be written.
    let other = Scene::new(panels());
    let fixture = other.fixture();
    let ctx = fixture.ctx();

    std::fs::remove_dir_all(ctx.report.dir().join("raw")).unwrap();
    assert!(
        Windows::start(&ctx)
            .err()
            .unwrap()
            .contains("ux8406-live-test-watch.js")
    );
}

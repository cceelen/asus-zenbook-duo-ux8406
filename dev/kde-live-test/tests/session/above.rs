//! The terminal window of the tester stays above the other windows.

use std::sync::{Arc, Mutex};

use asus_ux8406_kde_live_test::windows::{above_script, ancestors, keep_above};

use crate::support::services::{Scene, panels};
use crate::support::{Fixture, detail, expect};

/// A session without a bus: the real one of the machine is not touched.
fn dark() -> Fixture {
    Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus")
}

fn me() -> u32 {
    std::process::id()
}

fn parent(fixture: &Fixture, pid: u32, parent: u32) {
    fixture.file(
        &format!("/proc/{pid}/stat"),
        &format!("{pid} (a name with ) and spaces) S {parent} 1 1 0\n"),
    );
}

#[test]
fn the_script_names_the_processes_and_the_state() {
    let script = above_script(&[7, 12], true);

    assert!(script.contains("const pids = [7, 12];"));
    assert!(script.contains("w.keepAbove = true;"));
    assert!(above_script(&[], false).contains("w.keepAbove = false;"));
}

#[test]
fn the_ancestors_are_the_tool_and_its_parents_up_to_init() {
    let fixture = Fixture::new();

    parent(&fixture, me(), 500);
    parent(&fixture, 500, 400);
    parent(&fixture, 400, 1);
    assert_eq!(ancestors(&fixture.ctx()), [me(), 500, 400]);
}

#[test]
fn the_ancestors_stop_where_the_process_table_does() {
    let fixture = Fixture::new();

    assert_eq!(ancestors(&fixture.ctx()), [me()]);
    fixture.file(&format!("/proc/{}/stat", me()), "no brackets\n");
    assert_eq!(ancestors(&fixture.ctx()), [me()]);
    fixture.file(&format!("/proc/{}/stat", me()), "1 (x) S\n");
    assert_eq!(ancestors(&fixture.ctx()), [me()]);
    fixture.file(&format!("/proc/{}/stat", me()), "1 (x) S nonsense\n");
    assert_eq!(ancestors(&fixture.ctx()), [me()]);
}

#[test]
fn a_parent_that_comes_again_ends_the_walk() {
    let fixture = Fixture::new();

    parent(&fixture, me(), 500);
    parent(&fixture, 500, me());
    assert_eq!(ancestors(&fixture.ctx()), [me(), 500]);
}

#[test]
fn a_script_runs_once_for_each_state_and_is_unloaded() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let sources = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&sources);

    parent(&fixture, me(), 500);
    scene.scripts.lock().unwrap().on_start = Some(Box::new(move |_, path| {
        seen.lock()
            .unwrap()
            .push(std::fs::read_to_string(path).unwrap());
    }));

    let ctx = fixture.ctx();

    keep_above(&ctx, true).unwrap();
    keep_above(&ctx, false).unwrap();

    let sources = sources.lock().unwrap();

    assert!(sources[0].contains(&format!("const pids = [{}, 500];", me())));
    assert!(sources[0].contains("keepAbove = true"));
    assert!(sources[1].contains("keepAbove = false"));
    assert_eq!(scene.scripts.lock().unwrap().loaded.len(), 0);
}

#[test]
fn a_kwin_without_the_scripting_or_with_a_refusal_is_an_error() {
    let fixture = dark();

    assert!(
        keep_above(&fixture.ctx(), true)
            .unwrap_err()
            .contains("session bus")
    );

    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    scene.scripts.lock().unwrap().refuse = true;
    assert!(
        keep_above(&fixture.ctx(), true)
            .unwrap_err()
            .contains("did not load the script")
    );
}

#[test]
fn a_run_that_cannot_keep_the_window_above_says_so_and_goes_on() {
    let fixture = dark();
    let mut ctx = fixture.ctx();

    fixture.runner.rule("pgrep", 1, "");
    fixture.console.skip_at(">>>");
    asus_ux8406_kde_live_test::guided::run(&mut ctx);
    expect(&ctx, "terminal window above the others", "OK");
    assert!(detail(&ctx, "terminal window above the others").contains("information, not a fault"));
}

//! Where the program does nothing, or stops.

use std::process::Command;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use asus_ux8406_kde_fakes::{Bus, Compositor, Program, Style, program};

use crate::{PROGRAM, SERVICE, Session, UX8406CA, panels, touch};

fn run(command: Command) -> (Option<i32>, Vec<String>) {
    let (status, log) = Program::start(command).end();

    (status.code(), log)
}

fn on_ux8406ca(mut command: Command) -> Command {
    command.args(["--root", UX8406CA]);
    command
}

fn said(log: &[String], text: &str) -> bool {
    log.iter().any(|line| line.contains(text))
}

#[test]
fn on_another_model_it_does_nothing() {
    let root = TempDir::new().expect("directory");

    root.copy_from(UX8406CA, &["**"]).expect("copy");
    root.child("class/dmi/id/product_name")
        .write_str("ASUS Zenbook 14 UX3405MA_UX3405MA\n")
        .expect("product name");

    let mut command = program::command(PROGRAM, None, None);

    command.arg("-r").arg(root.path());

    let (code, log) = run(command);

    assert_eq!(code, Some(0));
    assert!(said(&log, "this is not a Zenbook Duo UX8406"), "{log:?}");
}

#[test]
fn without_a_wayland_session_it_stops() {
    let (code, log) = run(on_ux8406ca(program::command(PROGRAM, None, None)));

    assert_eq!(code, Some(1));
    assert!(said(&log, "no Wayland session"), "{log:?}");
}

#[test]
fn with_another_compositor_it_stops() {
    let compositor = Compositor::new(Style::NoManagement, panels());
    let (code, log) = run(on_ux8406ca(program::command(
        PROGRAM,
        Some(&compositor),
        None,
    )));

    assert_eq!(code, Some(1));
    assert!(said(&log, "not KWin"), "{log:?}");
}

#[test]
fn without_a_session_bus_it_stops() {
    let compositor = Compositor::new(Style::Registry, panels());
    let (code, log) = run(on_ux8406ca(program::command(
        PROGRAM,
        Some(&compositor),
        None,
    )));

    assert_eq!(code, Some(1));
    assert!(said(&log, "session bus: "), "{log:?}");
}

#[test]
fn a_second_one_stops() {
    let session = Session::new(panels(), Some(touch()));
    let _first = session.start();
    let (code, log) = run(on_ux8406ca(program::command(
        PROGRAM,
        Some(&session.compositor),
        Some(&session.bus),
    )));

    assert_eq!(code, Some(1));
    assert!(said(&log, "session bus: "), "{log:?}");
}

#[test]
fn another_owner_of_the_name_keeps_it() {
    let compositor = Compositor::new(Style::Registry, panels());
    let bus = Bus::new();
    let owner = bus.connect();

    owner.request_name(SERVICE).expect("name");

    let (code, _) = run(on_ux8406ca(program::command(
        PROGRAM,
        Some(&compositor),
        Some(&bus),
    )));

    assert_eq!(code, Some(1));
}

#[test]
fn under_the_journal_each_line_has_its_priority() {
    let mut command = on_ux8406ca(program::command(PROGRAM, None, None));

    command.env("JOURNAL_STREAM", "8:1234");

    let (_, log) = run(command);

    assert!(log.iter().any(|line| line.starts_with("<3>")), "{log:?}");
}

#[test]
fn help_and_version() {
    for option in ["--help", "--version"] {
        let output = Command::new(PROGRAM).arg(option).output().expect("program");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("asus-ux8406-kde-keys"));
    }
}

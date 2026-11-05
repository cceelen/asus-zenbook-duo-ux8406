//! Where the program stops, or goes on without a part.

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use asus_ux8406_kde_fakes::{Compositor, Program, Style, program};

use crate::{PROGRAM, SERVICE, Session, panels};

fn run(command: Command) -> (Option<i32>, Vec<String>) {
    let (status, log) = Program::start(command).end();

    (status.code(), log)
}

fn said(log: &[String], text: &str) -> bool {
    log.iter().any(|line| line.contains(text))
}

#[test]
fn without_a_configuration_directory_it_stops() {
    for action in [None, Some("restore")] {
        let mut command = program::command(PROGRAM, None, None);

        command.env_remove("HOME");
        command.args(action);

        let (code, log) = run(command);

        assert_eq!(code, Some(1));
        assert!(
            said(&log, "neither XDG_CONFIG_HOME nor HOME is set"),
            "{log:?}"
        );
    }
}

#[test]
fn home_is_the_configuration_directory_without_xdg_config_home() {
    let session = Session::new(panels(), None);
    let mut command = program::command(PROGRAM, Some(&session.compositor), Some(&session.bus));

    command
        .env("HOME", session.config.path())
        .env("XDG_CONFIG_HOME", "");

    let program = Program::start(command);

    program.wait_log("ready");
    program.wait_for("the policies kept", || {
        session
            .config
            .path()
            .join(".config/plasma-builtin-screen-rotation.toml")
            .exists()
    });
}

#[test]
fn a_configuration_that_is_not_one_stops_it() {
    let session = Session::new(panels(), None);

    std::fs::write(session.file(), "turned = 3\n").expect("configuration");

    let (code, log) = run(session.command(&[]));

    assert_eq!(code, Some(1));
    assert!(said(&log, "unknown field"), "{log:?}");
}

#[test]
fn a_configuration_that_cannot_be_kept_is_said() {
    let session = Session::new(panels(), Some("normal"));

    std::fs::write(session.file(), "enabled = true\n").expect("configuration");
    std::fs::set_permissions(
        session.config.path(),
        std::fs::Permissions::from_mode(0o555),
    )
    .expect("read only");

    let program = session.start();

    program.wait_log("plasma-builtin-screen-rotation.toml: ");
    drop(program);
    std::fs::set_permissions(
        session.config.path(),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("writable");
}

#[test]
fn without_a_wayland_session_it_stops() {
    let session = Session::new(panels(), None);
    let mut command = program::command(PROGRAM, None, Some(&session.bus));

    command.env("XDG_CONFIG_HOME", session.config.path());

    let (code, log) = run(command);

    assert_eq!(code, Some(1));
    assert!(said(&log, "no Wayland session"), "{log:?}");
}

#[test]
fn with_another_compositor_it_stops() {
    let session = Session::of_style(Style::NoManagement, panels(), None);
    let (code, log) = run(session.command(&["run"]));

    assert_eq!(code, Some(1));
    assert!(said(&log, "not KWin"), "{log:?}");
}

#[test]
fn without_a_session_bus_it_stops() {
    let compositor = Compositor::new(Style::Registry, panels());
    let config = tempfile::TempDir::new().expect("directory");
    let mut command = program::command(PROGRAM, Some(&compositor), None);

    command.env("XDG_CONFIG_HOME", config.path());

    let (code, log) = run(command);

    assert_eq!(code, Some(1));
    assert!(said(&log, "no orientation sensor: system bus: "), "{log:?}");
    assert!(said(&log, "session bus: "), "{log:?}");
}

#[test]
fn another_owner_of_the_name_keeps_it() {
    let session = Session::new(panels(), None);
    let owner = session.bus.connect();

    owner.request_name(SERVICE).expect("name");

    let (code, _) = run(session.command(&[]));

    assert_eq!(code, Some(1));
}

#[test]
fn the_program_ends_with_kwin() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.compositor.kick();

    let (status, log) = program.end();

    assert_eq!(status.code(), Some(1));
    assert!(said(&log, "Wayland"), "{log:?}");
}

#[test]
fn sigterm_ends_it_cleanly() {
    let session = Session::new(panels(), Some("normal"));
    let (status, _) = session.start().stop();

    assert!(status.success());
}

#[test]
fn help_and_version() {
    for option in ["--help", "--version"] {
        let output = Command::new(PROGRAM).arg(option).output().expect("program");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("plasma-builtin-screen-rotation"));
    }
}

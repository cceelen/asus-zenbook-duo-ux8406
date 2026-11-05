//! A compositor that is not KWin, or a connection that ends.

use std::os::unix::net::{UnixListener, UnixStream};

use asus_ux8406_kde_common::output::{Error, Session};
use asus_ux8406_kde_fakes::{Compositor, Style};
use wayland_client::Connection;

use crate::{panels, session};

#[test]
fn a_compositor_without_output_management_is_not_kwin() {
    let compositor = Compositor::new(Style::NoManagement, panels());
    let stream = UnixStream::connect(compositor.socket()).expect("socket");
    let result = Session::with(Connection::from_socket(stream).expect("connection"));

    assert!(matches!(result, Err(Error::NotKwin)));
    assert_eq!(
        Error::NotKwin.to_string(),
        "the compositor is not KWin: it has no output management"
    );
}

#[test]
fn a_compositor_that_closes_at_once_gives_no_globals() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("closing");
    let listener = UnixListener::bind(&path).expect("socket");
    let closing = std::thread::spawn(move || drop(listener.accept()));
    let stream = UnixStream::connect(&path).expect("socket");

    closing.join().expect("thread");

    let result = Session::with(Connection::from_socket(stream).expect("connection"));

    assert!(
        matches!(result, Err(Error::Globals(_))),
        "{:?}",
        result.err()
    );
}

#[test]
fn without_a_session_there_is_nothing_to_connect_to() {
    // The tests do not have WAYLAND_DISPLAY to a compositor of their own;
    // the variable of a desktop session that runs them is not used here.
    let error = Error::from(wayland_client::ConnectError::NoCompositor);

    assert!(error.to_string().starts_with("no Wayland session"));
}

#[test]
fn nothing_to_read_is_no_error() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);

    assert!(session.dispatch().is_ok());
    assert_eq!(session.take_events(), []);
    assert!(std::os::fd::AsRawFd::as_raw_fd(&session.fd()) >= 0);
}

#[test]
fn a_connection_that_ends_fails_each_call() {
    let compositor = Compositor::new(Style::Registry, panels());
    let mut session = session(&compositor);

    compositor.kick();
    drop(compositor);

    let number = session.apply(&[]);

    // The first call that reads finds the end; the connection stays dead.
    assert!(session.roundtrip().is_err());
    assert!(session.dispatch().is_err());
    assert!(session.flush().is_err());
    assert!(session.wait(number).is_err());
}

/// The tests below run this binary again with the environment of a session.
const CHILD: &str = "ASUS_UX8406_TEST_CONNECT";

#[test]
fn a_child_connects_to_the_compositor_of_its_environment() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    assert!(Session::connect().is_ok());
}

#[test]
fn a_child_without_a_compositor_gets_an_error() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    assert!(matches!(Session::connect(), Err(Error::Connect(_))));
}

fn child(test: &str, display: Option<&Compositor>) -> bool {
    let mut command = std::process::Command::new(std::env::current_exe().expect("test binary"));

    command
        .args(["--exact", test])
        .env(CHILD, "1")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET");
    if let Some(compositor) = display {
        command
            .env("XDG_RUNTIME_DIR", compositor.runtime_dir())
            .env("WAYLAND_DISPLAY", compositor.display());
    } else {
        command.env("XDG_RUNTIME_DIR", "/nonexistent");
    }
    let output = command.output().expect("child");

    // The child must have run the test, not found none.
    output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed")
}

#[test]
fn connect_follows_the_environment() {
    let compositor = Compositor::new(Style::Registry, panels());

    assert!(child(
        "connection::a_child_connects_to_the_compositor_of_its_environment",
        Some(&compositor)
    ));
    assert!(child(
        "connection::a_child_without_a_compositor_gets_an_error",
        None
    ));
}

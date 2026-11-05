//! The client of KWin's output management, against the fake compositor of
//! kde-fakes: one module per topic. All of them run in this one test
//! program, in parallel threads; each test has a compositor of its own.

mod apply;
mod connection;
mod description;
mod hotplug;
mod modes;
mod policy;

use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use asus_ux8406_kde_common::output::{Event, Session};
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use wayland_client::Connection;

/// The upper panel.
const UPPER: &str = "eDP-1";
/// The lower panel.
const LOWER: &str = "eDP-2";

/// The two panels, one below the other, the upper one the primary.
fn panels() -> Vec<Output> {
    vec![
        Output {
            priority: 1,
            ..Output::panel(UPPER, 0, 0)
        },
        Output::panel(LOWER, 0, 1440),
    ]
}

/// A client of the compositor.
fn session(compositor: &Compositor) -> Session {
    let stream = UnixStream::connect(compositor.socket()).expect("socket");

    Session::with(Connection::from_socket(stream).expect("connection")).expect("session")
}

/// Read events until `done` is true of the session, at most five seconds.
fn until(session: &mut Session, what: &str, done: impl Fn(&Session) -> bool) {
    let end = Instant::now() + Duration::from_secs(5);

    while !done(session) {
        assert!(Instant::now() < end, "waited in vain: {what}");
        session.roundtrip().expect("roundtrip");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Wait for KWin's answer to a configuration. If it does not come in three
/// seconds, the connection is shut, so that the wait fails at once and the
/// test with it, and not when the test run runs out of time.
fn wait(
    session: &mut Session,
    number: u64,
) -> Result<Result<(), String>, asus_ux8406_kde_common::output::Error> {
    let socket = UnixStream::from(session.fd().try_clone_to_owned().expect("descriptor"));
    let (done, ticks) = std::sync::mpsc::channel::<()>();

    std::thread::scope(|scope| {
        scope.spawn(move || {
            if ticks.recv_timeout(Duration::from_secs(3)).is_err() {
                let _ = socket.shutdown(std::net::Shutdown::Both);
            }
        });

        let answer = session.wait(number);

        drop(done);
        answer
    })
}

/// Apply a layout and wait for KWin's answer.
fn apply(
    session: &mut Session,
    layout: &[asus_ux8406_kde_common::geometry::Monitor],
) -> Result<(), String> {
    let number = session.apply(layout);

    session.flush().expect("flush");
    wait(session, number).expect("answer")
}

/// The compositors of KWin 6.6 and 6.8.
const STYLES: [Style; 2] = [Style::Registry, Style::Globals];

/// Whether an event list says that the outputs changed.
fn changed(events: &[Event]) -> bool {
    events.contains(&Event::Changed)
}

//! SIGTERM ends the loop. The signal goes to the whole process, so this is the
//! only test of its binary.

use std::os::unix::net::UnixStream;
use std::time::Instant;

use asus_ux8406_kde_common::daemon::{self, Handler};
use asus_ux8406_kde_common::output::{Event, Session};
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use wayland_client::Connection;

/// Raises SIGTERM at its first tick: the loop calls it only after it has set
/// up its signals.
struct Idle {
    raised: bool,
}

impl Handler for Idle {
    type Command = ();

    fn outputs(&mut self, _: &mut Session, _: Vec<Event>) {}

    fn command(&mut self, _: &mut Session, (): ()) {}

    fn tick(&mut self, _: &mut Session) -> Option<Instant> {
        if !self.raised {
            self.raised = true;
            signal_hook::low_level::raise(signal_hook::consts::SIGTERM).expect("raise");
        }
        None
    }

    fn done(&self) -> bool {
        false
    }
}

#[test]
fn sigterm_ends_the_loop_without_an_error() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel("eDP-1", 0, 0)]);
    let stream = UnixStream::connect(compositor.socket()).expect("socket");
    let session =
        Session::with(Connection::from_socket(stream).expect("connection")).expect("session");
    let (_sender, commands) = calloop::channel::channel();

    assert!(daemon::run(session, Idle { raised: false }, commands).is_ok());
}

//! The event loop of the KDE programs, with a handler that writes down what
//! it is given. The signals are tested with the programs themselves: a
//! signal to this test process would reach each test at once.

use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use asus_ux8406_kde_common::daemon::{self, Error, Handler};
use asus_ux8406_kde_common::output::{Event, Session};
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use wayland_client::Connection;

/// What the handler was given.
type Notes = Arc<Mutex<Vec<String>>>;

struct Probe {
    notes: Notes,
    /// When to wake up, once.
    alarm: Option<Instant>,
    /// Done after this many notes.
    enough: usize,
}

impl Probe {
    fn note(&self, text: String) {
        self.notes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(text);
    }
}

impl Handler for Probe {
    type Command = &'static str;

    fn outputs(&mut self, _: &mut Session, events: Vec<Event>) {
        self.note(format!("outputs {events:?}"));
    }

    fn command(&mut self, _: &mut Session, command: &'static str) {
        self.note(format!("command {command}"));
    }

    fn tick(&mut self, _: &mut Session) -> Option<Instant> {
        if self.alarm.is_some_and(|at| at <= Instant::now()) {
            self.alarm = None;
            self.note("alarm".to_owned());
        }
        self.alarm
    }

    fn done(&self) -> bool {
        self.notes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
            >= self.enough
    }
}

fn session(compositor: &Compositor) -> Session {
    let stream = UnixStream::connect(compositor.socket()).expect("socket");

    Session::with(Connection::from_socket(stream).expect("connection")).expect("session")
}

fn probe(alarm: Option<Duration>, enough: usize) -> (Probe, Notes) {
    let notes = Notes::default();

    (
        Probe {
            notes: Arc::clone(&notes),
            alarm: alarm.map(|after| Instant::now() + after),
            enough,
        },
        notes,
    )
}

/// Run the loop in a thread and wait three seconds for its end at most. A
/// loop that does not end then is left to run: the test fails at once, and
/// not when the test run runs out of time.
fn run_within(
    session: Session,
    handler: Probe,
    commands: calloop::channel::Channel<&'static str>,
) -> Result<(), Error> {
    let (ended, result) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let _ = ended.send(daemon::run(session, handler, commands));
    });
    result
        .recv_timeout(Duration::from_secs(3))
        .expect("the loop ends")
}

fn notes(notes: &Notes) -> Vec<String> {
    notes.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

#[test]
fn commands_come_in_order_and_the_end_of_the_senders_stops_the_loop() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel("eDP-1", 0, 0)]);
    let (handler, written) = probe(None, usize::MAX);
    let (sender, commands) = calloop::channel::channel();

    sender.send("one").expect("send");
    sender.send("two").expect("send");
    drop(sender);
    assert!(run_within(session(&compositor), handler, commands).is_ok());
    assert_eq!(notes(&written), ["command one", "command two"]);
}

#[test]
fn the_handler_wakes_up_at_its_time() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel("eDP-1", 0, 0)]);
    let start = Instant::now();
    let (handler, written) = probe(Some(Duration::from_millis(30)), 1);
    let (_sender, commands) = calloop::channel::channel();

    assert!(run_within(session(&compositor), handler, commands).is_ok());
    assert_eq!(notes(&written), ["alarm"]);
    assert!(start.elapsed() >= Duration::from_millis(30));
}

#[test]
fn a_change_of_the_outputs_reaches_the_handler() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel("eDP-1", 0, 0)]);
    let (handler, written) = probe(None, 1);
    let (_sender, commands) = calloop::channel::channel();
    let changer = std::thread::spawn({
        let socket = compositor.socket();

        move || {
            // The loop runs by then; the compositor is shared through its
            // socket only, so the change comes from a second client.
            std::thread::sleep(Duration::from_millis(100));
            let mut other = Session::with(
                Connection::from_socket(UnixStream::connect(socket).expect("socket"))
                    .expect("connection"),
            )
            .expect("session");
            let sent = other.set_policies(&[(
                "eDP-1".to_owned(),
                asus_ux8406_kde_common::output::Policy::Always,
            )]);

            other.flush().expect("flush");
            sent.expect("policy")
        }
    });

    assert!(run_within(session(&compositor), handler, commands).is_ok());
    changer.join().expect("thread");
    assert_eq!(notes(&written), ["outputs [Changed]"]);
}

#[test]
fn the_end_of_kwin_ends_the_loop_with_its_error() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel("eDP-1", 0, 0)]);
    let (handler, _) = probe(None, usize::MAX);
    let (_sender, commands) = calloop::channel::channel();
    let session = session(&compositor);

    compositor.kick();

    let result = run_within(session, handler, commands);

    assert!(matches!(result, Err(Error::Output(_))), "{result:?}");
}

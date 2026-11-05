//! The event loop of the two KDE programs: the events of KWin's outputs,
//! the commands that come over D-Bus, a time to wait for, and the signals
//! that stop the program.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::time::Instant;

use calloop::channel::{self, Channel};
use calloop::generic::Generic;
use calloop::{EventLoop, Interest, Mode, PostAction};
use log::{Level, LevelFilter};
use signal_hook::consts::{SIGINT, SIGTERM};

use crate::output::{Event, Session};

/// The syslog priorities of the levels, from `Error` to `Trace`, as the
/// journal reads them at the start of a line.
const PRIORITY: [&str; 5] = ["<3>", "<4>", "<6>", "<7>", "<7>"];

/// Log one line per message to standard error. Where the journal reads
/// it, each line carries its priority; the journal adds the time. The level
/// is `info` for the programs of this tree and `warn` for the crates they
/// use, or what `RUST_LOG` says.
pub fn start_logging() {
    let journal = std::env::var_os("JOURNAL_STREAM").is_some();

    env_logger::Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("asus_ux8406", LevelFilter::Info)
        .parse_default_env()
        .format(move |out, record| {
            let priority = if journal {
                PRIORITY[record.level() as usize - Level::Error as usize]
            } else {
                ""
            };

            writeln!(out, "{priority}{}", record.args())
        })
        .init();
}

/// What a program does with what comes in.
pub trait Handler {
    /// What comes over D-Bus.
    type Command;

    /// The outputs changed, or KWin answered a configuration.
    fn outputs(&mut self, session: &mut Session, events: Vec<Event>);

    /// A command came.
    fn command(&mut self, session: &mut Session, command: Self::Command);

    /// Do what is due now, and say when to come again: none for only when
    /// something comes in. Called before each wait.
    fn tick(&mut self, session: &mut Session) -> Option<Instant>;

    /// Whether to stop, as a command can ask.
    fn done(&self) -> bool;
}

/// Why the event loop ended other than by a signal or by the handler.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The connection to KWin failed.
    #[error(transparent)]
    Output(#[from] crate::output::Error),
    /// The event loop could not be set up or failed.
    #[error("event loop: {0}")]
    Loop(#[from] calloop::Error),
}

/// What the callbacks of the loop share.
struct Data<H> {
    session: Session,
    handler: H,
    stop: bool,
    failure: Option<crate::output::Error>,
    broken: Option<calloop::Error>,
}

/// Run until SIGTERM or SIGINT, until the handler is done, or until the
/// connection to KWin fails. It logs "ready" when the signals are watched:
/// from then on SIGTERM ends it cleanly.
///
/// # Errors
///
/// [`Error::Output`] if the connection to KWin failed, [`Error::Loop`] if
/// the loop could not be set up or failed.
pub fn run<H: Handler + 'static>(
    session: Session,
    handler: H,
    commands: Channel<H::Command>,
) -> Result<(), Error> {
    let mut data = Data {
        session,
        handler,
        stop: false,
        failure: None,
        broken: None,
    };

    EventLoop::<Data<H>>::try_new()
        .and_then(|event_loop| watch(&event_loop, &data.session, commands).map(|()| event_loop))
        .map(|mut event_loop| {
            log::info!("ready");
            turn(&mut event_loop, &mut data);
        })
        .map_err(Error::from)
        .and(outcome(data))
}

/// How the loop ended.
fn outcome<H>(data: Data<H>) -> Result<(), Error> {
    data.failure
        .map(Error::from)
        .or(data.broken.map(Error::from))
        .map_or(Ok(()), Err)
}

/// A socket that becomes readable at SIGTERM or SIGINT.
fn signal_pipe() -> std::io::Result<UnixStream> {
    UnixStream::pair().and_then(|(signals, wakeup)| {
        [SIGTERM, SIGINT]
            .into_iter()
            .try_for_each(|signal| {
                wakeup
                    .try_clone()
                    .and_then(|pipe| signal_hook::low_level::pipe::register(signal, pipe))
                    .map(drop)
            })
            .and_then(|()| signals.set_nonblocking(true))
            .map(|()| signals)
    })
}

/// Put the sources into the loop: the signals, the connection to KWin and
/// the commands.
fn watch<H: Handler + 'static>(
    event_loop: &EventLoop<'_, Data<H>>,
    session: &Session,
    commands: Channel<H::Command>,
) -> Result<(), calloop::Error> {
    let handle = event_loop.handle();

    signal_pipe()
        .map_err(calloop::Error::from)
        .and_then(|signals| {
            handle
                .insert_source(
                    Generic::new(signals, Interest::READ, Mode::Level),
                    |_, _, data| Ok(on_signal(data)),
                )
                .map_err(calloop::Error::from)
        })
        .and_then(|_| {
            session
                .fd()
                .try_clone_to_owned()
                .map_err(calloop::Error::from)
        })
        .and_then(|wayland| {
            handle
                .insert_source(
                    Generic::new(wayland, Interest::READ, Mode::Level),
                    |_, _, data| Ok(on_outputs(data)),
                )
                .map_err(calloop::Error::from)
        })
        .and_then(|_| {
            handle
                .insert_source(commands, on_command)
                .map_err(calloop::Error::from)
        })
        .map(drop)
}

fn on_signal<H>(data: &mut Data<H>) -> PostAction {
    data.stop = true;
    PostAction::Remove
}

fn on_outputs<H: Handler>(data: &mut Data<H>) -> PostAction {
    match data.session.dispatch() {
        Ok(()) => {
            let events = data.session.take_events();

            data.handler.outputs(&mut data.session, events);
        }
        Err(error) => data.failure = Some(error),
    }
    PostAction::Continue
}

fn on_command<H: Handler>(event: channel::Event<H::Command>, (): &mut (), data: &mut Data<H>) {
    match event {
        channel::Event::Msg(command) => data.handler.command(&mut data.session, command),
        // No one can send any more.
        channel::Event::Closed => data.stop = true,
    }
}

/// Whether there is no reason to stop.
fn going<H: Handler>(data: &Data<H>) -> bool {
    !data.stop && !data.handler.done() && data.failure.is_none() && data.broken.is_none()
}

/// Wait and dispatch until there is a reason to stop. What the handler did
/// at its time can be such a reason.
fn turn<H: Handler>(event_loop: &mut EventLoop<'_, Data<H>>, data: &mut Data<H>) {
    while going(data) {
        let timeout = data
            .handler
            .tick(&mut data.session)
            .map(|at| at.saturating_duration_since(Instant::now()));

        if going(data) {
            data.failure = data.session.flush().err();
            data.broken = event_loop.dispatch(timeout, data).err();
        }
    }
    // What the handler asked at the end still goes to KWin; if that fails,
    // the program says so, unless it failed before.
    if let Err(error) = data.session.flush() {
        data.failure.get_or_insert(error);
    }
}

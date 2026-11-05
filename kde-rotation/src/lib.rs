//! The program that turns the built-in screens with the laptop in KDE
//! Plasma, as the GNOME extension `builtin-screen-rotation` does in GNOME:
//! both screens turned and side by side, in the order in which they lie,
//! and the layout from before upright again. Nothing in it is specific to
//! one model.
//!
//! It is a client of KWin's output management on Wayland (refer to
//! [`asus_ux8406_kde_common::output`]) and of iio-sensor-proxy on the system
//! bus ([`sensor`]), and it offers [`service::NAME`] on the session bus for
//! its applet in the system tray.

pub mod cli;
pub mod config;
pub mod rotation;
pub mod sensor;
pub mod service;
pub mod shortcut;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use asus_ux8406_kde_common::output::{Policy, Session};
use asus_ux8406_kde_common::{daemon, output};
use log::{info, warn};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::process::{Pid, PidfdFlags, pidfd_open};

pub use cli::{Action, Cli};
use rotation::Rotation;
use sensor::{Backoff, Sensor, Worker};
use service::Command;

/// Why the program stopped.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The outputs cannot be followed.
    #[error(transparent)]
    Outputs(#[from] output::Error),
    /// The session bus failed, or the service runs already.
    #[error("session bus: {0}")]
    Bus(#[from] zbus::Error),
    /// The configuration file cannot be read or written.
    #[error(transparent)]
    Config(#[from] config::Error),
    /// The event loop failed.
    #[error(transparent)]
    Loop(#[from] daemon::Error),
    /// KWin did not take the policies back.
    #[error("KWin did not take the policies: {0}")]
    Refused(String),
}

/// Do what the command line asks.
///
/// # Errors
///
/// Refer to [`Error`].
pub fn execute(cli: &Cli) -> Result<(), Error> {
    match cli.action.unwrap_or(Action::Run) {
        Action::Run => run(),
        Action::Restore => restore(),
    }
}

/// The worker for the sensor service, whose reports go to the program as
/// commands. None without a system bus.
fn sensor(sender: &calloop::channel::Sender<Command>) -> Option<Worker> {
    let sender = sender.clone();

    Sensor::connect()
        .and_then(|sensor| {
            Worker::spawn(sensor, Backoff::DEFAULT, move |report| {
                // The receiver ends with the program: a report is of no use then.
                let _ = sender.send(Command::Sensor(report));
            })
        })
        .inspect_err(|error| warn!("no orientation sensor: system bus: {error}"))
        .ok()
}

/// Follow the orientation of the laptop until the session ends.
///
/// # Errors
///
/// Refer to [`Error`].
pub fn run() -> Result<(), Error> {
    let path = config::path()?;
    let config = config::load(&path)?;
    let mut session = Session::connect()?;
    let (sender, commands) = calloop::channel::channel();
    let sensor = sensor(&sender);
    let status = service::Status {
        enabled: config.enabled,
        screens: Vec::new(),
        control: asus_ux8406_kde_common::rotation::Control::None,
        clockwise: false,
        counterclockwise: false,
    };
    let stopping = Arc::new(AtomicBool::new(false));
    let pressed = sender.clone();
    let bus = service::serve(sender, status, Arc::clone(&stopping))?;

    // Without the signals of kglobalacceld, the program works without the
    // shortcut, and says so. A kglobalacceld that is not there yet is no
    // error: the program registers when it comes.
    if let Err(error) = shortcut::watch(&bus, move || {
        // The receiver ends with the program: a key is of no use then.
        let _ = pressed.send(Command::Shortcut);
    }) {
        warn!("no global shortcut: {error}");
    }
    let rotation = Rotation::new(&mut session, bus, sensor, path, config, stopping);

    daemon::run(session, rotation, commands)?;
    Ok(())
}

/// How long `restore` waits for the program of the session to end.
const QUIT_PATIENCE: Duration = Duration::from_secs(5);

/// The process that owns the name of the service on the bus. None where no
/// program owns it: that is the normal case of `restore`, and the bus says it
/// as an error.
fn owner_process(bus: &zbus::blocking::Connection) -> Option<u32> {
    bus.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "GetConnectionUnixProcessID",
        &(service::NAME,),
    )
    .ok()
    .and_then(|reply| reply.body().deserialize::<u32>().ok())
}

/// Wait until the process has ended, at most `patience`. Whether it has
/// ended: a process that is gone, or one that cannot be watched, counts as
/// ended.
pub fn wait_ended(process: u32, patience: Duration) -> bool {
    i32::try_from(process)
        .ok()
        .and_then(Pid::from_raw)
        .and_then(|pid| pidfd_open(pid, PidfdFlags::empty()).ok())
        .is_none_or(|watch| {
            let mut fds = [PollFd::new(&watch, PollFlags::IN)];
            // A patience beyond the range of a time spec is a wait without
            // limit.
            let limit = Timespec::try_from(patience).ok();

            poll(&mut fds, limit.as_ref()).is_ok_and(|ready| ready > 0)
        })
}

/// The messages that say a new owner of the name of the service.
fn name_changes(bus: &zbus::blocking::Connection) -> zbus::Result<zbus::blocking::MessageIterator> {
    zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")
        .and_then(|rule| rule.member("NameOwnerChanged"))
        .and_then(|rule| rule.add_arg(service::NAME))
        .map(zbus::match_rule::Builder::build)
        .and_then(|rule| zbus::blocking::MessageIterator::for_match_rule(rule, bus, None))
}

/// Stop the program of this session, if it runs, and wait until its name
/// is free and its process has ended, each at most [`QUIT_PATIENCE`]. The
/// program can end before it answers `Quit`: the answer does not matter.
fn quit_running() {
    let Ok(bus) = zbus::blocking::Connection::session() else {
        return;
    };
    if let Some(process) = owner_process(&bus) {
        let (done, wait) = std::sync::mpsc::channel();
        let messages = name_changes(&bus)
            .inspect_err(|error| {
                warn!("the end of the program is not followed on the bus: {error}");
            })
            .ok();

        // The program can end before it answers: the answer does not matter.
        let _ = bus.call_method(
            Some(service::NAME),
            service::PATH,
            Some(service::NAME),
            "Quit",
            &(),
        );
        std::thread::spawn(move || {
            // Without the messages the wait for the name is over at once.
            // A message that cannot be read is not the release of the name.
            let _ = messages.into_iter().flatten().find(|message| {
                message
                    .as_ref()
                    .ok()
                    .and_then(|message| {
                        message
                            .body()
                            .deserialize::<(String, String, String)>()
                            .ok()
                    })
                    .is_some_and(|(_, _, owner)| owner.is_empty())
            });
            // The receiver stops waiting after the patience: then nobody needs this.
            let _ = done.send(());
        });

        let released = wait.recv_timeout(QUIT_PATIENCE).is_ok();
        let ended = wait_ended(process, QUIT_PATIENCE);

        if [released, ended].contains(&false) {
            warn!(
                "the program of the session did not end within {} s (name free: {released}, process gone: {ended})",
                QUIT_PATIENCE.as_secs(),
            );
        }
    }
}

/// Stop the program of this session, and give each built-in screen the
/// auto-rotate policy of KWin from before the program set it to `never`.
///
/// # Errors
///
/// Refer to [`Error`].
pub fn restore() -> Result<(), Error> {
    let path = config::path()?;

    quit_running();

    let mut config = config::load(&path)?;
    let mut session = Session::connect()?;
    let connected = session.outputs.state().connected;
    let policies: Vec<(String, Policy)> = config
        .policies
        .iter()
        .filter(|(connector, _)| connected.contains_key(*connector))
        .filter_map(|(connector, policy)| {
            Policy::parse(policy).map(|policy| (connector.clone(), policy))
        })
        .collect();

    if let Some(number) = session.set_policies(&policies) {
        session
            .flush()
            .and_then(|()| session.wait(number))?
            .map_err(Error::Refused)?;
    }
    for (connector, policy) in &policies {
        info!(
            "{connector}: KWin's auto-rotate is {} again",
            policy.as_str()
        );
        config.policies.remove(connector);
    }
    config::save(&path, &config)?;
    Ok(())
}

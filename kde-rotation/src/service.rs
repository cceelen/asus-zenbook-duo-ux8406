//! The D-Bus service of the program on the session bus: what the applet in
//! the system tray shows, and what it can ask.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use asus_ux8406_kde_common::rotation::{Control, Direction, Orientation, Screen};
use calloop::channel::Sender;
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::fdo::RequestNameFlags;

/// The name of the service.
pub const NAME: &str = "io.github.cceelen.BuiltinScreenRotation";
/// Its object.
pub const PATH: &str = "/io/github/cceelen/BuiltinScreenRotation";

/// What comes in for the event loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Follow the orientation of the laptop, or not.
    SetEnabled(bool),
    /// Follow the orientation of the laptop when it is off, and not when it
    /// is on.
    Toggle,
    /// Turn all screens by 90 degrees by hand, where no sensor gives the
    /// orientation.
    Turn(Direction),
    /// The user pressed the global shortcut: as `Toggle`.
    Shortcut,
    /// Turn one built-in screen to an orientation, by hand.
    TurnScreen(String, Orientation),
    /// End the program.
    Quit,
    /// The worker for the sensor service reports.
    Sensor(crate::sensor::Report),
}

/// What the applet shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// Whether the screens follow the orientation of the laptop: not while a
    /// screen is turned by hand.
    pub enabled: bool,
    /// The built-in screens that can be turned one by one.
    pub screens: Vec<Screen>,
    /// The control to show: the switch, the buttons, or none.
    pub control: Control,
    /// Whether a turn clockwise is possible.
    pub clockwise: bool,
    /// Whether a turn counterclockwise is possible.
    pub counterclockwise: bool,
}

struct Service {
    sender: Sender<Command>,
    stopping: Arc<AtomicBool>,
    status: Status,
}

#[zbus::interface(name = "io.github.cceelen.BuiltinScreenRotation")]
impl Service {
    /// Whether the screens follow the orientation of the laptop.
    #[zbus(property)]
    fn enabled(&self) -> bool {
        self.status.enabled
    }

    /// `switch` where a sensor gives the orientation, `buttons` where none
    /// does, `none` without a built-in screen.
    #[zbus(property)]
    fn control(&self) -> String {
        self.status.control.as_str().to_owned()
    }

    /// The built-in screens that are on, each as its connector, its name and
    /// how it is turned: `normal`, `left-up`, `right-up` or `bottom-up`; empty
    /// for a mirrored screen.
    #[zbus(property)]
    fn screens(&self) -> Vec<(String, String, String)> {
        self.status
            .screens
            .iter()
            .map(|screen| {
                (
                    screen.connector.clone(),
                    screen.name.clone(),
                    screen
                        .orientation
                        .map_or("", Orientation::as_str)
                        .to_owned(),
                )
            })
            .collect()
    }

    /// Whether a turn clockwise is possible (`buttons` only).
    #[zbus(property)]
    fn can_turn_clockwise(&self) -> bool {
        self.status.clockwise
    }

    /// Whether a turn counterclockwise is possible (`buttons` only).
    #[zbus(property)]
    fn can_turn_counterclockwise(&self) -> bool {
        self.status.counterclockwise
    }

    /// Follow the orientation of the laptop, or not. The program keeps the
    /// choice in its configuration file.
    fn set_enabled(&self, enabled: bool) {
        let _ = self.sender.send(Command::SetEnabled(enabled));
    }

    /// Follow the orientation of the laptop when the screens do not, and
    /// not when they do. The program decides, so a caller needs no state.
    fn toggle(&self) {
        let _ = self.sender.send(Command::Toggle);
    }

    /// Turn one built-in screen to `normal`, `left-up`, `right-up` or
    /// `bottom-up`. The screens follow the sensor no more until `SetEnabled`
    /// or `Toggle` switches the rotation on again; the saved setting stays.
    fn turn_screen(&self, connector: &str, orientation: &str) -> zbus::fdo::Result<()> {
        let orientation = Some(Orientation::from_sensor(orientation))
            .filter(|orientation| orientation.transform().is_some())
            .ok_or_else(|| {
                zbus::fdo::Error::InvalidArgs(format!("no orientation: {orientation}"))
            })?;

        let _ = self
            .sender
            .send(Command::TurnScreen(connector.to_owned(), orientation));
        Ok(())
    }

    /// Turn the screens by 90 degrees: `clockwise` or `counterclockwise`.
    fn turn(&self, direction: &str) -> zbus::fdo::Result<()> {
        Direction::parse(direction)
            .map(|direction| {
                let _ = self.sender.send(Command::Turn(direction));
            })
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("no direction: {direction}")))
    }

    /// End the program, as `plasma-builtin-screen-rotation restore` asks.
    /// The program stops all changes to the outputs before this call
    /// returns.
    fn quit(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        let _ = self.sender.send(Command::Quit);
    }
}

/// Connect to the session bus and take the name of the service. The
/// commands go to `sender`; `stopping` becomes true as soon as a caller asks
/// the program to quit.
///
/// # Errors
///
/// There is no session bus, or another program has the name.
pub fn serve(
    sender: Sender<Command>,
    status: Status,
    stopping: Arc<AtomicBool>,
) -> zbus::Result<Connection> {
    Builder::session()
        .and_then(|builder| {
            builder.serve_at(
                PATH,
                Service {
                    sender,
                    stopping,
                    status,
                },
            )
        })
        .and_then(Builder::build)
        .and_then(|connection| {
            connection
                .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
                .map(|_| connection)
        })
}

/// Show a new status, and tell the applet what changed. A signal that
/// cannot be sent is lost: the applet reads the properties again when the
/// service comes back.
pub fn publish(connection: &Connection, status: Status) {
    let _: zbus::Result<()> = connection
        .object_server()
        .interface::<_, Service>(PATH)
        .and_then(|service| {
            let emitter = service.signal_emitter();
            let mut object = service.get_mut();

            object.status = status;
            zbus::block_on(async {
                [
                    object.enabled_changed(emitter).await,
                    object.control_changed(emitter).await,
                    object.screens_changed(emitter).await,
                    object.can_turn_clockwise_changed(emitter).await,
                    object.can_turn_counterclockwise_changed(emitter).await,
                ]
                .into_iter()
                .collect()
            })
        });
}

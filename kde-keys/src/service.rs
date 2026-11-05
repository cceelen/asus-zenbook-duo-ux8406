//! The D-Bus service of the program on the session bus, which the KWin
//! script calls when the key right of F12 is pressed.

use calloop::channel::Sender;
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::fdo::RequestNameFlags;

/// The name of the service.
pub const NAME: &str = "io.github.cceelen.AsusUX8406.Keys";
/// Its object.
pub const PATH: &str = "/io/github/cceelen/AsusUX8406/Keys";

/// What comes in for the event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Switch the lower screen on or off.
    Toggle,
    /// KWin has a new input device.
    Devices,
}

struct Service(Sender<Command>);

#[zbus::interface(name = "io.github.cceelen.AsusUX8406.Keys")]
impl Service {
    /// Switch the lower screen on or off. With the keyboard on the lower
    /// screen this does nothing.
    fn toggle_lower_screen(&self) {
        let _ = self.0.send(Command::Toggle);
    }
}

/// Connect to the session bus and take the name of the service. The
/// commands go to `sender`.
///
/// # Errors
///
/// There is no session bus, or another program has the name.
pub fn serve(sender: Sender<Command>) -> zbus::Result<Connection> {
    Builder::session()
        .and_then(|builder| builder.serve_at(PATH, Service(sender)))
        .and_then(Builder::build)
        .and_then(|connection| {
            connection
                .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
                .map(|_| connection)
        })
}

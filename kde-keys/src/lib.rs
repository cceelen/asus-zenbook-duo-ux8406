//! The program for the ASUS Zenbook Duo UX8406 in KDE Plasma that does what
//! the KWin script cannot do itself: switch the lower screen on and off for
//! the key right of F12, and give each touchscreen and pen its panel.
//!
//! It is a client of KWin's output management on Wayland (refer to
//! [`asus_ux8406_kde_common::output`]) and of KWin's input devices on D-Bus
//! ([`input`]), and it offers [`service::NAME`] on the session bus, which
//! the KWin script calls. On any other model it does nothing.

pub mod cli;
pub mod input;
pub mod keys;
pub mod machine;
pub mod service;

use asus_ux8406_kde_common::output::Session;
use asus_ux8406_kde_common::touch::controllers_of;
use asus_ux8406_kde_common::{daemon, output};
use log::{info, warn};

pub use cli::Cli;
use keys::Keys;
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
    /// The event loop failed.
    #[error(transparent)]
    Loop(#[from] daemon::Error),
}

/// Run until the session ends: SIGTERM, SIGINT, or the end of KWin.
///
/// # Errors
///
/// Refer to [`Error`].
pub fn run(cli: &Cli) -> Result<(), Error> {
    if !machine::is_model(&cli.root) {
        info!(
            "this is not a Zenbook Duo {}: nothing to do",
            machine::MODEL
        );
        return Ok(());
    }

    // Without the list the touchscreens get no panel, but the key works.
    let controllers = match machine::i2c_devices(&cli.root) {
        Ok(devices) => controllers_of(&devices),
        Err(error) => {
            warn!("touch controllers: {error}");
            Vec::new()
        }
    };
    let session = Session::connect()?;
    let (sender, commands) = calloop::channel::channel();
    let added = sender.clone();
    let bus = service::serve(sender).and_then(|bus| {
        input::on_added(&bus, move || {
            // The program ends with the receiver: a device is of no use then.
            let _ = added.send(Command::Devices);
        })
        .map(|()| bus)
    })?;
    let keys = Keys::new(bus, controllers);

    keys.assign(&session.outputs.state());
    daemon::run(session, keys, commands)?;
    Ok(())
}

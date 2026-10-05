//! Why asus-ux8406-second-screen could not do its work.

use std::io;

use std::path::PathBuf;
use thiserror::Error as ThisError;

use crate::dock::Panel;

/// Why the lower panel's connector was not set.
#[derive(Debug, ThisError)]
pub enum Error {
    /// The machine is not the model whose port and connector are known.
    #[error("this is not a Zenbook Duo {}; nothing done", crate::sysfs::MODEL)]
    UnsupportedModel,
    /// There is no connector for the lower panel.
    #[error("no lower panel connector")]
    NoLowerPanel,
    /// A panel has no backlight device.
    #[error("no backlight for the {0} panel")]
    NoBacklight(Panel),
    /// A backlight attribute does not hold a brightness.
    #[error("{}: not a brightness", .0.display())]
    BadBrightness(PathBuf),
    /// A file could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// What the system said.
        source: io::Error,
    },
}

impl Error {
    /// Whether a write was refused because something else has taken the
    /// device over.
    ///
    /// A kernel with the DRM backlight property refuses writes to a panel's
    /// backlight with `EBUSY` while a compositor sets the brightness through
    /// that property. The compositor then looks after both panels itself.
    #[must_use]
    pub fn is_taken_over(&self) -> bool {
        matches!(self, Self::Io { source, .. } if source.kind() == io::ErrorKind::ResourceBusy)
    }
}

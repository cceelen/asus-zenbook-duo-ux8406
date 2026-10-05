//! Why the guard does not start, or stops.

use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::input::MAX_PACKAGES;

/// Why the guard does not start, or stops.
#[derive(Debug, Error)]
pub enum Error {
    /// A file could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// What the system said.
        source: io::Error,
    },
    /// A file holds something other than the value expected of it.
    #[error("{}: not the value expected there", .0.display())]
    BadValue(PathBuf),
    /// A file or directory that has to be the administrator's is not.
    #[error(
        "{}: has to be a regular file or a directory, not too big, owned by root or by you, \
         and writable by nobody else", .0.display()
    )]
    NotTrusted(PathBuf),
    /// What the configuration file holds is not acceptable.
    #[error("{}: {reason}", path.display())]
    BadConfig {
        /// The configuration file.
        path: PathBuf,
        /// What is wrong with it.
        reason: String,
    },
    /// Settings that are each acceptable but do not go together.
    #[error(
        "the settings contradict each other: probe_interval has to be at least interval, \
         probe_max at least probe_interval, and tjmax 0 or a plausible limit"
    )]
    Contradiction,
    /// The machine is not the one the settings are for.
    #[error(
        "this machine is {found:?}, not a {wanted}: set \"model\" in the configuration to run \
         here"
    )]
    WrongModel {
        /// What the product name has to contain.
        wanted: String,
        /// The product name.
        found: String,
    },
    /// No coretemp package temperature sensor was found.
    #[error("no coretemp package temperature sensor")]
    NoSensor,
    /// A package has two temperature sensors.
    #[error("package {0} has two temperature sensors")]
    TwoSensors(i32),
    /// There are more packages than one guard looks after.
    #[error("more than {MAX_PACKAGES} packages")]
    TooManyPackages,
    /// A package's limit is neither configured nor known to the kernel.
    #[error("package {0}: no usable temp1_crit; set tjmax")]
    NoTjmax(i32),
    /// The configuration names a package that is not there.
    #[error("package {0} is configured but not present")]
    NotPresent(i32),
    /// There is no telling where a package's offset is written.
    #[error("package {0}: name the file of its offset in a [[package]] table of the configuration")]
    NoControl(i32),
    /// The offset of a package cannot be written.
    #[error("{}: {source} (needs root)", path.display())]
    ReadOnly {
        /// The file of the offset.
        path: PathBuf,
        /// What the system said.
        source: io::Error,
    },
    /// The signals that stop the guard could not be set up.
    #[error("signals: {0}")]
    Signals(#[source] io::Error),
}

impl Error {
    /// The error of an operation on `path`, for `map_err`. It takes what
    /// the standard library and what rustix report.
    pub fn io<E: Into<io::Error>>(path: &Path) -> impl Fn(E) -> Self {
        |source| Self::Io {
            path: path.to_owned(),
            source: source.into(),
        }
    }

    /// Whether this says that a file is not there.
    #[must_use]
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Io { source, .. } if source.kind() == io::ErrorKind::NotFound)
    }
}

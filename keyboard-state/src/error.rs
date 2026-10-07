//! Why asus-ux8406-keyboard-state could not do its work.

use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error as ThisError;

/// Why the state of the keyboard was not saved or loaded.
#[derive(Debug, ThisError)]
pub enum Error {
    /// The name is not that of a HID device of the keyboard.
    #[error("{0}: not a HID device of the keyboard")]
    NotTheKeyboard(String),
    /// A map or a file does not hold a state.
    #[error("{}: not a keyboard state", .0.display())]
    BadState(PathBuf),
    /// A pinned map could not be read.
    #[error("{}: {reason}", path.display())]
    Map {
        /// The pin of the map.
        path: PathBuf,
        /// What the system said.
        reason: String,
    },
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
    /// A file that could not be read or written.
    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

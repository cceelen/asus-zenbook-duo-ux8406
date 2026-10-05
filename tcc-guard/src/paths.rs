//! Where the guard finds the system: everything below one root directory,
//! which is `/` unless the command line names another (`--root`).

use std::path::{Path, PathBuf};

/// The files and directories of the system that the guard uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// sysfs: sensors, offsets and the product name.
    pub sysfs: PathBuf,
    /// The kernel log.
    pub kmsg: PathBuf,
    /// Where the guard notes the offsets it has changed.
    pub state: PathBuf,
    /// The configuration file that is read if none is named.
    pub config: PathBuf,
}

impl Paths {
    /// The system below `root`.
    #[must_use]
    pub fn below(root: &Path) -> Self {
        Self {
            sysfs: root.join("sys"),
            kmsg: root.join("dev/kmsg"),
            state: root.join("run/asus-ux8406-tcc-guard"),
            config: root.join("etc/asus-ux8406-tcc-guard.toml"),
        }
    }
}

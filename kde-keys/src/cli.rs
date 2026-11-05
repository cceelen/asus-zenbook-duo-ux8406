//! The command line of asus-ux8406-kde-keys.

use std::path::PathBuf;

use clap::Parser;

/// What the command line asks for.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "asus-ux8406-kde-keys",
    version,
    about = "The lower screen key and the touchscreens of the ASUS Zenbook Duo UX8406 in KDE Plasma"
)]
pub struct Cli {
    /// Use this directory in place of /sys.
    #[arg(short, long, value_name = "DIR", default_value = "/sys")]
    pub root: PathBuf,
}

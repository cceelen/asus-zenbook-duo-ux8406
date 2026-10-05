//! The command line of asus-ux8406-second-screen.
//!
//! [`Cli`] is what `asus-ux8406-second-screen [-n] [-r ROOT] dock | brightness | release`
//! asks for; the help and version options are handled by the parser.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::{Mode, Sysfs};

/// The text of `--help`, and of the reply to a command line that is refused.
pub const USAGE: &str = "\
usage: asus-ux8406-second-screen [-n] [-r ROOT] dock | brightness | release
       asus-ux8406-second-screen -h | -V
  dock             lower panel off if the keyboard lies on it, else on,
                   at the brightness of the upper panel
  brightness       lower panel to the brightness of the upper panel
  release          lower panel on wherever the keyboard is (for removal)
  -n, --dry-run    say what would be written, write nothing
  -r, --root ROOT  use ROOT in place of /sys (for the tests)
  -h, --help       print this and exit
  -V, --version    print the version and exit";

/// What to bring in line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Action {
    /// Lower panel off if the keyboard lies on it, else on, at the
    /// brightness of the upper panel.
    Dock,
    /// Lower panel to the brightness of the upper panel.
    Brightness,
    /// Lower panel on wherever the keyboard is.
    Release,
}

/// What the command line asks for.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(name = "asus-ux8406-second-screen", version, help_template = USAGE)]
pub struct Cli {
    /// Say what would be written, write nothing.
    #[arg(short = 'n', long = "dry-run", overrides_with = "dry_run")]
    pub dry_run: bool,

    /// Use this directory in place of `/sys`.
    #[arg(
        short,
        long,
        value_name = "ROOT",
        default_value = "/sys",
        allow_hyphen_values = true,
        overrides_with = "root"
    )]
    pub root: PathBuf,

    /// What to bring in line.
    #[arg(value_enum)]
    pub action: Action,
}

impl Cli {
    /// The tree the program reads and writes.
    #[must_use]
    pub fn sysfs(&self) -> Sysfs {
        Sysfs::new(&self.root)
    }

    /// Whether to write, or only to say what would be written.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        if self.dry_run {
            Mode::DryRun
        } else {
            Mode::Apply
        }
    }
}

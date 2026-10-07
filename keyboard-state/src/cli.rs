//! The command line of asus-ux8406-keyboard-state.
//!
//! [`Cli`] is what `asus-ux8406-keyboard-state [-b DIR] [-s FILE] save | load DEVICE`
//! asks for; the help and version options are handled by the parser.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::{Bpffs, Device};

/// The text of `--help`, and of the reply to a command line that is refused.
pub const USAGE: &str = "\
usage: asus-ux8406-keyboard-state [-b DIR] [-s FILE] save | load DEVICE
       asus-ux8406-keyboard-state -h | -V
  save DEVICE       write down the backlight and the function row's mode that
                    the program of the HID device DEVICE holds
  load DEVICE       print them as udev properties for the HID device DEVICE:
                    those of another connection of the keyboard if one is
                    there, else those written down
  -b, --bpffs DIR   use DIR in place of /sys/fs/bpf (for the tests)
  -s, --state FILE  use FILE in place of /run/asus-ux8406-keyboard-state
  -h, --help        print this and exit
  -V, --version     print the version and exit";

/// What to do with the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Action {
    /// Write down the state that the program of the device holds.
    Save,
    /// Print the state for the device as udev properties.
    Load,
}

/// What the command line asks for.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(name = "asus-ux8406-keyboard-state", version, help_template = USAGE)]
pub struct Cli {
    /// Use this directory in place of `/sys/fs/bpf`.
    #[arg(
        short,
        long,
        value_name = "DIR",
        default_value = "/sys/fs/bpf",
        allow_hyphen_values = true,
        overrides_with = "bpffs"
    )]
    pub bpffs: PathBuf,

    /// Use this file in place of `/run/asus-ux8406-keyboard-state`.
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = "/run/asus-ux8406-keyboard-state",
        allow_hyphen_values = true,
        overrides_with = "state"
    )]
    pub state: PathBuf,

    /// What to do with the state.
    #[arg(value_enum)]
    pub action: Action,

    /// The HID device, by the kernel's name for it.
    #[arg(value_name = "DEVICE")]
    pub device: Device,
}

impl Cli {
    /// The file system the program looks for the pins in.
    #[must_use]
    pub fn bpffs(&self) -> Bpffs {
        Bpffs::new(&self.bpffs)
    }
}

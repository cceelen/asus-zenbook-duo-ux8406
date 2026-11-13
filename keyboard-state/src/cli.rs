//! The command line of asus-ux8406-keyboard-state.
//!
//! [`Cli`] is what
//! `asus-ux8406-keyboard-state [-b DIR] [-s FILE] [-y DIR] [-d DIR] save DEVICE | load DEVICE | sleep pre|post`
//! asks for; the help and version options are handled by the parser.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::{Bpffs, Device, Phase};

/// The text of `--help`, and of the reply to a command line that is refused.
pub const USAGE: &str = "\
usage: asus-ux8406-keyboard-state [-b DIR] [-s FILE] [-y DIR] [-d DIR]
           save DEVICE | load DEVICE | sleep pre|post
       asus-ux8406-keyboard-state -h | -V
  save DEVICE       write down the backlight and the function row's mode that
                    the program of the HID device DEVICE holds
  load DEVICE       print them as udev properties for the HID device DEVICE:
                    those of another connection of the keyboard if one is
                    there, else those written down
  sleep pre|post    before the system sleeps: switch the backlight of the
                    keyboard off; after it: set it to the level that the
                    program holds. The words of a system-sleep hook of systemd
                    (pre or post, then suspend, hibernate, hybrid-sleep or
                    suspend-then-hibernate) are taken as they are
  -b, --bpffs DIR   use DIR in place of /sys/fs/bpf (for the tests)
  -s, --state FILE  use FILE in place of /run/asus-ux8406-keyboard-state
  -y, --sysfs DIR   use DIR in place of /sys (for the tests)
  -d, --dev DIR     use DIR in place of /dev (for the tests)
  -h, --help        print this and exit
  -V, --version     print the version and exit";

/// The kind of sleep that systemd gives a hook as the second word. It does not
/// change what the helper does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    /// Suspend to memory.
    Suspend,
    /// Suspend to disk.
    Hibernate,
    /// Both at once.
    HybridSleep,
    /// Suspend, and hibernate later.
    SuspendThenHibernate,
}

/// What to do.
#[derive(Clone, Debug, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Write down the state that the program of the device holds.
    #[command(help_template = USAGE)]
    Save {
        /// The HID device, by the kernel's name for it.
        #[arg(value_name = "DEVICE")]
        device: Device,
    },
    /// Print the state for the device as udev properties.
    #[command(help_template = USAGE)]
    Load {
        /// The HID device, by the kernel's name for it.
        #[arg(value_name = "DEVICE")]
        device: Device,
    },
    /// Set the backlight for the sleep of the system.
    #[command(help_template = USAGE)]
    Sleep {
        /// Before the sleep, or after it.
        #[arg(value_enum)]
        phase: Phase,
        /// The kind of sleep. It is not used.
        #[arg(value_enum)]
        kind: Option<Kind>,
    },
}

/// What the command line asks for.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(name = "asus-ux8406-keyboard-state", version, help_template = USAGE, disable_help_subcommand = true)]
pub struct Cli {
    /// Use this directory in place of `/sys/fs/bpf`.
    #[arg(
        short,
        long,
        value_name = "DIR",
        default_value = "/sys/fs/bpf",
        allow_hyphen_values = true,
        global = true,
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
        global = true,
        overrides_with = "state"
    )]
    pub state: PathBuf,

    /// Use this directory in place of `/sys`.
    #[arg(
        short = 'y',
        long,
        value_name = "DIR",
        default_value = "/sys",
        allow_hyphen_values = true,
        global = true,
        overrides_with = "sysfs"
    )]
    pub sysfs: PathBuf,

    /// Use this directory in place of `/dev`.
    #[arg(
        short,
        long,
        value_name = "DIR",
        default_value = "/dev",
        allow_hyphen_values = true,
        global = true,
        overrides_with = "dev"
    )]
    pub dev: PathBuf,

    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// The file system the program looks for the pins in.
    #[must_use]
    pub fn bpffs(&self) -> Bpffs {
        Bpffs::new(&self.bpffs)
    }
}

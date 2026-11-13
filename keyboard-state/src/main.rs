//! The command line of asus-ux8406-keyboard-state. Needs root to read a map and
//! to open a hidraw node.
//!
//! ```text
//! usage: asus-ux8406-keyboard-state [-b DIR] [-s FILE] [-y DIR] [-d DIR]
//!            save DEVICE | load DEVICE | sleep pre|post
//!        asus-ux8406-keyboard-state -h | -V
//!   save DEVICE       write down the backlight and the function row's mode that
//!                     the program of the HID device DEVICE holds
//!   load DEVICE       print them as udev properties for the HID device DEVICE:
//!                     those of another connection of the keyboard if one is
//!                     there, else those written down
//!   sleep pre|post    before the system sleeps: switch the backlight of the
//!                     keyboard off; after it: set it to the level that the
//!                     program holds. The words of a system-sleep hook of systemd
//!                     (pre or post, then suspend, hibernate, hybrid-sleep or
//!                     suspend-then-hibernate) are taken as they are
//!   -b, --bpffs DIR   use DIR in place of /sys/fs/bpf (for the tests)
//!   -s, --state FILE  use FILE in place of /run/asus-ux8406-keyboard-state
//!   -y, --sysfs DIR   use DIR in place of /sys (for the tests)
//!   -d, --dev DIR     use DIR in place of /dev (for the tests)
//!   -h, --help        print this and exit
//!   -V, --version     print the version and exit
//! ```

use std::process::ExitCode;

use asus_ux8406_keyboard_state::cli::USAGE;
use asus_ux8406_keyboard_state::{Cli, Command, HidrawSender, PinnedMap, load, save, sleep};
use clap::Parser;
use clap::error::ErrorKind;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            // Printing can only fail with a closed stdout, which is not worth
            // a second message.
            let _ = error.print();
            return ExitCode::SUCCESS;
        }
        Err(_) => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let bpffs = cli.bpffs();
    let result = match cli.command {
        Command::Save { device } => save(&bpffs, &PinnedMap, &device, &cli.state).map(|_| ()),
        Command::Load { device } => load(&bpffs, &PinnedMap, &device, &cli.state).map(|state| {
            if let Some(state) = state {
                print!("{}", state.properties());
            }
        }),
        Command::Sleep { phase, .. } => sleep(
            &bpffs,
            &PinnedMap,
            &HidrawSender,
            &cli.sysfs,
            &cli.dev,
            phase,
        ),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("asus-ux8406-keyboard-state: {error}");
            ExitCode::FAILURE
        }
    }
}

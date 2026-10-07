//! The command line of asus-ux8406-keyboard-state. Needs root to read a map.
//!
//! ```text
//! usage: asus-ux8406-keyboard-state [-b DIR] [-s FILE] save | load DEVICE
//!        asus-ux8406-keyboard-state -h | -V
//!   save DEVICE       write down the backlight and the function row's mode that
//!                     the program of the HID device DEVICE holds
//!   load DEVICE       print them as udev properties for the HID device DEVICE:
//!                     those of another connection of the keyboard if one is
//!                     there, else those written down
//!   -b, --bpffs DIR   use DIR in place of /sys/fs/bpf (for the tests)
//!   -s, --state FILE  use FILE in place of /run/asus-ux8406-keyboard-state
//!   -h, --help        print this and exit
//!   -V, --version     print the version and exit
//! ```

use std::process::ExitCode;

use asus_ux8406_keyboard_state::cli::USAGE;
use asus_ux8406_keyboard_state::{Action, Cli, PinnedMap, load, save};
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
    let result = match cli.action {
        Action::Save => save(&bpffs, &PinnedMap, &cli.device, &cli.state).map(|_| ()),
        Action::Load => load(&bpffs, &PinnedMap, &cli.device, &cli.state).map(|state| {
            if let Some(state) = state {
                print!("{}", state.properties());
            }
        }),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("asus-ux8406-keyboard-state: {error}");
            ExitCode::FAILURE
        }
    }
}

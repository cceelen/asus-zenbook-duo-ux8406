//! The command line of asus-ux8406-second-screen. Needs root for the writes.
//!
//! ```text
//! usage: asus-ux8406-second-screen [-n] [-r ROOT] dock | brightness | release
//!        asus-ux8406-second-screen -h | -V
//!   dock             lower panel off if the keyboard lies on it, else on,
//!                    at the brightness of the upper panel
//!   brightness       lower panel to the brightness of the upper panel
//!   release          lower panel on wherever the keyboard is (for removal)
//!   -n, --dry-run    say what would be written, write nothing
//!   -r, --root ROOT  use ROOT in place of /sys (for the tests)
//!   -h, --help       print this and exit
//!   -V, --version    print the version and exit
//! ```

use std::path::Path;
use std::process::ExitCode;

use asus_ux8406_second_screen::cli::USAGE;
use asus_ux8406_second_screen::{
    Action, Cli, Detection, Error, Mode, Outcome, Sysfs, release_panel, sync_brightness, sync_panel,
};
use clap::Parser;
use clap::error::ErrorKind;

/// Say what a dry run would have written.
fn would_write(mode: Mode, text: &dyn std::fmt::Display, file: &Path) {
    if mode == Mode::DryRun {
        eprintln!(
            "asus-ux8406-second-screen: would write \"{text}\" to {}",
            file.display()
        );
    }
}

fn brightness(sysfs: &Sysfs, mode: Mode) -> Result<(), Error> {
    match sync_brightness(sysfs, mode) {
        Ok(brightness) => would_write(mode, &brightness.value, &brightness.file),
        // The compositor sets the brightness of both panels itself.
        Err(error) if error.is_taken_over() => {}
        Err(error) => return Err(error),
    }
    Ok(())
}

/// Say what a dry run would have written to the connector.
fn report(outcome: &Outcome, mode: Mode) {
    for status in &outcome.status_files {
        would_write(mode, &outcome.detection, status);
    }
}

fn release(sysfs: &Sysfs, mode: Mode) -> Result<(), Error> {
    report(&release_panel(sysfs, mode)?, mode);
    Ok(())
}

fn dock(sysfs: &Sysfs, mode: Mode) -> Result<(), Error> {
    let outcome = sync_panel(sysfs, mode)?;

    report(&outcome, mode);
    // A panel that comes back does so at the brightness it had when it left.
    if outcome.detection == Detection::Automatic {
        brightness(sysfs, mode)?;
    }
    Ok(())
}

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
    let (sysfs, mode) = (cli.sysfs(), cli.mode());
    let result = match cli.action {
        Action::Dock => dock(&sysfs, mode),
        Action::Brightness => brightness(&sysfs, mode),
        Action::Release => release(&sysfs, mode),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("asus-ux8406-second-screen: {error}");
            ExitCode::FAILURE
        }
    }
}

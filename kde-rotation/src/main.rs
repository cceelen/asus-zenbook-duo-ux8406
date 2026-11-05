//! `plasma-builtin-screen-rotation`: refer to the library for what it does.
//! The autostart entry of the package starts it in each KDE Plasma session.

use std::process::ExitCode;

use clap::Parser;
use log::error;

use asus_ux8406_kde_rotation::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();

    asus_ux8406_kde_common::daemon::start_logging();
    match asus_ux8406_kde_rotation::execute(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            error!("{failure}");
            ExitCode::FAILURE
        }
    }
}

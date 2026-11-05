//! `ux8406-kde-live-test`: refer to the library for what it does.

use std::process::ExitCode;

use clap::Parser;

use asus_ux8406_kde_live_test::cli::{Cli, execute};

fn main() -> ExitCode {
    execute(&Cli::parse())
}

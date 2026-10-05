//! `asus-ux8406-tcc-guard`: see the library for what it does.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use log::{Level, LevelFilter, error};

use asus_ux8406_tcc_guard::cli::Cli;

/// The syslog priorities of the levels, from `Error` to `Trace`, as the
/// journal reads them at the start of a line.
const PRIORITY: [&str; 5] = ["<3>", "<4>", "<6>", "<7>", "<7>"];

/// One line per message. Under systemd each carries its priority the way
/// the journal reads it; the journal adds the time.
fn start_logging() {
    let journal = std::env::var_os("JOURNAL_STREAM").is_some();

    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .format(move |out, record| {
            let priority = if journal {
                PRIORITY[record.level() as usize - Level::Error as usize]
            } else {
                ""
            };

            writeln!(out, "{priority}{}", record.args())
        })
        .init();
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    #[cfg(feature = "failpoints")]
    let _faults = fail::FailScenario::setup();

    start_logging();
    match asus_ux8406_tcc_guard::stop_on_signals()
        .and_then(|stop| asus_ux8406_tcc_guard::execute(&cli, &stop))
    {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(failure) => {
            error!("{failure}");
            ExitCode::FAILURE
        }
    }
}

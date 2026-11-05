//! The command line, and the whole run on the machine.

use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

use crate::console::{Console, Terminal};
use crate::context::{Ctx, Timing};
use crate::report::Report;
use crate::runner::System;
use crate::{live, run};

/// What the command line asks for.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "ux8406-kde-live-test",
    version,
    about = "Tests the KDE packages in the Fedora KDE live session on the ASUS Zenbook Duo UX8406"
)]
pub struct Cli {
    /// The directory with `rpms/`; `results/` is made in it. Default: the
    /// directory of this program.
    #[arg(short, long, value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Give up an action of the tester after this many seconds. Without
    /// this option the tool waits until the action is done or the tester
    /// types `s` and Enter.
    #[arg(short, long, value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Also test the pen on each screen. Without this option the tool asks
    /// for no action with a pen.
    #[arg(long)]
    pub pen: bool,

    /// Do not let sudo ask for a password on the terminal. For the tests of
    /// the tool.
    #[arg(long, hide = true)]
    pub no_prompt: bool,

    /// Use this directory in place of the root file system.
    #[arg(short, long, value_name = "DIR", default_value = "/", hide = true)]
    pub root: PathBuf,
}

/// The directory of the running program; the current directory if the
/// system does not say.
fn own_directory() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_default()
}

/// Run the test; the exit code is 0 if every check that ran was as
/// expected, 1 if a check failed, 2 if the test could not run.
#[must_use]
pub fn execute(cli: &Cli) -> ExitCode {
    let terminal = Terminal::new(
        Box::new(BufReader::new(std::io::stdin())),
        Box::new(std::io::stdout()),
    );

    ExitCode::from(execute_with(cli, &terminal))
}

/// [`execute`] with a console of its own.
#[must_use]
pub fn execute_with(cli: &Cli, console: &dyn Console) -> u8 {
    if let Err(reason) = live::check(&cli.root) {
        console.say(&format!(
            "Stop: this is not a live session ({reason}). The tool never changes an installed system."
        ));
        return 2;
    }

    let dir = cli.dir.clone().unwrap_or_else(own_directory);
    let results = dir.join("results");
    let report = match Report::open(&results, "KDE packages on the UX8406: live test") {
        Ok(report) => report,
        Err(error) => {
            console.say(&format!("Stop: cannot make {}: {error}", results.display()));
            return 2;
        }
    };
    let mut ctx = Ctx {
        root: cli.root.clone(),
        dir,
        vars: std::env::vars().collect::<BTreeMap<_, _>>(),
        runner: &System,
        console,
        prompt: !cli.no_prompt,
        pen: cli.pen,
        timing: Timing {
            guided: cli.timeout.map(Duration::from_secs),
            ..Timing::default()
        },
        report,
    };

    match run(&mut ctx) {
        Ok(counts) => u8::from(counts.fail > 0),
        Err(error) => {
            console.say(&format!("Stop: {error}"));
            2
        }
    }
}

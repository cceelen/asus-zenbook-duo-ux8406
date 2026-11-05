//! Commands and background programs. The checks go through the trait
//! [`Runner`], so that the tests give canned answers.

use std::fs::File;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// What a command did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// The exit code; none if the command could not run or a signal ended it.
    pub code: Option<i32>,
    /// Standard output.
    pub stdout: String,
    /// Standard error; for a command that could not run, the reason.
    pub stderr: String,
}

impl Output {
    /// A command that ended with this code and wrote this to standard output.
    #[must_use]
    pub fn new(code: i32, stdout: &str) -> Self {
        Self {
            code: Some(code),
            stdout: stdout.to_owned(),
            stderr: String::new(),
        }
    }

    /// A command that ended with code 0.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.code, Some(0))
    }

    /// Standard output and standard error as one text.
    #[must_use]
    pub fn text(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// How the tool runs programs. A program can run on a thread of its own, so
/// the runner is shared between threads.
pub trait Runner: Sync {
    /// Run a program and wait for its end.
    fn run(&self, program: &str, args: &[&str]) -> Output;

    /// Run a program that uses the terminal of the tester (standard input,
    /// output and error) and wait for its end. The output has the code and,
    /// for a program that could not run, the reason; the streams are empty.
    fn run_attached(&self, program: &str, args: &[&str]) -> Output;

    /// Start a program that goes on after this tool, with its output in the
    /// file `log`. The result is its process number.
    ///
    /// # Errors
    ///
    /// The reason why the program did not start.
    fn start(&self, program: &str, args: &[&str], log: &Path) -> Result<u32, String>;
}

/// The programs of this machine.
#[derive(Clone, Copy, Debug, Default)]
pub struct System;

impl Runner for System {
    fn run(&self, program: &str, args: &[&str]) -> Output {
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_or_else(
                |error| Output {
                    code: None,
                    stdout: String::new(),
                    stderr: format!("{program}: {error}\n"),
                },
                |output| Output {
                    code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                },
            )
    }

    fn run_attached(&self, program: &str, args: &[&str]) -> Output {
        Command::new(program).args(args).status().map_or_else(
            |error| Output {
                code: None,
                stdout: String::new(),
                stderr: format!("{program}: {error}\n"),
            },
            |status| Output {
                code: status.code(),
                ..Output::default()
            },
        )
    }

    fn start(&self, program: &str, args: &[&str], log: &Path) -> Result<u32, String> {
        File::create(log)
            .and_then(|out| out.try_clone().map(|err| (out, err)))
            .and_then(|(out, err)| {
                // A group of its own: Ctrl+C in the terminal does not end it.
                Command::new(program)
                    .args(args)
                    .stdin(Stdio::null())
                    .stdout(out)
                    .stderr(err)
                    .process_group(0)
                    .spawn()
            })
            .map(|child| child.id())
            .map_err(|error| format!("{program}: {error}"))
    }
}

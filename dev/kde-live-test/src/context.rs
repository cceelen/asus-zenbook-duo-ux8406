//! What every check shares: the files of the machine, the session, the
//! programs, the tester and the report.

use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use asus_ux8406_kde_common::output::Session;
use wayland_client::Connection;
use zbus::blocking::connection::Builder;

use crate::console::Console;
use crate::report::{Kind, Report, Status};
use crate::runner::{Output, Runner};

/// How long the tool waits.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// For an action of the tester; none: as long as the tester needs.
    pub guided: Option<Duration>,
    /// For the machine to settle after a change.
    pub settle: Duration,
    /// For a thing that must not happen.
    pub quiet: Duration,
    /// Between two renewals of the rights of sudo.
    pub keepalive: Duration,
    /// Between two looks.
    pub poll: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            guided: None,
            settle: Duration::from_secs(10),
            quiet: Duration::from_secs(4),
            keepalive: Duration::from_secs(60),
            poll: Duration::from_millis(100),
        }
    }
}

/// The context of a run.
pub struct Ctx<'a> {
    /// The root of the file system: `/`. A test gives another one.
    pub root: PathBuf,
    /// The directory of the tool: `rpms/` and `results/` are in it.
    pub dir: PathBuf,
    /// The environment of the session.
    pub vars: BTreeMap<String, String>,
    /// How programs run.
    pub runner: &'a dyn Runner,
    /// The tester.
    pub console: &'a dyn Console,
    /// Whether sudo may ask the tester for a password on the terminal.
    pub prompt: bool,
    /// Whether the tester has a pen: without one, the tool asks for no
    /// action with it.
    pub pen: bool,
    /// The waits.
    pub timing: Timing,
    /// The results.
    pub report: Report,
}

impl Ctx<'_> {
    /// The path of an absolute path below the root.
    #[must_use]
    pub fn path(&self, absolute: &str) -> PathBuf {
        self.root.join(absolute.trim_start_matches('/'))
    }

    /// The text of a file below the root; none if it cannot be read.
    #[must_use]
    pub fn read(&self, absolute: &str) -> Option<String> {
        std::fs::read_to_string(self.path(absolute)).ok()
    }

    /// The names in a directory below the root, sorted.
    #[must_use]
    pub fn list(&self, absolute: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.path(absolute))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();

        names.sort();
        names
    }

    /// A variable of the session.
    #[must_use]
    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// Say a line to the tester.
    pub fn say(&self, text: &str) {
        self.console.say(text);
    }

    /// Run a program.
    #[must_use]
    pub fn run(&self, program: &str, args: &[&str]) -> Output {
        self.runner.run(program, args)
    }

    /// Run a program with the tester's terminal.
    #[must_use]
    pub fn run_attached(&self, program: &str, args: &[&str]) -> Output {
        self.runner.run_attached(program, args)
    }

    /// Run a program with the rights of root. The call never asks for a
    /// password (`sudo -n`): the preflight asks once with `sudo -v`, and
    /// [`crate::keepalive::keep`] renews the rights until the run ends.
    #[must_use]
    pub fn sudo(&self, program: &str, args: &[&str]) -> Output {
        let mut all = vec!["-n", program];

        all.extend_from_slice(args);
        self.runner.run("sudo", &all)
    }

    /// The first directory of `PATH` that has this program.
    #[must_use]
    pub fn which(&self, program: &str) -> Option<PathBuf> {
        self.var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(|dir| Path::new(dir).join(program))
            .find(|path| path.is_file())
    }

    /// One row in the report, and a line for the tester.
    pub fn record(&mut self, check: &str, kind: Kind, status: Status, detail: &str) {
        let short: String = detail.chars().take(200).collect();

        self.report.add(check, kind, status, detail);
        self.console.say(&format!("[{status}] {check}: {short}"));
    }

    /// One row: OK with this detail, or FAIL with the reason.
    pub fn check(&mut self, check: &str, kind: Kind, result: Result<String, String>) {
        match result {
            Ok(detail) => self.record(check, kind, Status::Ok, &detail),
            Err(reason) => self.record(check, kind, Status::Fail, &reason),
        }
    }

    /// An automatic check that ends with this result.
    pub fn auto(&mut self, check: &str, result: Result<String, String>) {
        self.check(check, Kind::Auto, result);
    }

    /// A raw log.
    pub fn raw(&mut self, name: &str, text: &str) {
        self.report.raw(name, text);
    }

    /// The address of the session bus.
    #[must_use]
    pub fn session_address(&self) -> String {
        self.var("DBUS_SESSION_BUS_ADDRESS").map_or_else(
            || {
                format!(
                    "unix:path={}/bus",
                    self.var("XDG_RUNTIME_DIR").unwrap_or("/run/user/1000")
                )
            },
            str::to_owned,
        )
    }

    /// The address of the system bus.
    #[must_use]
    pub fn system_address(&self) -> String {
        self.var("DBUS_SYSTEM_BUS_ADDRESS").map_or_else(
            || "unix:path=/run/dbus/system_bus_socket".to_owned(),
            str::to_owned,
        )
    }

    /// A connection to the session bus.
    ///
    /// # Errors
    ///
    /// Why the bus is not there.
    pub fn session_bus(&self) -> Result<zbus::blocking::Connection, String> {
        connect_bus(&self.session_address()).map_err(|error| format!("session bus: {error}"))
    }

    /// A connection to the system bus.
    ///
    /// # Errors
    ///
    /// Why the bus is not there.
    pub fn system_bus(&self) -> Result<zbus::blocking::Connection, String> {
        connect_bus(&self.system_address()).map_err(|error| format!("system bus: {error}"))
    }

    /// The socket of the Wayland compositor of the session.
    #[must_use]
    pub fn wayland_socket(&self) -> Option<PathBuf> {
        self.var("WAYLAND_DISPLAY").map(|display| {
            Path::new(self.var("XDG_RUNTIME_DIR").unwrap_or("/run/user/1000")).join(display)
        })
    }

    /// A raw connection to KWin.
    ///
    /// # Errors
    ///
    /// Why there is none.
    pub fn wayland(&self) -> Result<Connection, String> {
        let socket = self
            .wayland_socket()
            .ok_or_else(|| "WAYLAND_DISPLAY is not set".to_owned())?;

        UnixStream::connect(&socket)
            .map_err(text)
            .and_then(|stream| Connection::from_socket(stream).map_err(text))
            .map_err(|error| format!("Wayland socket {}: {error}", socket.display()))
    }

    /// KWin's outputs.
    ///
    /// # Errors
    ///
    /// Why they cannot be followed.
    pub fn outputs(&self) -> Result<Session, String> {
        self.wayland()
            .and_then(|connection| Session::with(connection).map_err(text))
    }
}

fn connect_bus(address: &str) -> zbus::Result<zbus::blocking::Connection> {
    Builder::address(address).and_then(Builder::build)
}

/// An error as text.
fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

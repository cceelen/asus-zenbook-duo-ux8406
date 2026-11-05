//! A program under test, run as a child process: its log lines as they
//! come, and its end at SIGTERM.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process};

use crate::{Bus, Compositor};

/// A running program.
pub struct Program {
    child: Child,
    log: Arc<Mutex<Vec<String>>>,
    /// Whether the program closed its standard error: it has ended.
    ended: Arc<AtomicBool>,
}

/// How long the tests wait for a program.
const PATIENCE: Duration = Duration::from_secs(10);

/// The command for a program with the environment of a session: the
/// compositor and the bus (as session and as system bus). Nothing of the
/// session that runs the tests reaches it.
#[must_use]
pub fn command(program: &str, compositor: Option<&Compositor>, bus: Option<&Bus>) -> Command {
    let mut command = Command::new(program);

    for name in [
        "WAYLAND_DISPLAY",
        "WAYLAND_SOCKET",
        "DBUS_SESSION_BUS_ADDRESS",
        "DBUS_SYSTEM_BUS_ADDRESS",
        "JOURNAL_STREAM",
        "XDG_CONFIG_HOME",
        "RUST_LOG",
    ] {
        command.env_remove(name);
    }
    // No bus: an address where there is none.
    command.env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    command.env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    if let Some(compositor) = compositor {
        command.env("WAYLAND_DISPLAY", compositor.socket());
    }
    if let Some(bus) = bus {
        command.env("DBUS_SESSION_BUS_ADDRESS", bus.address());
        command.env("DBUS_SYSTEM_BUS_ADDRESS", bus.address());
    }
    command
}

impl Program {
    /// Start the command, with its standard error read line by line.
    ///
    /// # Panics
    ///
    /// If it cannot be started.
    #[must_use]
    pub fn start(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the program");
        let log = Arc::new(Mutex::new(Vec::new()));
        let lines = BufReader::new(child.stderr.take().expect("stderr"));
        let shared = Arc::clone(&log);
        let ended = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&ended);

        std::thread::spawn(move || {
            for line in lines.lines().map_while(Result::ok) {
                shared
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(line);
            }
            signal.store(true, Ordering::SeqCst);
        });
        Self { child, log, ended }
    }

    /// The lines logged so far.
    #[must_use]
    pub fn log(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Whether a line logged so far contains this text.
    #[must_use]
    pub fn logged(&self, text: &str) -> bool {
        self.log().iter().any(|line| line.contains(text))
    }

    /// Wait until a line contains this text.
    ///
    /// # Panics
    ///
    /// If none does in time.
    pub fn wait_log(&self, text: &str) {
        self.wait_for(&format!("a line with {text:?}"), || self.logged(text));
    }

    /// Wait until `done` is true.
    ///
    /// # Panics
    ///
    /// If it is not in time, or the program ends first; the panic shows the
    /// log.
    pub fn wait_for(&self, what: &str, done: impl Fn() -> bool) {
        let end = Instant::now() + PATIENCE;

        loop {
            // Read before `done`: what the program did before it ended counts.
            let gone = self.ended.load(Ordering::SeqCst);

            if done() {
                return;
            }
            assert!(
                !gone,
                "the program ended, and {what} did not come; log: {:#?}",
                self.log()
            );
            assert!(
                Instant::now() < end,
                "waited in vain for {what}; log: {:#?}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Wait until the program ends by itself.
    ///
    /// # Panics
    ///
    /// If it does not end in time.
    #[must_use]
    pub fn end(mut self) -> (ExitStatus, Vec<String>) {
        let end = Instant::now() + PATIENCE;

        loop {
            if let Some(status) = self.child.try_wait().expect("status") {
                // The last lines come after the end.
                std::thread::sleep(Duration::from_millis(50));
                return (status, self.log());
            }
            if Instant::now() > end {
                let _ = self.child.kill();
                panic!("the program did not end; log: {:#?}", self.log());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Send SIGTERM, as at the end of the session, and wait for the end.
    ///
    /// # Panics
    ///
    /// If the signal cannot be sent or the program does not end.
    #[must_use]
    pub fn stop(self) -> (ExitStatus, Vec<String>) {
        let pid = Pid::from_child(&self.child);

        kill_process(pid, Signal::TERM).expect("SIGTERM");
        self.end()
    }
}

impl Drop for Program {
    /// SIGTERM, so that the program ends as it does at the end of a
    /// session (and writes its coverage data); SIGKILL after a while.
    fn drop(&mut self) {
        let end = Instant::now() + PATIENCE;

        let _ = kill_process(Pid::from_child(&self.child), Signal::TERM);
        while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

//! The machine as it behaves while the program runs on it.

use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process};

use crate::common::{
    self, CONFIG, HWMON, KMSG, Machine, OFFSET, OFFSET_TWO, STATE, TEMP, TEMP_TWO,
};

const TICK: Duration = Duration::from_millis(50);
const TIMEOUT: Duration = Duration::from_secs(60);
/// Where a coverage build of the program writes what it ran.
const PROFILE: &str = "LLVM_PROFILE_FILE";

/// The last line the program says before it starts to watch: its log is
/// open by then and its packages are checked.
const READY: &str = "event text: ";
pub const EVENT_RECORD: &str = "6,1,1,-;test: thermal warning\n";
pub const HOLDING: &str = "alert cleared: holding offset";
pub const LOWERED: &str = "no event: offset lowered to";
pub const RAISED: &str = "offset raised to";
pub const PUT_BACK: &str = "package 0: a guard was killed while it held an offset: back to 10";
/// What a refused command line ends with, and what any other refusal.
pub const USAGE: i32 = 2;
pub const FAILURE: i32 = 1;

const OFFSETS: [&str; 2] = [OFFSET, OFFSET_TWO];
const TEMPS: [&str; 2] = [TEMP, TEMP_TWO];

/// What a step of a test does.
#[derive(Clone, Copy)]
pub enum Kind {
    /// Append this to the kernel log.
    Record(&'static str),
    /// The load on the package ends.
    Unload(usize),
    /// The temperature of the package becomes unreadable.
    BreakSensor(usize),
    /// The temperature file of the package disappears.
    RemoveSensor(usize),
    /// The offset of the package can no longer be written.
    LockOffset(usize),
    /// Something else writes this as the offset of the package.
    SetOffset(usize, i32),
    /// The offset of the package must be this.
    ExpectOffset(usize, i32),
    /// The offset of the package must be above this.
    ExpectRaised(usize, i32),
    /// The program is sent this signal.
    Signal(Signal),
}

/// One step of a test: `delay_ms` after the log first contains `after`, or,
/// if that is `None`, after the program has said that it is at work. Not
/// after it was started: on a busy machine that can be seconds before it
/// looks at anything, and a record written to the log before then is one
/// it rightly never sees.
#[derive(Clone, Copy)]
pub struct Step {
    after: Option<&'static str>,
    delay_ms: u64,
    kind: Kind,
}

pub const fn step(after: Option<&'static str>, delay_ms: u64, kind: Kind) -> Step {
    Step {
        after,
        delay_ms,
        kind,
    }
}

/// When a step of a running test is due.
#[derive(Clone, Copy, PartialEq)]
enum Due {
    Waiting,
    At(Duration),
    Done,
}

/// A machine and what its processors do.
pub struct World {
    pub machine: Machine,
    /// At the limit its offset allows; else 60 C.
    loaded: [bool; 2],
    /// The temperature sensor returns rubbish.
    broken: [bool; 2],
    /// The temperature sensor has disappeared.
    gone: [bool; 2],
    /// The files stay as the test has put them.
    pub frozen: bool,
    /// What the program is told about faults to inject.
    pub faults: Option<&'static str>,
    /// The log, as last read.
    pub text: String,
}

impl World {
    /// The UX8406CA of the fixtures, under load.
    pub fn new() -> Self {
        Self::on(common::machine())
    }

    /// The same with a second package.
    pub fn two_packages() -> Self {
        Self::on(common::two_packages())
    }

    fn on(machine: Machine) -> Self {
        common::ordinary_user();
        machine.put("log", "");
        Self {
            machine,
            loaded: [true; 2],
            broken: [false; 2],
            gone: [false; 2],
            frozen: false,
            faults: None,
            text: String::new(),
        }
    }

    pub fn offset(&self, package: usize) -> Option<i32> {
        self.machine.number(OFFSETS[package])
    }

    /// The offsets the program has noted as changed: file names and values.
    pub fn notes(&self) -> Vec<(String, Option<i32>)> {
        let Ok(entries) = fs::read_dir(self.machine.join(STATE)) else {
            return Vec::new();
        };
        let mut notes: Vec<_> = entries
            .map(|entry| {
                let name = entry.unwrap().file_name().into_string().unwrap();
                let value = self.machine.number(&format!("{STATE}/{name}"));

                (name, value)
            })
            .collect();

        notes.sort();
        notes
    }

    /// The path of something on the machine, as text for a command line.
    pub fn path(&self, relative: &str) -> String {
        self.machine.join(relative).to_str().unwrap().to_owned()
    }

    /// What the processors do: a loaded package runs at the limit its
    /// offset allows, an idle one at 60 C.
    fn plant(&self) {
        if self.frozen {
            return;
        }
        for (package, temp) in TEMPS.into_iter().enumerate() {
            if self.gone[package] {
                let _ = fs::remove_file(self.machine.join(temp));
            } else if self.broken[package] {
                self.machine.put(temp, "rubbish\n");
            } else if let Some(offset) = self.offset(package) {
                let degrees = if self.loaded[package] {
                    105 - offset
                } else {
                    60
                };

                self.machine.put(temp, format!("{}\n", degrees * 1000));
            }
        }
    }

    /// Start the program on the machine with its output in the log file.
    fn spawn(&self, arguments: &[&str]) -> Child {
        let log = OpenOptions::new()
            .append(true)
            .open(self.machine.join("log"))
            .unwrap();

        let mut program = Command::new(assert_cmd::cargo::cargo_bin!("asus-ux8406-tcc-guard"));

        // On the machine, unless the test names another way to it.
        if !arguments.contains(&"--root") {
            program.arg("--root").arg(self.machine.root());
        }
        program
            .args(arguments)
            .env_clear()
            .envs(std::env::var_os(PROFILE).map(|file| (PROFILE, file)))
            .envs(self.faults.map(|faults| ("FAILPOINTS", faults)))
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap()
    }

    fn act(&mut self, child: &Child, kind: Kind) {
        match kind {
            Kind::Record(text) => self.machine.append(KMSG, text),
            Kind::Unload(package) => self.loaded[package] = false,
            Kind::BreakSensor(package) => self.broken[package] = true,
            Kind::RemoveSensor(package) => self.gone[package] = true,
            Kind::LockOffset(package) => self.machine.chmod(OFFSETS[package], 0o400),
            Kind::SetOffset(package, value) => {
                self.machine.put(OFFSETS[package], format!("{value}\n"));
            }
            Kind::ExpectOffset(package, value) => {
                assert_eq!(self.offset(package), Some(value), "{}", self.text);
            }
            Kind::ExpectRaised(package, value) => {
                assert!(
                    self.offset(package).is_some_and(|offset| offset > value),
                    "offset of package {package} is {:?}, expected more than {value}\n{}",
                    self.offset(package),
                    self.text
                );
            }
            Kind::Signal(signal) => kill_process(Pid::from_child(child), signal).unwrap(),
        }
    }

    /// Run the program and play the machine until the program ends; its exit
    /// status, or `None` if a signal ended it.
    pub fn play(&mut self, steps: &[Step], arguments: &[&str]) -> Option<i32> {
        let mut child = self.spawn(arguments);
        let mut due = vec![Due::Waiting; steps.len()];
        let start = Instant::now();

        loop {
            let ended = child.try_wait().unwrap();
            let now = start.elapsed();

            self.text = fs::read_to_string(self.machine.join("log")).unwrap();
            if let Some(status) = ended {
                assert!(
                    due.iter().all(|due| *due == Due::Done),
                    "the program ended before the test did\n{}",
                    self.text
                );
                return status.code();
            }
            if now > TIMEOUT {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("the program did not end\n{}", self.text);
            }
            for (step, due) in steps.iter().zip(&mut due) {
                if *due == Due::Waiting && self.text.contains(step.after.unwrap_or(READY)) {
                    *due = Due::At(now + Duration::from_millis(step.delay_ms));
                }
                if matches!(*due, Due::At(at) if now >= at) {
                    self.act(&child, step.kind);
                    *due = Due::Done;
                }
            }
            self.plant();
            thread::sleep(TICK);
        }
    }

    /// Run the program to its end, with a log of this run only.
    pub fn run(&mut self, arguments: &[&str]) -> Option<i32> {
        self.machine.put("log", "");
        self.play(&[], arguments)
    }

    pub fn expect_log(&self, text: &str) {
        assert!(
            self.text.contains(text),
            "the log lacks {text:?}\n{}",
            self.text
        );
    }

    pub fn expect_no_log(&self, text: &str) {
        assert!(
            !self.text.contains(text),
            "the log has {text:?}\n{}",
            self.text
        );
    }

    /// The program must refuse to start, and leave the offset alone.
    pub fn expect_refusal(&mut self, what: &str, arguments: &[&str]) {
        self.expect_refusal_with(FAILURE, what, arguments);
    }

    pub fn expect_refusal_with(&mut self, status: i32, what: &str, arguments: &[&str]) {
        self.frozen = true;
        assert_eq!(
            self.run(arguments),
            Some(status),
            "not refused: {what}\n{}",
            self.text
        );
        self.expect_no_log("armed");
        assert_eq!(self.offset(0), Some(10), "{what}");
    }

    /// The opposite, after a refusal has been undone: the program must
    /// start, so that the next refusal is known to have its own cause.
    pub fn expect_start(&mut self, after: &str) {
        self.frozen = true;
        assert_eq!(
            self.run(&["-t", "1"]),
            Some(0),
            "does not start {after}\n{}",
            self.text
        );
        self.expect_log("armed");
    }
}

/// The configuration of the fixture, as it was.
pub fn usual_config() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ux8406ca")
            .join(CONFIG),
    )
    .unwrap()
}

/// An attribute of the hwmon device of the first package.
pub fn hwmon(attribute: &str) -> String {
    format!("{HWMON}/{attribute}")
}

//! What the tests share: canned programs, a scripted tester, a root file
//! system in a temporary directory, and the fakes of KWin's services.
#![allow(dead_code, reason = "each test binary uses a part of it")]

pub mod services;

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use asus_ux8406_kde_live_test::console::Console;
use asus_ux8406_kde_live_test::context::{Ctx, Timing};
use asus_ux8406_kde_live_test::report::Report;
use asus_ux8406_kde_live_test::runner::{Output, Runner};
use tempfile::TempDir;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Programs that answer from a table. The longest rule that starts the
/// command line (program and arguments, joined by spaces) answers; a command
/// without a rule succeeds and says nothing.
#[derive(Default)]
pub struct FakeRunner {
    rules: Mutex<Vec<(String, Output)>>,
    sequences: Mutex<Vec<(String, VecDeque<Output>)>>,
    calls: Mutex<Vec<String>>,
    starts: Mutex<Vec<String>>,
    logs: Mutex<Vec<(String, String)>>,
    refused: Mutex<Vec<String>>,
    hook: Mutex<Option<Hook>>,
}

/// Called with each command line that runs or starts, to give it an effect.
pub type Hook = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

impl FakeRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer commands that start with `prefix`.
    pub fn rule(&self, prefix: &str, code: i32, stdout: &str) -> &Self {
        lock(&self.rules).push((prefix.to_owned(), Output::new(code, stdout)));
        self
    }

    /// Answer commands that start with `prefix` with these codes, one for each
    /// call, before the rules answer.
    pub fn sequence(&self, prefix: &str, codes: &[i32]) -> &Self {
        lock(&self.sequences).push((
            prefix.to_owned(),
            codes.iter().map(|code| Output::new(*code, "")).collect(),
        ));
        self
    }

    /// Answer commands that start with `prefix` with these codes and texts,
    /// one for each call, before the rules answer.
    pub fn sequence_texts(&self, prefix: &str, answers: &[(i32, &str)]) -> &Self {
        lock(&self.sequences).push((
            prefix.to_owned(),
            answers
                .iter()
                .map(|(code, text)| Output::new(*code, text))
                .collect(),
        ));
        self
    }

    /// Answer with a failure to run.
    pub fn missing(&self, prefix: &str) -> &Self {
        lock(&self.rules).push((
            prefix.to_owned(),
            Output {
                code: None,
                stdout: String::new(),
                stderr: "not found".to_owned(),
            },
        ));
        self
    }

    /// Write this to the log of a started program whose command line
    /// starts with `prefix`.
    pub fn log(&self, prefix: &str, text: &str) -> &Self {
        lock(&self.logs).push((prefix.to_owned(), text.to_owned()));
        self
    }

    /// Refuse to start programs whose command line starts with `prefix`.
    pub fn refuse(&self, prefix: &str) -> &Self {
        lock(&self.refused).push(prefix.to_owned());
        self
    }

    /// Call this for each command, after it is recorded.
    pub fn on_command(&self, hook: impl Fn(&str) + Send + Sync + 'static) -> &Self {
        *lock(&self.hook) = Some(std::sync::Arc::new(hook));
        self
    }

    fn hooked(&self, line: &str) {
        let hook = lock(&self.hook).clone();

        if let Some(hook) = hook {
            hook(line);
        }
    }

    pub fn calls(&self) -> Vec<String> {
        lock(&self.calls).clone()
    }

    pub fn called(&self, prefix: &str) -> bool {
        self.calls().iter().any(|call| call.starts_with(prefix))
    }

    pub fn starts(&self) -> Vec<String> {
        lock(&self.starts).clone()
    }

    /// The process numbers of the started programs that start with `prefix`.
    pub fn pids(&self, prefix: &str) -> Vec<u32> {
        self.starts()
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with(prefix))
            .map(|(index, _)| 1000 + u32::try_from(index).expect("small"))
            .collect()
    }
}

fn line(program: &str, args: &[&str]) -> String {
    std::iter::once(program)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}

impl FakeRunner {
    fn answer(&self, line: &str) -> Output {
        lock(&self.calls).push(line.to_owned());
        self.hooked(line);
        if let Some(output) = lock(&self.sequences)
            .iter_mut()
            .find(|(prefix, answers)| line.starts_with(prefix.as_str()) && !answers.is_empty())
            .and_then(|(_, answers)| answers.pop_front())
        {
            return output;
        }
        lock(&self.rules)
            .iter()
            .filter(|(prefix, _)| line.starts_with(prefix.as_str()))
            .max_by_key(|(prefix, _)| prefix.len())
            .map_or_else(|| Output::new(0, ""), |(_, output)| output.clone())
    }
}

impl Runner for FakeRunner {
    fn run(&self, program: &str, args: &[&str]) -> Output {
        self.answer(&line(program, args))
    }

    fn run_attached(&self, program: &str, args: &[&str]) -> Output {
        self.answer(&line(program, args))
    }

    fn start(&self, program: &str, args: &[&str], log: &Path) -> Result<u32, String> {
        let line = line(program, args);

        if lock(&self.refused)
            .iter()
            .any(|prefix| line.starts_with(prefix.as_str()))
        {
            return Err(format!("{program}: cannot start"));
        }

        let text = lock(&self.logs)
            .iter()
            .filter(|(prefix, _)| line.starts_with(prefix.as_str()))
            .map(|(_, text)| text.clone())
            .collect::<String>();
        std::fs::write(log, text).map_err(|error| error.to_string())?;

        let number = {
            let mut starts = lock(&self.starts);

            starts.push(line.clone());
            starts.len() - 1
        };

        self.hooked(&line);
        Ok(1000 + u32::try_from(number).expect("small"))
    }
}

/// A tester that answers from a script.
#[derive(Default)]
pub struct FakeConsole {
    said: Mutex<Vec<String>>,
    answers: Mutex<VecDeque<bool>>,
    skip_at: Mutex<Vec<String>>,
    skipping: AtomicBool,
}

impl FakeConsole {
    pub fn new() -> Self {
        Self::default()
    }

    /// The answers to the next questions; no more answers means yes.
    pub fn answer(&self, answers: &[bool]) -> &Self {
        lock(&self.answers).extend(answers.iter().copied());
        self
    }

    /// Skip (type `s`) once a line says this.
    pub fn skip_at(&self, text: &str) -> &Self {
        lock(&self.skip_at).push(text.to_owned());
        self
    }

    pub fn said(&self) -> Vec<String> {
        lock(&self.said).clone()
    }

    pub fn has_said(&self, text: &str) -> bool {
        self.said().iter().any(|line| line.contains(text))
    }

    /// Wait until `times` lines say this: for a thread that plays the
    /// tester's hands.
    pub fn wait_said(&self, text: &str, times: usize) {
        let end = Instant::now() + Duration::from_secs(20);

        while self
            .said()
            .iter()
            .filter(|line| line.contains(text))
            .count()
            < times
        {
            assert!(
                Instant::now() < end,
                "never said {text:?}: {:#?}",
                self.said()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Console for FakeConsole {
    fn say(&self, text: &str) {
        lock(&self.said).push(text.to_owned());
        if lock(&self.skip_at)
            .iter()
            .any(|skip| text.contains(skip.as_str()))
        {
            self.skipping.store(true, Ordering::SeqCst);
        }
    }

    fn confirm(&self, question: &str) -> bool {
        self.say(question);
        lock(&self.answers).pop_front().unwrap_or(true)
    }

    fn skip_requested(&self) -> bool {
        self.skipping.swap(false, Ordering::SeqCst)
    }
}

/// The directories of a run: the root file system, the directory of the
/// tool, the home of the user.
pub struct Fixture {
    pub root: TempDir,
    pub dir: TempDir,
    pub home: TempDir,
    pub runner: Arc<FakeRunner>,
    pub console: FakeConsole,
    pub vars: BTreeMap<String, String>,
}

pub const FAST: Timing = Timing {
    guided: Some(Duration::from_secs(2)),
    keepalive: Duration::from_millis(20),
    settle: Duration::from_secs(1),
    quiet: Duration::from_millis(300),
    poll: Duration::from_millis(5),
};

impl Fixture {
    pub fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().expect("root"),
            dir: tempfile::tempdir().expect("dir"),
            home: tempfile::tempdir().expect("home"),
            runner: Arc::new(FakeRunner::new()),
            console: FakeConsole::new(),
            // No bus: a test never reaches the session of the machine.
            vars: BTreeMap::from([
                (
                    "DBUS_SESSION_BUS_ADDRESS".to_owned(),
                    "unix:path=/nonexistent/bus".to_owned(),
                ),
                (
                    "DBUS_SYSTEM_BUS_ADDRESS".to_owned(),
                    "unix:path=/nonexistent/bus".to_owned(),
                ),
            ]),
        };

        std::fs::create_dir_all(fixture.dir.path().join("rpms")).expect("rpms");
        fixture
    }

    /// A file below the root.
    pub fn file(&self, absolute: &str, text: &str) -> PathBuf {
        let path = self.root.path().join(absolute.trim_start_matches('/'));

        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, text).expect("file");
        path
    }

    /// A file of the home directory.
    pub fn home_file(&self, relative: &str, text: &str) {
        let path = self.home.path().join(relative);

        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, text).expect("file");
    }

    pub fn var(mut self, name: &str, value: &str) -> Self {
        self.vars.insert(name.to_owned(), value.to_owned());
        self
    }

    /// The context of a run, with a fresh report.
    pub fn ctx(&self) -> Ctx<'_> {
        let mut vars = self.vars.clone();

        vars.entry("HOME".to_owned())
            .or_insert_with(|| self.home.path().display().to_string());
        Ctx {
            root: self.root.path().to_owned(),
            dir: self.dir.path().to_owned(),
            vars,
            runner: self.runner.as_ref(),
            console: &self.console,
            prompt: true,
            pen: true,
            timing: FAST,
            report: Report::open(&self.dir.path().join("results"), "Test").expect("report"),
        }
    }
}

/// The rows of the report as (check, mode, result, detail).
pub fn rows(ctx: &Ctx<'_>) -> Vec<(String, String, String, String)> {
    let text = std::fs::read_to_string(ctx.report.dir().join("report.md")).unwrap_or_default();

    text.lines()
        .filter(|line| {
            line.starts_with("| ") && !line.starts_with("| Check") && !line.starts_with("| ---")
        })
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();

            (
                cells[0].to_owned(),
                cells[1].to_owned(),
                cells[2].to_owned(),
                cells.get(3).copied().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The result of the check with this name.
pub fn result(ctx: &Ctx<'_>, check: &str) -> String {
    rows(ctx)
        .into_iter()
        .rev()
        .find(|row| row.0 == check)
        .map_or_else(|| format!("no row {check:?}"), |row| row.2)
}

/// The detail of the check with this name.
pub fn detail(ctx: &Ctx<'_>, check: &str) -> String {
    rows(ctx)
        .into_iter()
        .rev()
        .find(|row| row.0 == check)
        .map(|row| row.3)
        .unwrap_or_default()
}

/// Assert that the check ended with this result.
#[track_caller]
pub fn expect(ctx: &Ctx<'_>, check: &str, wanted: &str) {
    assert_eq!(
        result(ctx, check),
        wanted,
        "{check}: {}; all rows: {:#?}",
        detail(ctx, check),
        rows(ctx)
    );
}

/// One action of the hands: once the tool said `prompt`, do this.
pub type Step<'a> = (&'a str, Box<dyn FnOnce() + Send + 'a>);

/// Run `body` while a thread plays the hands of the tester.
pub fn with_hands<R>(console: &FakeConsole, steps: Vec<Step<'_>>, body: impl FnOnce() -> R) -> R {
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut seen: BTreeMap<&str, usize> = BTreeMap::new();

            for (prompt, action) in steps {
                let times = seen.entry(prompt).or_default();

                *times += 1;
                console.wait_said(prompt, *times);
                action();
            }
        });
        body()
    })
}

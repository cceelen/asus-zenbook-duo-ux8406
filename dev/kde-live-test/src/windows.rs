//! Windows for the swap check, placed and watched through KWin's scripting
//! interface. A KWin script cannot return a value, so the tool offers a
//! service on the session bus that the scripts call.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::fdo::RequestNameFlags;

use crate::context::Ctx;
use crate::dbus;

/// The name of the service of the tool.
pub const SERVICE: &str = "io.github.cceelen.Ux8406LiveTest";
/// Its object.
pub const PATH: &str = "/io/github/cceelen/Ux8406LiveTest";

/// The script that tells the tool the output of every window now and each
/// time it changes.
pub const WATCH: &str = r"
const tell = (window) => {
    if (window.normalWindow && window.output) {
        callDBus('io.github.cceelen.Ux8406LiveTest', '/io/github/cceelen/Ux8406LiveTest',
            'io.github.cceelen.Ux8406LiveTest', 'Report', window.pid + ' ' + window.output.name);
    }
};
const watch = (window) => {
    tell(window);
    window.outputChanged.connect(() => tell(window));
};
workspace.windowList().forEach(watch);
workspace.windowAdded.connect(watch);
";

/// The script that keeps the windows of these processes above the others, or
/// stops doing it.
#[must_use]
pub fn above_script(pids: &[u32], above: bool) -> String {
    let list = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r"
const pids = [{list}];
Array.from(workspace.windowList())
    .filter((w) => pids.includes(w.pid))
    .forEach((w) => {{ w.keepAbove = {above}; }});
"
    )
}

/// The parent of a process, from `/proc/<pid>/stat`: the field after the
/// name, which is in brackets and can have spaces in it.
fn parent_of(ctx: &Ctx<'_>, pid: u32) -> Option<u32> {
    let stat = ctx.read(&format!("/proc/{pid}/stat"))?;

    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// This process and its ancestors: the terminal window of the tester belongs
/// to one of them.
#[must_use]
pub fn ancestors(ctx: &Ctx<'_>) -> Vec<u32> {
    let mut pids = vec![std::process::id()];

    while let Some(parent) = pids.last().and_then(|&pid| parent_of(ctx, pid)) {
        if parent <= 1 || pids.contains(&parent) {
            break;
        }
        pids.push(parent);
    }
    pids
}

/// Keep the terminal window of the tester above the other windows, or let
/// it go again.
///
/// # Errors
///
/// KWin does not take the script.
pub fn keep_above(ctx: &Ctx<'_>, above: bool) -> Result<(), String> {
    let bus = ctx.session_bus()?;
    let name = "ux8406-live-test-above";

    run_script(
        &bus,
        &ctx.report.dir().join("raw"),
        name,
        &above_script(&ancestors(ctx), above),
    )?;
    dbus::call::<_, bool>(
        &bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "unloadScript",
        &(name,),
    )
    .map(drop)
}

/// Write a script, and have KWin load and start it.
fn run_script(
    bus: &Connection,
    directory: &std::path::Path,
    name: &str,
    source: &str,
) -> Result<(), String> {
    let path = directory.join(format!("{name}.js"));

    std::fs::write(&path, source).map_err(|error| format!("{}: {error}", path.display()))?;

    let id: i32 = dbus::call(
        bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "loadScript",
        &(path.to_string_lossy().as_ref(), name),
    )?;

    if id < 0 {
        return Err(format!("KWin did not load the script {name} (id {id})"));
    }
    dbus::call::<_, ()>(
        bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "start",
        &(),
    )
}

/// The script that sends the window of a process to an output.
#[must_use]
pub fn place_script(pid: u32, output: &str) -> String {
    format!(
        r"
const screen = Array.from(workspace.screens).find((s) => s.name === '{output}');
const window = Array.from(workspace.windowList()).find((w) => w.pid === {pid});
if (screen && window) workspace.sendClientToScreen(window, screen);
"
    )
}

type Seen = Arc<Mutex<BTreeMap<u32, String>>>;

struct Reports(Seen);

#[zbus::interface(name = "io.github.cceelen.Ux8406LiveTest")]
impl Reports {
    /// A window and its output: `<process> <connector>`.
    fn report(&self, line: &str) {
        let parsed = line
            .split_once(' ')
            .and_then(|(pid, output)| pid.parse().ok().map(|pid| (pid, output)));

        if let Some((pid, output)) = parsed {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(pid, output.to_owned());
        }
    }
}

/// The service, and KWin's scripting interface.
pub struct Windows {
    _service: Connection,
    bus: Connection,
    seen: Seen,
    scripts: std::path::PathBuf,
}

impl Windows {
    /// Offer the service and load the script that watches the windows.
    ///
    /// # Errors
    ///
    /// Why the service or the script is not there.
    pub fn start(ctx: &Ctx<'_>) -> Result<Self, String> {
        let seen: Seen = Arc::new(Mutex::new(BTreeMap::new()));
        let bus = ctx.session_bus()?;
        let service = Builder::address(ctx.session_address().as_str())
            .and_then(|builder| builder.serve_at(PATH, Reports(Arc::clone(&seen))))
            .and_then(Builder::build)
            .and_then(|connection| {
                connection
                    .request_name_with_flags(SERVICE, RequestNameFlags::DoNotQueue.into())
                    .map(|_| connection)
            })
            .map_err(|error| format!("service {SERVICE}: {error}"))?;
        let windows = Self {
            _service: service,
            bus,
            seen,
            scripts: ctx.report.dir().join("raw"),
        };

        windows.load("ux8406-live-test-watch", WATCH)?;
        Ok(windows)
    }

    /// Load and start a script of this tool.
    ///
    /// # Errors
    ///
    /// KWin does not take it.
    pub fn load(&self, name: &str, source: &str) -> Result<(), String> {
        run_script(&self.bus, &self.scripts, name, source)
    }

    fn scripting<B, R>(&self, method: &str, body: &B) -> Result<R, String>
    where
        B: zbus::export::serde::Serialize + zbus::zvariant::DynamicType,
        R: zbus::export::serde::de::DeserializeOwned + zbus::zvariant::Type,
    {
        dbus::call(
            &self.bus,
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting",
            method,
            body,
        )
    }

    /// Place the window of a process on an output, with a script that
    /// runs once.
    ///
    /// # Errors
    ///
    /// KWin does not take the script.
    pub fn place(&self, pid: u32, output: &str) -> Result<(), String> {
        let name = format!("ux8406-live-test-place-{pid}");

        self.load(&name, &place_script(pid, output))?;
        self.scripting::<_, bool>("unloadScript", &(name.as_str(),))
            .map(drop)
    }

    /// The output of the window of a process, as the script told last.
    #[must_use]
    pub fn output_of(&self, pid: u32) -> Option<String> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&pid)
            .cloned()
    }

    /// Unload the watching script.
    pub fn stop(&self) {
        let _ = self.scripting::<_, bool>("unloadScript", &("ux8406-live-test-watch",));
    }
}

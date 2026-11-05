//! The fakes of the services around KWin: scripting, kglobalacceld, the two
//! programs of the packages and the sensor, on the private bus of a scene.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use asus_ux8406_kde_fakes::input::Device;
use asus_ux8406_kde_fakes::{Bus, Compositor, Input, Output, Sensor, Style};
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::zvariant::OwnedObjectPath;

use super::Fixture;

pub const UPPER: &str = "eDP-1";
pub const LOWER: &str = "eDP-2";

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Called with the name and file of each script at `start`.
pub type StartHook = Box<dyn Fn(&str, &str) + Send>;

/// What the fake scripting service did.
#[derive(Default)]
pub struct Scripts {
    pub loaded: Vec<(String, String)>,
    /// Refuse to load: the id is -1.
    pub refuse: bool,
    /// Answer `loadScript` and `start` with an error.
    pub fail_load: bool,
    pub fail_start: bool,
    /// Called with name and file for each script at `start`.
    pub on_start: Option<StartHook>,
    pub pending: Vec<(String, String)>,
}

pub type SharedScripts = Arc<Mutex<Scripts>>;

struct Scripting(SharedScripts);

#[zbus::interface(name = "org.kde.kwin.Scripting")]
impl Scripting {
    #[zbus(name = "isScriptLoaded")]
    fn is_script_loaded(&self, name: &str) -> bool {
        lock(&self.0).loaded.iter().any(|(known, _)| known == name)
    }

    #[zbus(name = "unloadScript")]
    fn unload_script(&self, name: &str) -> bool {
        let mut scripts = lock(&self.0);
        let before = scripts.loaded.len();

        scripts.loaded.retain(|(known, _)| known != name);
        before != scripts.loaded.len()
    }

    #[zbus(name = "loadScript")]
    fn load_script(&self, path: &str, name: &str) -> zbus::fdo::Result<i32> {
        let mut scripts = lock(&self.0);

        if scripts.fail_load {
            return Err(zbus::fdo::Error::Failed("no".to_owned()));
        }
        if scripts.refuse {
            return Ok(-1);
        }
        scripts.loaded.push((name.to_owned(), path.to_owned()));
        scripts.pending.push((name.to_owned(), path.to_owned()));
        Ok(7)
    }

    #[zbus(name = "start")]
    fn start(&self) -> zbus::fdo::Result<()> {
        if lock(&self.0).fail_start {
            return Err(zbus::fdo::Error::Failed("no".to_owned()));
        }

        let pending = std::mem::take(&mut lock(&self.0).pending);

        for (name, path) in pending {
            // The hook runs without the lock: it calls back over the bus.
            let hook = lock(&self.0).on_start.take();

            if let Some(hook) = hook {
                hook(&name, &path);
                lock(&self.0).on_start = Some(hook);
            }
        }
        Ok(())
    }
}

/// kglobalacceld with these components, each with its actions.
pub struct Accel {
    _connection: Connection,
}

struct GlobalAccel(Vec<String>);

#[zbus::interface(name = "org.kde.KGlobalAccel")]
impl GlobalAccel {
    #[zbus(name = "allComponents")]
    fn all_components(&self) -> Vec<OwnedObjectPath> {
        self.0
            .iter()
            .map(|path| OwnedObjectPath::try_from(path.as_str()).expect("path"))
            .collect()
    }
}

struct Component(Vec<String>);

#[zbus::interface(name = "org.kde.kglobalaccel.Component")]
impl Component {
    #[zbus(name = "shortcutNames")]
    fn shortcut_names(&self) -> Vec<String> {
        self.0.clone()
    }
}

/// A component whose method takes the context.
struct ComponentWithContext(Vec<String>);

#[zbus::interface(name = "org.kde.kglobalaccel.Component")]
impl ComponentWithContext {
    #[zbus(name = "shortcutNames")]
    fn shortcut_names(&self, context: &str) -> zbus::fdo::Result<Vec<String>> {
        if context == "default" {
            Ok(self.0.clone())
        } else {
            Err(zbus::fdo::Error::InvalidArgs(context.to_owned()))
        }
    }
}

impl Accel {
    /// kglobalacceld whose components take the context as an argument.
    pub fn requiring_context(address: &str, components: &[(&str, &[&str])]) -> Self {
        let paths = components
            .iter()
            .map(|(path, _)| (*path).to_owned())
            .collect();
        let mut builder = Builder::address(address)
            .and_then(|builder| builder.serve_at("/kglobalaccel", GlobalAccel(paths)))
            .expect("accel");

        for (path, actions) in components {
            builder = builder
                .serve_at(
                    *path,
                    ComponentWithContext(
                        actions.iter().map(|action| (*action).to_owned()).collect(),
                    ),
                )
                .expect("component");
        }
        Self {
            _connection: builder
                .name("org.kde.kglobalaccel")
                .and_then(Builder::build)
                .expect("accel connection"),
        }
    }

    pub fn new(address: &str, components: &[(&str, &[&str])]) -> Self {
        let paths = components
            .iter()
            .map(|(path, _)| (*path).to_owned())
            .collect();
        let mut builder = Builder::address(address)
            .and_then(|builder| builder.serve_at("/kglobalaccel", GlobalAccel(paths)))
            .expect("accel");

        for (path, actions) in components {
            builder = builder
                .serve_at(
                    *path,
                    Component(actions.iter().map(|action| (*action).to_owned()).collect()),
                )
                .expect("component");
        }
        Self {
            _connection: builder
                .name("org.kde.kglobalaccel")
                .and_then(Builder::build)
                .expect("accel connection"),
        }
    }
}

/// The program that offers the lower screen key, with a count of calls.
pub struct KeysService {
    _connection: Connection,
    pub toggles: Arc<Mutex<u32>>,
}

struct Keys(Arc<Mutex<u32>>, Arc<dyn Fn() + Send + Sync>);

#[zbus::interface(name = "io.github.cceelen.AsusUX8406.Keys")]
impl Keys {
    fn toggle_lower_screen(&self) {
        *lock(&self.0) += 1;
        (self.1)();
    }
}

impl KeysService {
    pub fn new(address: &str, action: impl Fn() + Send + Sync + 'static) -> Self {
        let count = Arc::new(Mutex::new(0));
        let connection = Builder::address(address)
            .and_then(|builder| {
                builder.serve_at(
                    "/io/github/cceelen/AsusUX8406/Keys",
                    Keys(Arc::clone(&count), Arc::new(action)),
                )
            })
            .and_then(|builder| builder.name("io.github.cceelen.AsusUX8406.Keys"))
            .and_then(Builder::build)
            .expect("keys service");

        Self {
            _connection: connection,
            toggles: count,
        }
    }
}

/// The program of the rotation, with its property `Enabled`.
pub struct RotationService {
    connection: Connection,
}

/// The state of the program, and how it takes `SetEnabled`: 0 does nothing,
/// 1 obeys, 2 answers with an error.
struct Rotation(bool, u8, Option<CallHook>);

/// Called with the method, the connector and the orientation (or the value).
pub type CallHook = Arc<dyn Fn(&str, &str, &str) + Send + Sync>;

#[zbus::interface(name = "io.github.cceelen.BuiltinScreenRotation")]
impl Rotation {
    #[zbus(property)]
    fn enabled(&self) -> bool {
        self.0
    }

    /// Follow the orientation, or not; a program that does not obey leaves
    /// the property as it is.
    async fn set_enabled(
        &mut self,
        enabled: bool,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if let Some(hook) = &self.2 {
            hook("SetEnabled", "", &enabled.to_string());
        }
        match self.1 {
            1 => {
                self.0 = enabled;
                self.enabled_changed(&emitter).await?;
                Ok(())
            }
            2 => Err(zbus::fdo::Error::Failed("no".to_owned())),
            _ => Ok(()),
        }
    }

    /// Turn one screen: the hook does what the program would do.
    fn turn_screen(&self, connector: &str, orientation: &str) {
        if let Some(hook) = &self.2 {
            hook("TurnScreen", connector, orientation);
        }
    }
}

impl RotationService {
    pub fn new(address: &str) -> Self {
        Self {
            connection: Builder::address(address)
                .and_then(|builder| {
                    builder.serve_at(
                        "/io/github/cceelen/BuiltinScreenRotation",
                        Rotation(true, 1, None),
                    )
                })
                .and_then(|builder| builder.name("io.github.cceelen.BuiltinScreenRotation"))
                .and_then(Builder::build)
                .expect("rotation service"),
        }
    }

    /// Call this for `TurnScreen` and `SetEnabled`, as the program would act.
    pub fn on_call(&self, hook: impl Fn(&str, &str, &str) + Send + Sync + 'static) {
        self.interface().get_mut().2 = Some(Arc::new(hook));
    }

    /// From now on the program takes `SetEnabled` and does nothing.
    pub fn ignore_set_enabled(&self) {
        self.interface().get_mut().1 = 0;
    }

    /// From now on the program answers `SetEnabled` with an error.
    pub fn fail_set_enabled(&self) {
        self.interface().get_mut().1 = 2;
    }

    fn interface(&self) -> zbus::blocking::object_server::InterfaceRef<Rotation> {
        self.connection
            .object_server()
            .interface::<_, Rotation>("/io/github/cceelen/BuiltinScreenRotation")
            .expect("interface")
    }

    pub fn set_enabled(&self, enabled: bool) {
        let interface = self
            .connection
            .object_server()
            .interface::<_, Rotation>("/io/github/cceelen/BuiltinScreenRotation")
            .expect("interface");

        interface.get_mut().0 = enabled;
    }
}

/// The two panels as the UX8406CA has them.
pub fn panels() -> Vec<Output> {
    vec![
        Output::panel(UPPER, 0, 0),
        Output {
            priority: 1,
            uuid: "uuid-lower".to_owned(),
            ..Output::panel(LOWER, 0, 1440)
        },
    ]
}

/// KWin, the bus and what is on it.
pub struct Scene {
    pub compositor: Arc<Compositor>,
    pub bus: Bus,
    pub input: Input,
    pub scripts: SharedScripts,
}

impl Scene {
    pub fn new(outputs: Vec<Output>) -> Self {
        Self::with(Style::Registry, outputs)
    }

    pub fn with(style: Style, outputs: Vec<Output>) -> Self {
        let compositor = Arc::new(Compositor::new(style, outputs));
        let bus = Bus::new();
        let input = Input::new(
            bus.address(),
            vec![
                Device::touchscreen("event7", 0x4447),
                Device::pen("event8", 0x4447),
                Device::touchscreen("event9", 0x4448),
            ],
        );
        let scripts: SharedScripts = Arc::new(Mutex::new(Scripts::default()));

        input
            .connection()
            .object_server()
            .at("/Scripting", Scripting(Arc::clone(&scripts)))
            .expect("scripting");
        Self {
            compositor,
            bus,
            input,
            scripts,
        }
    }

    /// The fixture whose session is this scene.
    pub fn fixture(&self) -> Fixture {
        Fixture::new()
            .var(
                "WAYLAND_DISPLAY",
                &self.compositor.socket().display().to_string(),
            )
            .var("DBUS_SESSION_BUS_ADDRESS", self.bus.address())
            .var("DBUS_SYSTEM_BUS_ADDRESS", self.bus.address())
            .var("XDG_SESSION_TYPE", "wayland")
    }

    /// The panels mapped as the packages map them.
    pub fn map_touch(&self) {
        self.map_touch_to([UPPER, UPPER, LOWER]);
    }

    /// The panels of `event7`, `event8` and `event9`.
    pub fn map_touch_to(&self, panels: [&str; 3]) {
        for (event, panel) in ["event7", "event8", "event9"].into_iter().zip(panels) {
            self.bus
                .connect()
                .call_method(
                    Some("org.kde.KWin"),
                    format!("/org/kde/KWin/InputDevice/{event}"),
                    Some("org.freedesktop.DBus.Properties"),
                    "Set",
                    &(
                        "org.kde.KWin.InputDevice",
                        "outputName",
                        zbus::zvariant::Value::from(panel),
                    ),
                )
                .expect("set output");
        }
    }

    pub fn sensor(&self, orientation: &str) -> Sensor {
        Sensor::new(self.bus.address(), orientation)
    }
}

/// The rotation program's part in the policy: it sets `never` and sets it
/// back whenever someone changes it. A thread with a connection of its own.
pub struct PolicyKeeper {
    stop: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl PolicyKeeper {
    pub fn start(scene: &Scene) -> Self {
        use asus_ux8406_kde_common::output::{Policy, Session};

        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let socket = scene.compositor.socket();
        let thread = std::thread::spawn(move || {
            let connection = wayland_client::Connection::from_socket(
                std::os::unix::net::UnixStream::connect(socket).expect("socket"),
            )
            .expect("connection");
            let mut session = Session::with(connection).expect("session");
            let never = [
                (UPPER.to_owned(), Policy::Never),
                (LOWER.to_owned(), Policy::Never),
            ];

            while !flag.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = session.dispatch();
                if session
                    .outputs
                    .policies()
                    .iter()
                    .any(|(_, policy)| *policy != Policy::Never)
                {
                    session.set_policies(&never);
                    let _ = session.flush();
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });

        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for PolicyKeeper {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// KWin's windows as the scripts of the tool see them: the test windows
/// are the programs that the fake runner started as `konsole`.
pub struct FakeWindows {
    bus: Connection,
    runner: Arc<super::FakeRunner>,
    outputs: Mutex<BTreeMap<u32, String>>,
    /// Whether a script that places a window has an effect.
    pub obey: std::sync::atomic::AtomicBool,
    /// The screen on which new windows open.
    pub opens_on: Mutex<String>,
}

impl FakeWindows {
    /// The windows, and the hook that makes the scripts of the tool act.
    pub fn install(scene: &Scene, runner: &Arc<super::FakeRunner>) -> Arc<Self> {
        let windows = Arc::new(Self {
            bus: scene.bus.connect(),
            runner: Arc::clone(runner),
            outputs: Mutex::new(BTreeMap::new()),
            obey: std::sync::atomic::AtomicBool::new(true),
            opens_on: Mutex::new(UPPER.to_owned()),
        });
        let hook = Arc::clone(&windows);

        lock(&scene.scripts).on_start = Some(Box::new(move |name, path| hook.run(name, path)));

        let appear = Arc::clone(&windows);

        runner.on_command(move |line| {
            if line.starts_with("konsole") {
                appear.appear();
            }
        });
        windows
    }

    /// New terminal windows open on the screen of `opens_on`.
    fn appear(&self) {
        let screen = lock(&self.opens_on).clone();

        for pid in self.runner.pids("konsole") {
            let new = lock(&self.outputs).insert(pid, screen.clone()).is_none();

            if new {
                self.report(pid);
            }
        }
    }

    fn run(&self, name: &str, path: &str) {
        if name.ends_with("-watch") {
            self.appear();
        } else if self.obey.load(std::sync::atomic::Ordering::SeqCst) {
            let source = std::fs::read_to_string(path).expect("script");
            let output = source.split('\'').nth(1).expect("output").to_owned();
            let pid = source
                .rsplit("=== ")
                .next()
                .and_then(|rest| rest.split(')').next())
                .and_then(|number| number.parse().ok())
                .expect("process");

            lock(&self.outputs).insert(pid, output);
            self.report(pid);
        }
    }

    fn report(&self, pid: u32) {
        let output = lock(&self.outputs).get(&pid).cloned().expect("window");

        self.bus
            .call_method(
                Some(asus_ux8406_kde_live_test::windows::SERVICE),
                asus_ux8406_kde_live_test::windows::PATH,
                Some(asus_ux8406_kde_live_test::windows::SERVICE),
                "Report",
                &(format!("{pid} {output}"),),
            )
            .expect("report");
    }

    /// F8: the two windows swap screens.
    pub fn swap(&self) {
        let pids: Vec<u32> = lock(&self.outputs).keys().copied().collect();

        for pid in pids {
            {
                let mut outputs = lock(&self.outputs);
                let output = outputs.get_mut(&pid).expect("window");

                if output == UPPER { LOWER } else { UPPER }.clone_into(output);
            }
            self.report(pid);
        }
    }
}

/// KWin with the scripting interface only: no input devices.
pub fn scripting_only(address: &str) -> (Connection, SharedScripts) {
    let scripts: SharedScripts = Arc::new(Mutex::new(Scripts::default()));
    let connection = Builder::address(address)
        .and_then(|builder| builder.serve_at("/Scripting", Scripting(Arc::clone(&scripts))))
        .and_then(|builder| builder.name("org.kde.KWin"))
        .and_then(Builder::build)
        .expect("scripting only");

    (connection, scripts)
}

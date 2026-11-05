//! A fake KWin for the outputs: a Wayland server with the protocols
//! `kde_output_device_v2` and `kde_output_management_v2`, in a thread of the
//! test, on a socket in a temporary directory.
//!
//! It behaves as KWin does in the parts that the KDE programs use: it
//! describes each output, takes a configuration and describes the outputs
//! that it changed again, and it remembers each request as a line of text
//! for the test to read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use wayland_server::backend::{ClientData, ClientId, DisconnectReason, GlobalId};
use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, ListeningSocket, New,
    Resource,
};

use crate::protocol::device::kde_output_device_mode_v2::{self, KdeOutputDeviceModeV2};
use crate::protocol::device::kde_output_device_registry_v2::{self, KdeOutputDeviceRegistryV2};
use crate::protocol::device::kde_output_device_v2::{self, KdeOutputDeviceV2};
use crate::protocol::management::kde_output_configuration_v2::{self, KdeOutputConfigurationV2};
use crate::protocol::management::kde_output_management_v2::{self, KdeOutputManagementV2};

/// How the compositor offers the outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// As KWin 6.8: one global `kde_output_device_registry_v2` (version 26),
    /// management version 22.
    Registry,
    /// As KWin 6.6: each output a global `kde_output_device_v2` (version
    /// 20), management version 19.
    Globals,
    /// As a KWin before Plasma 5.26: outputs as globals (version 3),
    /// management version 4, which cannot set the auto-rotate policy.
    Old,
    /// As the first KWin that can set the auto-rotate policy: outputs as
    /// globals (version 3), management version 5.
    FirstPolicy,
    /// Not KWin: no output management.
    NoManagement,
}

impl Style {
    const fn management(self) -> Option<u32> {
        match self {
            Self::Registry => Some(22),
            Self::Globals => Some(19),
            Self::Old => Some(4),
            Self::FirstPolicy => Some(5),
            Self::NoManagement => None,
        }
    }

    const fn device(self) -> u32 {
        match self {
            Self::Registry => 26,
            Self::Globals | Self::NoManagement => 20,
            Self::Old | Self::FirstPolicy => 3,
        }
    }
}

/// One output as the fake has it.
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    /// The connector.
    pub name: String,
    /// The size of its mode in pixels.
    pub width: i32,
    /// The height of its mode in pixels.
    pub height: i32,
    /// Its place in the layout.
    pub x: i32,
    /// Its place in the layout.
    pub y: i32,
    /// Its scale.
    pub scale: f64,
    /// Its transform, as `wl_output` numbers it.
    pub transform: u32,
    /// Whether it is on.
    pub enabled: bool,
    /// Its auto-rotate policy: 0 never, 1 in tablet mode, 2 always.
    pub policy: u32,
    /// Its UUID.
    pub uuid: String,
    /// Its priority; 1 is the primary output.
    pub priority: u32,
    /// The UUID of the output it shows; empty for none.
    pub replication_source: String,
    /// Whether it tells a current mode, or only the one it prefers.
    pub has_current_mode: bool,
}

impl Output {
    /// A panel of the UX8406: 2880x1800 at scale 1.25, upright, on, with
    /// KWin's default policy.
    #[must_use]
    pub fn panel(name: &str, x: i32, y: i32) -> Self {
        Self {
            name: name.to_owned(),
            width: 2880,
            height: 1800,
            x,
            y,
            scale: 1.25,
            transform: 0,
            enabled: true,
            policy: 1,
            uuid: format!("uuid-{name}"),
            priority: 2,
            replication_source: String::new(),
            has_current_mode: true,
        }
    }

    /// A monitor of this size at scale 1.
    #[must_use]
    pub fn monitor(name: &str, width: i32, height: i32, x: i32, y: i32) -> Self {
        Self {
            width,
            height,
            scale: 1.0,
            policy: 0,
            ..Self::panel(name, x, y)
        }
    }
}

/// What the test asks the server thread to do.
enum Command {
    Plug(Output),
    Unplug(String),
    SetPolicy(String, u32),
    SetTransform(String, u32),
    Resize(String, i32, i32),
    AddGlobal,
    Finish,
    Kick,
}

/// What the test and the server thread share.
#[derive(Default)]
struct World {
    outputs: Vec<Output>,
    requests: Vec<String>,
    refusal: Option<String>,
    commands: Vec<Command>,
    clients: usize,
    /// How many times a client released a mode.
    mode_releases: usize,
    /// Do not answer the next `apply`, as a compositor that is slow.
    ignore_apply: bool,
    /// Stop at the next `apply`, and whether that came.
    stop_on_apply: bool,
    stopped: bool,
}

/// The fake. The server stops, and its directory goes, when it is dropped.
pub struct Compositor {
    dir: TempDir,
    world: Arc<Mutex<World>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// The name of the socket in the directory.
const SOCKET: &str = "wayland-test";

fn lock(world: &Mutex<World>) -> MutexGuard<'_, World> {
    world.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Compositor {
    /// A compositor of this style with these outputs.
    ///
    /// # Panics
    ///
    /// If the socket cannot be made.
    #[must_use]
    pub fn new(style: Style, outputs: Vec<Output>) -> Self {
        let dir = tempfile::tempdir().expect("temporary directory");
        let world = Arc::new(Mutex::new(World {
            outputs,
            ..World::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let socket =
            ListeningSocket::bind_absolute(dir.path().join(SOCKET)).expect("Wayland socket");
        let thread = {
            let (world, stop) = (Arc::clone(&world), Arc::clone(&stop));

            std::thread::spawn(move || serve(style, &socket, &world, &stop))
        };

        Self {
            dir,
            world,
            stop,
            thread: Some(thread),
        }
    }

    /// The directory to give as `XDG_RUNTIME_DIR`.
    #[must_use]
    pub fn runtime_dir(&self) -> &Path {
        self.dir.path()
    }

    /// The value to give as `WAYLAND_DISPLAY`.
    #[must_use]
    pub const fn display(&self) -> &'static str {
        SOCKET
    }

    /// The full path of the socket.
    #[must_use]
    pub fn socket(&self) -> PathBuf {
        self.dir.path().join(SOCKET)
    }

    /// The outputs as they are now.
    #[must_use]
    pub fn outputs(&self) -> Vec<Output> {
        lock(&self.world).outputs.clone()
    }

    /// One output as it is now.
    ///
    /// # Panics
    ///
    /// If there is no output of that name.
    #[must_use]
    pub fn output(&self, name: &str) -> Output {
        self.outputs()
            .into_iter()
            .find(|output| output.name == name)
            .expect("an output of that name")
    }

    /// Each request of a configuration so far, as text: `eDP-2 enable 0`,
    /// `eDP-1 position 0 1440`, `apply`, ...
    #[must_use]
    pub fn requests(&self) -> Vec<String> {
        lock(&self.world).requests.clone()
    }

    /// The number of clients connected now.
    #[must_use]
    pub fn clients(&self) -> usize {
        lock(&self.world).clients
    }

    /// Refuse the next configuration, with this reason.
    pub fn refuse_next(&self, reason: &str) {
        lock(&self.world).refusal = Some(reason.to_owned());
    }

    /// Do not answer the next configuration at all, as a compositor that is
    /// slow. Nothing is changed.
    pub fn ignore_next_apply(&self) {
        lock(&self.world).ignore_apply = true;
    }

    /// Stop the server at the next `apply`, as a compositor that ends while
    /// a client waits for its answer.
    pub fn stop_on_next_apply(&self) {
        lock(&self.world).stop_on_apply = true;
    }

    /// Connect an output, as when a monitor is plugged in.
    pub fn plug(&self, output: Output) {
        lock(&self.world).commands.push(Command::Plug(output));
    }

    /// Disconnect an output, as when the keyboard is put on the lower panel.
    pub fn unplug(&self, name: &str) {
        lock(&self.world)
            .commands
            .push(Command::Unplug(name.to_owned()));
    }

    /// Change the auto-rotate policy of an output, as a user does in the
    /// display settings.
    pub fn set_policy(&self, name: &str, policy: u32) {
        lock(&self.world)
            .commands
            .push(Command::SetPolicy(name.to_owned(), policy));
    }

    /// Turn an output to a transform, as another program does.
    pub fn set_transform(&self, name: &str, transform: u32) {
        lock(&self.world)
            .commands
            .push(Command::SetTransform(name.to_owned(), transform));
    }

    /// Change the size of the mode of an output, as when the user picks
    /// another resolution: the old mode goes and a new one comes, the
    /// current mode where the output has one, else the preferred mode.
    pub fn resize(&self, name: &str, width: i32, height: i32) {
        lock(&self.world)
            .commands
            .push(Command::Resize(name.to_owned(), width, height));
    }

    /// How many times a client released a mode.
    #[must_use]
    pub fn mode_releases(&self) -> usize {
        lock(&self.world).mode_releases
    }

    /// Announce a global that has nothing to do with the outputs.
    pub fn add_global(&self) {
        lock(&self.world).commands.push(Command::AddGlobal);
    }

    /// Send `finished` on each registry, without a `stop` from the client.
    pub fn finish(&self) {
        lock(&self.world).commands.push(Command::Finish);
    }

    /// Stop the server, as a compositor that ends: each connection closes.
    pub fn kick(&self) {
        lock(&self.world).commands.push(Command::Kick);
    }

    /// Wait until `done` is true of the fake, at most five seconds.
    ///
    /// # Panics
    ///
    /// If it does not become true in that time.
    pub fn wait_for(&self, what: &str, done: impl Fn(&Self) -> bool) {
        let end = Instant::now() + Duration::from_secs(5);

        while !done(self) {
            assert!(
                Instant::now() < end,
                "the compositor waited in vain: {what}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A connected client.
struct Connected(Arc<Mutex<World>>);

impl ClientData for Connected {
    fn initialized(&self, _: ClientId) {
        lock(&self.0).clients += 1;
    }

    fn disconnected(&self, _: ClientId, _: DisconnectReason) {
        lock(&self.0).clients -= 1;
    }
}

/// An output resource of a client, with its mode.
struct Bound {
    name: String,
    device: KdeOutputDeviceV2,
    mode: KdeOutputDeviceModeV2,
}

/// The state of the server thread.
struct Server {
    style: Style,
    world: Arc<Mutex<World>>,
    bound: Vec<Bound>,
    /// The global of each output, where the outputs are globals.
    globals: Vec<(String, GlobalId)>,
    /// The registries that clients bound.
    registries: Vec<KdeOutputDeviceRegistryV2>,
    clients: Vec<Client>,
}

/// What a configuration collects up to `apply`.
#[derive(Default)]
struct Changes(Mutex<Vec<(String, Change)>>);

#[derive(Clone, Copy)]
enum Change {
    Enable(bool),
    Position(i32, i32),
    Transform(u32),
    Scale(f64),
    Policy(u32),
    Mode,
}

fn serve(style: Style, socket: &ListeningSocket, world: &Arc<Mutex<World>>, stop: &AtomicBool) {
    let mut display = Display::<Server>::new().expect("Wayland display");
    let mut handle = display.handle();
    let mut server = Server {
        style,
        world: Arc::clone(world),
        bound: Vec::new(),
        globals: Vec::new(),
        registries: Vec::new(),
        clients: Vec::new(),
    };

    if let Some(version) = style.management() {
        handle.create_global::<Server, KdeOutputManagementV2, ()>(version, ());
    }
    if style == Style::Registry {
        handle.create_global::<Server, KdeOutputDeviceRegistryV2, ()>(style.device(), ());
    } else {
        for output in lock(world).outputs.clone() {
            server.add_output_global(&handle, &output.name);
        }
    }
    while !stop.load(Ordering::Relaxed) {
        if let Ok(Some(stream)) = socket.accept() {
            let client = handle
                .insert_client(stream, Arc::new(Connected(Arc::clone(world))))
                .expect("client");

            server.clients.push(client);
        }

        let commands = std::mem::take(&mut lock(world).commands);

        if lock(world).stopped
            || commands
                .iter()
                .any(|command| matches!(command, Command::Kick))
        {
            break;
        }
        for command in commands {
            server.run(&handle, command);
        }
        let _ = display.dispatch_clients(&mut server);
        let _ = display.flush_clients();
        std::thread::sleep(Duration::from_millis(1));
    }
}

impl Server {
    fn add_output_global(&mut self, handle: &DisplayHandle, name: &str) {
        let global = handle
            .create_global::<Self, KdeOutputDeviceV2, String>(self.style.device(), name.to_owned());

        self.globals.push((name.to_owned(), global));
    }

    fn output(&self, name: &str) -> Option<Output> {
        lock(&self.world)
            .outputs
            .iter()
            .find(|output| output.name == name)
            .cloned()
    }

    fn run(&mut self, handle: &DisplayHandle, command: Command) {
        match command {
            Command::Plug(output) => {
                let name = output.name.clone();

                lock(&self.world).outputs.push(output);
                if self.style == Style::Registry {
                    for registry in self
                        .registries
                        .clone()
                        .into_iter()
                        .filter(Resource::is_alive)
                    {
                        self.announce(handle, &registry, &name);
                    }
                } else {
                    self.add_output_global(handle, &name);
                }
            }
            Command::Unplug(name) => {
                lock(&self.world)
                    .outputs
                    .retain(|output| output.name != name);
                for bound in self.bound.iter().filter(|bound| bound.name == name) {
                    bound.mode.removed();
                    if self.style == Style::Registry {
                        bound.device.removed();
                    }
                }
                self.bound.retain(|bound| bound.name != name);
                if let Some(index) = self.globals.iter().position(|(n, _)| *n == name) {
                    let (_, global) = self.globals.remove(index);

                    handle.remove_global::<Self>(global);
                }
            }
            Command::SetPolicy(name, policy) => {
                for output in &mut lock(&self.world).outputs {
                    if output.name == name {
                        output.policy = policy;
                    }
                }
                self.describe_again(&[name]);
            }
            Command::SetTransform(name, transform) => {
                for output in &mut lock(&self.world).outputs {
                    if output.name == name {
                        output.transform = transform;
                    }
                }
                self.describe_again(&[name]);
            }
            Command::Resize(name, width, height) => {
                for output in &mut lock(&self.world).outputs {
                    if output.name == name {
                        output.width = width;
                        output.height = height;
                    }
                }
                self.resize(handle, &name);
            }
            Command::AddGlobal => {
                handle.create_global::<Self, KdeOutputDeviceRegistryV2, ()>(1, ());
            }
            Command::Finish => {
                for registry in self.registries.drain(..) {
                    registry.finished();
                }
            }
            // The loop ends before: the server stops.
            Command::Kick => {}
        }
    }

    /// Create an output resource for the client of `registry`.
    fn announce(
        &mut self,
        handle: &DisplayHandle,
        registry: &KdeOutputDeviceRegistryV2,
        name: &str,
    ) {
        let Some(client) = registry.client() else {
            return;
        };
        let device = client
            .create_resource::<KdeOutputDeviceV2, String, Self>(
                handle,
                registry.version(),
                name.to_owned(),
            )
            .expect("output resource");

        registry.output(&device);
        self.describe(handle, &client, device, name);
    }

    /// Send what an output is, with a new mode resource.
    fn describe(
        &mut self,
        handle: &DisplayHandle,
        client: &Client,
        device: KdeOutputDeviceV2,
        name: &str,
    ) {
        let Some(output) = self.output(name) else {
            return;
        };
        let mode = client
            .create_resource::<KdeOutputDeviceModeV2, (), Self>(handle, device.version(), ())
            .expect("mode resource");

        device.mode(&mode);
        mode.size(output.width, output.height);
        mode.refresh(60_000);
        mode.preferred();
        send(&device, &mode, &output);
        self.bound.push(Bound {
            name: name.to_owned(),
            device,
            mode,
        });
    }

    /// Replace the mode of an output by one of the size that the output has
    /// now, and send the output again.
    fn resize(&mut self, handle: &DisplayHandle, name: &str) {
        let Some(output) = self.output(name) else {
            return;
        };

        for bound in self.bound.iter_mut().filter(|bound| bound.name == name) {
            let Some(client) = bound.device.client() else {
                continue;
            };
            let mode = client
                .create_resource::<KdeOutputDeviceModeV2, (), Server>(
                    handle,
                    bound.device.version(),
                    (),
                )
                .expect("mode resource");

            bound.mode.removed();
            bound.device.mode(&mode);
            mode.size(output.width, output.height);
            mode.refresh(60_000);
            if !output.has_current_mode {
                mode.preferred();
            }
            bound.mode = mode;
            send(&bound.device, &bound.mode, &output);
        }
    }

    /// Send the outputs of these names again to each client.
    fn describe_again(&self, names: &[String]) {
        for bound in self
            .bound
            .iter()
            .filter(|bound| names.contains(&bound.name))
        {
            if let Some(output) = self.output(&bound.name) {
                send(&bound.device, &bound.mode, &output);
            }
        }
    }

    fn apply(&mut self, configuration: &KdeOutputConfigurationV2, changes: &Changes) {
        let changes =
            std::mem::take(&mut *changes.0.lock().unwrap_or_else(PoisonError::into_inner));
        let mut world = lock(&self.world);

        world.requests.push("apply".to_owned());
        if world.stop_on_apply {
            world.stopped = true;
            return;
        }
        if std::mem::take(&mut world.ignore_apply) {
            return;
        }
        if let Some(reason) = world.refusal.take() {
            drop(world);
            if configuration.version() >= 12 {
                configuration.failure_reason(reason);
            }
            configuration.failed();
            return;
        }

        let mut names = Vec::new();

        for (name, change) in changes {
            if let Some(output) = world.outputs.iter_mut().find(|output| output.name == name) {
                match change {
                    Change::Enable(on) => output.enabled = on,
                    Change::Position(x, y) => (output.x, output.y) = (x, y),
                    Change::Transform(transform) => output.transform = transform,
                    Change::Scale(scale) => output.scale = scale,
                    Change::Policy(policy) => output.policy = policy,
                    Change::Mode => {}
                }
                names.push(name);
            }
        }
        drop(world);
        self.describe_again(&names);
        configuration.applied();
    }
}

/// Send each property of an output and `done`, as far as the version of the
/// resource has it.
fn send(device: &KdeOutputDeviceV2, mode: &KdeOutputDeviceModeV2, output: &Output) {
    let version = device.version();

    device.geometry(
        output.x,
        output.y,
        300,
        190,
        0,
        "Samsung Display Corp.".to_owned(),
        "ATNA40CT07-0".to_owned(),
        i32::try_from(output.transform).unwrap_or_default(),
    );
    if output.has_current_mode {
        device.current_mode(mode);
    }
    device.serial_number("0".to_owned());
    device.scale(output.scale);
    device.enabled(i32::from(output.enabled));
    device.uuid(output.uuid.clone());
    if version >= 2 {
        device.name(output.name.clone());
    }
    if version >= 4 {
        device.auto_rotate_policy(match output.policy {
            0 => kde_output_device_v2::AutoRotatePolicy::Never,
            1 => kde_output_device_v2::AutoRotatePolicy::InTabletMode,
            _ => kde_output_device_v2::AutoRotatePolicy::Always,
        });
    }
    if version >= 13 {
        device.replication_source(output.replication_source.clone());
    }
    if version >= 18 {
        device.priority(output.priority);
    }
    device.done();
}

impl GlobalDispatch<KdeOutputManagementV2, ()> for Server {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<KdeOutputManagementV2>,
        (): &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl Dispatch<KdeOutputManagementV2, ()> for Server {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &KdeOutputManagementV2,
        request: kde_output_management_v2::Request,
        (): &(),
        _: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        if let kde_output_management_v2::Request::CreateConfiguration { id } = request {
            data_init.init(id, Changes::default());
        }
    }
}

impl Dispatch<KdeOutputConfigurationV2, Changes> for Server {
    fn request(
        server: &mut Self,
        _: &Client,
        configuration: &KdeOutputConfigurationV2,
        request: kde_output_configuration_v2::Request,
        changes: &Changes,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        use kde_output_configuration_v2::Request as R;

        let (device, change, text) = match request {
            R::Apply => return server.apply(configuration, changes),
            R::Enable {
                outputdevice,
                enable,
            } => (
                outputdevice,
                Change::Enable(enable != 0),
                format!("enable {enable}"),
            ),
            R::Position { outputdevice, x, y } => (
                outputdevice,
                Change::Position(x, y),
                format!("position {x} {y}"),
            ),
            R::Transform {
                outputdevice,
                transform,
            } => {
                let transform = u32::try_from(transform).unwrap_or_default();

                (
                    outputdevice,
                    Change::Transform(transform),
                    format!("transform {transform}"),
                )
            }
            R::Scale {
                outputdevice,
                scale,
            } => (outputdevice, Change::Scale(scale), format!("scale {scale}")),
            R::Mode { outputdevice, .. } => (outputdevice, Change::Mode, "mode".to_owned()),
            R::SetAutoRotatePolicy {
                outputdevice,
                policy,
            } => {
                let policy = match policy.into_result() {
                    Ok(kde_output_configuration_v2::AutoRotatePolicy::Never) => 0,
                    Ok(kde_output_configuration_v2::AutoRotatePolicy::InTabletMode) => 1,
                    _ => 2,
                };

                (
                    outputdevice,
                    Change::Policy(policy),
                    format!("policy {policy}"),
                )
            }
            _ => return,
        };
        let name = device.data::<String>().cloned().unwrap_or_default();

        lock(&server.world).requests.push(format!("{name} {text}"));
        changes
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((name, change));
    }
}

impl GlobalDispatch<KdeOutputDeviceRegistryV2, ()> for Server {
    fn bind(
        server: &mut Self,
        handle: &DisplayHandle,
        _: &Client,
        resource: New<KdeOutputDeviceRegistryV2>,
        (): &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let registry = data_init.init(resource, ());
        let names: Vec<String> = lock(&server.world)
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect();

        server.registries.push(registry.clone());
        for name in names {
            server.announce(handle, &registry, &name);
        }
    }
}

impl Dispatch<KdeOutputDeviceRegistryV2, ()> for Server {
    fn request(
        _: &mut Self,
        _: &Client,
        registry: &KdeOutputDeviceRegistryV2,
        request: kde_output_device_registry_v2::Request,
        (): &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        let kde_output_device_registry_v2::Request::Stop = request;
        registry.finished();
    }
}

impl GlobalDispatch<KdeOutputDeviceV2, String> for Server {
    fn bind(
        server: &mut Self,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<KdeOutputDeviceV2>,
        name: &String,
        data_init: &mut DataInit<'_, Self>,
    ) {
        let device = data_init.init(resource, name.clone());

        server.describe(handle, client, device, name);
    }
}

impl Dispatch<KdeOutputDeviceV2, String> for Server {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &KdeOutputDeviceV2,
        _: kde_output_device_v2::Request,
        _: &String,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        // `release` only.
    }
}

impl Dispatch<KdeOutputDeviceModeV2, ()> for Server {
    fn request(
        server: &mut Self,
        _: &Client,
        _: &KdeOutputDeviceModeV2,
        _: kde_output_device_mode_v2::Request,
        (): &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        // `release` only.
        lock(&server.world).mode_releases += 1;
    }
}

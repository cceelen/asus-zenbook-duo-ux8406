//! The outputs of KWin, through its Wayland protocols `kde_output_device_v2`
//! and `kde_output_management_v2`: what the outputs are, and changes to them.
//!
//! KWin 6.6 gives each output as a global of its own; KWin 6.8 gives them
//! through the global `kde_output_device_registry_v2` only. [`Session`] takes
//! the registry where there is one, else the globals.
//!
//! KWin keeps each change that a client applies in its own saved
//! configuration (`kwinoutputconfig.json`): there is no change for the
//! session only.

use std::collections::{BTreeMap, HashMap};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use wayland_client::backend::{ReadEventsGuard, WaylandError};
use wayland_client::globals::{BindError, GlobalError, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

use crate::geometry::{Monitor, Size, State};
use crate::protocol::device::kde_output_device_mode_v2::{self, KdeOutputDeviceModeV2};
use crate::protocol::device::kde_output_device_registry_v2::{self, KdeOutputDeviceRegistryV2};
use crate::protocol::device::kde_output_device_v2::{self, KdeOutputDeviceV2};
use crate::protocol::management::kde_output_configuration_v2::{self, KdeOutputConfigurationV2};
use crate::protocol::management::kde_output_management_v2::KdeOutputManagementV2;

/// The highest version of `kde_output_device_v2` and of its registry that
/// this client knows.
const DEVICE_VERSION: u32 = 26;
/// The first version of `kde_output_device_v2` that has the registry.
const REGISTRY_VERSION: u32 = 21;
/// The highest version of `kde_output_management_v2` that this client knows.
const MANAGEMENT_VERSION: u32 = 22;
/// The first version of the configuration that sets the auto-rotate policy.
const POLICY_VERSION: u32 = 5;
/// The first version of a mode that has the request `release`.
const MODE_RELEASE_VERSION: u32 = 26;
/// The name of the global of one output, as KWin 6.6 has it.
const DEVICE_GLOBAL: &str = "kde_output_device_v2";

/// The connector names of panels that are part of the machine.
const BUILTIN: [&str; 3] = ["eDP", "LVDS", "DSI"];

/// Whether a connector is that of a panel of the machine.
///
/// ```
/// use asus_ux8406_kde_common::output::is_builtin;
///
/// assert!(is_builtin("eDP-2"));
/// assert!(!is_builtin("DP-1"));
/// ```
#[must_use]
pub fn is_builtin(connector: &str) -> bool {
    BUILTIN.iter().any(|prefix| connector.starts_with(prefix))
}

/// When KWin turns an output by itself, after the orientation sensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Never.
    Never,
    /// Only in tablet mode (KWin's default).
    InTabletMode,
    /// Always.
    Always,
}

impl Policy {
    /// The name of the policy in a configuration file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::InTabletMode => "in-tablet-mode",
            Self::Always => "always",
        }
    }

    /// The policy of this name; `None` for another text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Never, Self::InTabletMode, Self::Always]
            .into_iter()
            .find(|policy| policy.as_str() == text)
    }

    const fn from_device(policy: kde_output_device_v2::AutoRotatePolicy) -> Self {
        match policy {
            kde_output_device_v2::AutoRotatePolicy::Never => Self::Never,
            kde_output_device_v2::AutoRotatePolicy::InTabletMode => Self::InTabletMode,
            kde_output_device_v2::AutoRotatePolicy::Always => Self::Always,
        }
    }

    const fn to_configuration(self) -> kde_output_configuration_v2::AutoRotatePolicy {
        match self {
            Self::Never => kde_output_configuration_v2::AutoRotatePolicy::Never,
            Self::InTabletMode => kde_output_configuration_v2::AutoRotatePolicy::InTabletMode,
            Self::Always => kde_output_configuration_v2::AutoRotatePolicy::Always,
        }
    }
}

/// What came from KWin since the last call of [`Session::take_events`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An output came, went or changed.
    Changed,
    /// KWin took the configuration with this number.
    Applied(u64),
    /// KWin refused the configuration with this number, for this reason
    /// (empty where KWin gave none).
    Failed(u64, String),
}

/// Why the outputs cannot be followed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// There is no Wayland compositor to connect to.
    #[error("no Wayland session: {0}")]
    Connect(#[from] wayland_client::ConnectError),
    /// The first answer of the compositor did not come.
    #[error("Wayland: {0}")]
    Globals(#[from] GlobalError),
    /// The compositor does not have the output management of KWin.
    #[error("the compositor is not KWin: it has no output management")]
    NotKwin,
    /// The connection to the compositor failed.
    #[error("Wayland: {0}")]
    Wayland(#[from] WaylandError),
    /// The compositor sent what this client cannot take.
    #[error("Wayland: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),
}

/// What an output says about itself, from one `done` to the next.
#[derive(Clone, Debug, Default)]
struct Info {
    name: Option<String>,
    x: i32,
    y: i32,
    transform: u32,
    scale: f64,
    enabled: bool,
    current_mode: Option<KdeOutputDeviceModeV2>,
    uuid: String,
    policy: Option<Policy>,
    replication_source: String,
    priority: u32,
}

/// What the events of one output built up: the user data of its proxy.
#[derive(Debug, Default)]
struct DeviceData {
    /// What came since the last `done`.
    pending: Mutex<Info>,
    /// What it said up to its last `done`; none before the first one.
    done: Mutex<Option<Info>>,
    /// Its modes.
    modes: Mutex<Vec<KdeOutputDeviceModeV2>>,
}

/// What a mode said: the user data of its proxy.
#[derive(Clone, Copy, Debug, Default)]
struct ModeInfo {
    size: Option<Size>,
    preferred: bool,
}

#[derive(Debug, Default)]
struct ModeData(Mutex<ModeInfo>);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a mode said, from the user data of its proxy.
fn mode_info(mode: &KdeOutputDeviceModeV2) -> ModeInfo {
    mode.data::<ModeData>()
        .map(|data| *lock(&data.0))
        .unwrap_or_default()
}

/// One output.
struct Device {
    proxy: KdeOutputDeviceV2,
    data: Arc<DeviceData>,
    /// The name of its global (KWin 6.6); none from the registry.
    global: Option<u32>,
}

impl Device {
    /// What the output said up to its last `done`, and its connector; none
    /// before it has said both.
    fn described(&self) -> Option<(Info, String)> {
        lock(&self.data.done)
            .clone()
            .and_then(|info| info.name.clone().map(|name| (info, name)))
    }

    /// The mode the output is in, or else the one it prefers, with its size.
    fn mode(&self, info: &Info) -> Option<(KdeOutputDeviceModeV2, Size)> {
        info.current_mode
            .clone()
            .or_else(|| {
                lock(&self.data.modes)
                    .iter()
                    .find(|mode| mode_info(mode).preferred)
                    .cloned()
            })
            .and_then(|mode| mode_info(&mode).size.map(|size| (mode, size)))
    }
}

/// The outputs as the compositor described them, and what it said since the
/// last call of [`Session::take_events`]. The events of the connection are
/// dispatched to it.
pub struct Outputs {
    management: KdeOutputManagementV2,
    /// Whether the outputs are globals of their own (KWin 6.6).
    globals: bool,
    devices: Vec<Device>,
    events: Vec<Event>,
    reasons: HashMap<u64, String>,
    next: u64,
}

impl Outputs {
    fn changed(&mut self) {
        if self.events.last() != Some(&Event::Changed) {
            self.events.push(Event::Changed);
        }
    }

    /// A global came: bind it if it is an output and the outputs are
    /// globals.
    fn announced(
        &mut self,
        registry: &WlRegistry,
        qh: &QueueHandle<Self>,
        name: u32,
        interface: &str,
        version: u32,
    ) {
        if self.globals && interface == DEVICE_GLOBAL {
            let data = Arc::new(DeviceData::default());
            let proxy = registry.bind::<KdeOutputDeviceV2, _, _>(
                name,
                version.min(DEVICE_VERSION),
                qh,
                Arc::clone(&data),
            );

            self.devices.push(Device {
                proxy,
                data,
                global: Some(name),
            });
        }
    }

    /// The outputs that have said who they are, with what they said.
    fn described(&self) -> impl Iterator<Item = (&Device, Info, String)> {
        self.devices
            .iter()
            .filter_map(|device| device.described().map(|(info, name)| (device, info, name)))
    }

    /// The outputs as plain data: the monitors that are on, in the order in
    /// which KWin announced them. An output that shows another one
    /// (replication) is in the monitor of that one. The primary output is
    /// the one of priority 1.
    #[must_use]
    pub fn state(&self) -> State {
        let mut state = State {
            logical_mode: true,
            connected: BTreeMap::new(),
            builtin: Vec::new(),
            logical: Vec::new(),
        };
        let mut uuids = HashMap::new();
        let mut replicas = Vec::new();

        for (device, info, name) in self.described() {
            state
                .connected
                .insert(name.clone(), device.mode(&info).map(|(_, size)| size));
            if is_builtin(&name) {
                state.builtin.push(name.clone());
            }
            if info.enabled && info.replication_source.is_empty() {
                uuids.insert(info.uuid.clone(), state.logical.len());
                state.logical.push(Monitor {
                    x: info.x,
                    y: info.y,
                    scale: info.scale,
                    transform: info.transform,
                    primary: info.priority == 1,
                    connectors: vec![name],
                });
            } else if info.enabled {
                replicas.push((info.replication_source, name));
            }
        }
        for (source, name) in replicas {
            match uuids.get(&source) {
                Some(&index) => state.logical[index].connectors.push(name),
                None => state.logical.push(Monitor {
                    x: 0,
                    y: 0,
                    scale: 1.0,
                    transform: 0,
                    primary: false,
                    connectors: vec![name],
                }),
            }
        }
        state
    }

    /// The auto-rotate policy of each built-in output that tells it.
    #[must_use]
    pub fn policies(&self) -> Vec<(String, Policy)> {
        self.described()
            .filter(|(_, _, name)| is_builtin(name))
            .filter_map(|(_, info, name)| info.policy.map(|policy| (name, policy)))
            .collect()
    }

    fn configuration(&mut self, qh: &QueueHandle<Self>) -> (KdeOutputConfigurationV2, u64) {
        self.next += 1;
        (
            self.management.create_configuration(qh, self.next),
            self.next,
        )
    }

    /// Give KWin a layout: each output that `layout` has first in a
    /// monitor goes on, to its place, transform and scale; an output that
    /// it does not have goes off. An output that it has after another one in
    /// a monitor (a replica) stays as it is. The number of the configuration
    /// comes back in an [`Event`].
    pub fn apply(&mut self, qh: &QueueHandle<Self>, layout: &[Monitor]) -> u64 {
        let (configuration, number) = self.configuration(qh);

        for (device, info, name) in self.described() {
            let first = layout
                .iter()
                .find(|monitor| monitor.connectors.first() == Some(&name));
            let shown = layout
                .iter()
                .any(|monitor| monitor.connectors.contains(&name));

            match first {
                Some(monitor) => {
                    if !info.enabled {
                        configuration.enable(&device.proxy, 1);
                        let _ = device
                            .mode(&info)
                            .map(|(mode, _)| configuration.mode(&device.proxy, &mode));
                    }
                    configuration.position(&device.proxy, monitor.x, monitor.y);
                    if monitor.transform != info.transform {
                        configuration.transform(
                            &device.proxy,
                            i32::try_from(monitor.transform).unwrap_or_default(),
                        );
                    }
                    if monitor.scale.to_bits() != info.scale.to_bits() {
                        configuration.scale(&device.proxy, monitor.scale);
                    }
                }
                None if info.enabled && !shown => configuration.enable(&device.proxy, 0),
                None => {}
            }
        }
        configuration.apply();
        number
    }

    /// Set the auto-rotate policy of outputs, by connector. `None` where
    /// KWin's configuration has no such setting (before Plasma 5.26).
    pub fn set_policies(
        &mut self,
        qh: &QueueHandle<Self>,
        policies: &[(String, Policy)],
    ) -> Option<u64> {
        if self.management.version() < POLICY_VERSION {
            return None;
        }

        let (configuration, number) = self.configuration(qh);

        for (device, _, name) in self.described() {
            if let Some((_, policy)) = policies.iter().find(|(connector, _)| *connector == name) {
                configuration.set_auto_rotate_policy(&device.proxy, policy.to_configuration());
            }
        }
        configuration.apply();
        Some(number)
    }
}

/// The connection to KWin, and the outputs it describes.
pub struct Session {
    connection: Connection,
    queue: EventQueue<Outputs>,
    /// The outputs, as the events of the connection left them.
    pub outputs: Outputs,
}

impl Session {
    /// Connect to the compositor of `WAYLAND_DISPLAY`; refer to
    /// [`Self::with`].
    ///
    /// # Errors
    ///
    /// [`Error::Connect`] without a compositor, and those of [`Self::with`].
    pub fn connect() -> Result<Self, Error> {
        Connection::connect_to_env()
            .map_err(Error::from)
            .and_then(Self::with)
    }

    /// Follow the outputs of the compositor on this connection. This waits
    /// until it has described each output.
    ///
    /// # Errors
    ///
    /// [`Error::NotKwin`] if the compositor is not KWin, and the others if
    /// the connection fails.
    pub fn with(connection: Connection) -> Result<Self, Error> {
        let (globals, queue) = registry_queue_init::<Outputs>(&connection)?;
        let qh = queue.handle();
        let management = globals
            .bind::<KdeOutputManagementV2, _, _>(&qh, 1..=MANAGEMENT_VERSION, ())
            .map_err(not_kwin)?;
        let registry = globals
            .bind::<KdeOutputDeviceRegistryV2, _, _>(&qh, REGISTRY_VERSION..=DEVICE_VERSION, ())
            .ok();
        let mut outputs = Outputs {
            management,
            globals: registry.is_none(),
            devices: Vec::new(),
            events: Vec::new(),
            reasons: HashMap::new(),
            next: 0,
        };

        for global in globals.contents().clone_list() {
            outputs.announced(
                globals.registry(),
                &qh,
                global.name,
                &global.interface,
                global.version,
            );
        }

        let mut session = Self {
            connection,
            queue,
            outputs,
        };

        // The globals, then the outputs and their modes.
        session
            .roundtrip()
            .and_then(|()| session.roundtrip())
            .map(|()| {
                session.outputs.events.clear();
                session
            })
    }

    /// The file descriptor to wait on for events of the compositor.
    #[must_use]
    pub fn fd(&self) -> BorrowedFd<'_> {
        self.connection.as_fd()
    }

    /// Send what is queued, and wait until the compositor has answered it.
    ///
    /// # Errors
    ///
    /// The connection failed.
    pub fn roundtrip(&mut self) -> Result<(), Error> {
        self.queue
            .roundtrip(&mut self.outputs)
            .map(drop)
            .map_err(Error::from)
    }

    /// Send the requests that are queued.
    ///
    /// # Errors
    ///
    /// The connection failed.
    pub fn flush(&self) -> Result<(), Error> {
        self.connection.flush().map_err(Error::from)
    }

    /// Read what the compositor sent, without waiting, and dispatch it.
    ///
    /// # Errors
    ///
    /// The connection failed, or the compositor sent what this client
    /// cannot take.
    pub fn dispatch(&mut self) -> Result<(), Error> {
        let read = self
            .queue
            .prepare_read()
            .map(ReadEventsGuard::read)
            .transpose()
            .map(drop)
            .or_else(unless_would_block);
        let dispatched = self
            .queue
            .dispatch_pending(&mut self.outputs)
            .map(drop)
            .map_err(Error::from);

        read.and(dispatched)
    }

    /// Give KWin a layout; refer to [`Outputs::apply`].
    pub fn apply(&mut self, layout: &[Monitor]) -> u64 {
        let qh = self.queue.handle();

        self.outputs.apply(&qh, layout)
    }

    /// Set auto-rotate policies; refer to [`Outputs::set_policies`].
    pub fn set_policies(&mut self, policies: &[(String, Policy)]) -> Option<u64> {
        let qh = self.queue.handle();

        self.outputs.set_policies(&qh, policies)
    }

    /// What came from KWin since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.outputs.events)
    }

    /// Wait until KWin has answered the configuration with this number:
    /// `Ok(Err(reason))` if it refused it. Other events are kept for
    /// [`Self::take_events`].
    ///
    /// # Errors
    ///
    /// The connection failed.
    pub fn wait(&mut self, number: u64) -> Result<Result<(), String>, Error> {
        loop {
            let answer = self.outputs.events.iter().position(
                |event| matches!(event, Event::Applied(n) | Event::Failed(n, _) if *n == number),
            );

            if let Some(index) = answer {
                return Ok(match self.outputs.events.remove(index) {
                    Event::Failed(_, reason) => Err(reason),
                    _ => Ok(()),
                });
            }
            self.queue
                .blocking_dispatch(&mut self.outputs)
                .map_err(Error::from)?;
        }
    }
}

/// The error of a compositor that is not KWin.
fn not_kwin(_: BindError) -> Error {
    Error::NotKwin
}

/// A failed read, unless it failed only because there was nothing to read.
fn unless_would_block(error: WaylandError) -> Result<(), Error> {
    match error {
        WaylandError::Io(io) if io.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        error => Err(error.into()),
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for Outputs {
    fn event(
        outputs: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => outputs.announced(registry, qh, name, &interface, version),
            // `global_remove`, the only other event.
            event => {
                outputs.devices.retain(|device| {
                    !matches!(event, wl_registry::Event::GlobalRemove { name } if device.global == Some(name))
                });
                outputs.changed();
            }
        }
    }
}

wayland_client::delegate_noop!(Outputs: ignore KdeOutputManagementV2);

impl Dispatch<KdeOutputDeviceRegistryV2, ()> for Outputs {
    fn event(
        outputs: &mut Self,
        _: &KdeOutputDeviceRegistryV2,
        event: kde_output_device_registry_v2::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let kde_output_device_registry_v2::Event::Output { output } = event {
            let data = output.data::<Arc<DeviceData>>().cloned();

            outputs.devices.extend(data.map(|data| Device {
                proxy: output,
                data,
                global: None,
            }));
        }
    }

    wayland_client::event_created_child!(Outputs, KdeOutputDeviceRegistryV2, [
        kde_output_device_registry_v2::EVT_OUTPUT_OPCODE => (KdeOutputDeviceV2, Arc::new(DeviceData::default())),
    ]);
}

impl Dispatch<KdeOutputDeviceV2, Arc<DeviceData>> for Outputs {
    fn event(
        outputs: &mut Self,
        proxy: &KdeOutputDeviceV2,
        event: kde_output_device_v2::Event,
        data: &Arc<DeviceData>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use kde_output_device_v2::Event as E;

        let mut info = lock(&data.pending);

        match event {
            E::Geometry {
                x, y, transform, ..
            } => {
                info.x = x;
                info.y = y;
                info.transform = u32::try_from(transform).unwrap_or_default();
            }
            E::CurrentMode { mode } => info.current_mode = Some(mode),
            E::Mode { mode } => lock(&data.modes).push(mode),
            E::Scale { factor } => info.scale = factor,
            E::Enabled { enabled } => info.enabled = enabled != 0,
            E::Uuid { uuid } => info.uuid = uuid,
            E::Name { name } => info.name = Some(name),
            E::AutoRotatePolicy { policy } => {
                info.policy = policy.into_result().ok().map(Policy::from_device);
            }
            E::ReplicationSource { source } => info.replication_source = source,
            E::Priority { priority } => info.priority = priority,
            E::Done => {
                *lock(&data.done) = Some(info.clone());
                outputs.changed();
            }
            E::Removed => {
                // The event exists from version 21 on, as `release` does.
                outputs.devices.retain(|device| device.proxy != *proxy);
                proxy.release();
                outputs.changed();
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(Outputs, KdeOutputDeviceV2, [
        kde_output_device_v2::EVT_MODE_OPCODE => (KdeOutputDeviceModeV2, ModeData::default()),
    ]);
}

impl Dispatch<KdeOutputDeviceModeV2, ModeData> for Outputs {
    fn event(
        outputs: &mut Self,
        proxy: &KdeOutputDeviceModeV2,
        event: kde_output_device_mode_v2::Event,
        data: &ModeData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use kde_output_device_mode_v2::Event as E;

        match event {
            E::Size { width, height } => lock(&data.0).size = Some(Size { width, height }),
            E::Preferred => lock(&data.0).preferred = true,
            E::Removed => {
                for device in &outputs.devices {
                    lock(&device.data.modes).retain(|mode| mode != proxy);
                }
                if proxy.version() >= MODE_RELEASE_VERSION {
                    proxy.release();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<KdeOutputConfigurationV2, u64> for Outputs {
    fn event(
        outputs: &mut Self,
        proxy: &KdeOutputConfigurationV2,
        event: kde_output_configuration_v2::Event,
        number: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use kde_output_configuration_v2::Event as E;

        match event {
            E::FailureReason { reason } => {
                outputs.reasons.insert(*number, reason);
            }
            E::Applied => {
                outputs.events.push(Event::Applied(*number));
                proxy.destroy();
            }
            E::Failed => {
                let reason = outputs.reasons.remove(number).unwrap_or_default();

                outputs.events.push(Event::Failed(*number, reason));
                proxy.destroy();
            }
        }
    }
}

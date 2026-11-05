//! A fake of KWin's input devices on D-Bus: the service `org.kde.KWin`
//! with the manager `/org/kde/KWin/InputDevice` and one object for each
//! device below it, with the properties that the KDE programs use.

use std::sync::{Arc, Mutex, PoisonError};

use zbus::blocking::Connection;
use zbus::interface;

/// The service.
pub const SERVICE: &str = "org.kde.KWin";
/// The path of the manager.
pub const MANAGER: &str = "/org/kde/KWin/InputDevice";

/// One input device.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Device {
    /// The name of its event node, as `event7`.
    pub sys_name: String,
    /// Its name, as libinput gives it.
    pub name: String,
    /// The vendor of its HID id.
    pub vendor: u32,
    /// The product of its HID id.
    pub product: u32,
    /// Whether it is a touchscreen.
    pub touch: bool,
    /// Whether it is a pen.
    pub tablet_tool: bool,
    /// The connector of the output it acts on; empty for none.
    pub output_name: String,
    /// Whether it refuses a new output.
    pub locked: bool,
}

impl Device {
    /// The touchscreen of a panel: an ELAN controller with this product id.
    #[must_use]
    pub fn touchscreen(sys_name: &str, product: u32) -> Self {
        Self {
            sys_name: sys_name.to_owned(),
            name: format!("ELAN9008:00 04F3:{product:04X}"),
            vendor: 0x04f3,
            product,
            touch: true,
            ..Self::default()
        }
    }

    /// The pen of a panel.
    #[must_use]
    pub fn pen(sys_name: &str, product: u32) -> Self {
        Self {
            name: format!("ELAN9008:00 04F3:{product:04X} Stylus"),
            touch: false,
            tablet_tool: true,
            ..Self::touchscreen(sys_name, product)
        }
    }
}

type Shared = Arc<Mutex<Vec<Device>>>;

fn with<T>(devices: &Shared, sys_name: &str, what: impl FnOnce(&mut Device) -> T) -> Option<T> {
    devices
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter_mut()
        .find(|device| device.sys_name == sys_name)
        .map(what)
}

struct Manager(Shared);

#[interface(name = "org.kde.KWin.InputDeviceManager")]
impl Manager {
    /// The event nodes of the devices.
    #[zbus(property, name = "devicesSysNames")]
    fn devices_sys_names(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|device| device.sys_name.clone())
            .collect()
    }
}

struct Object(Shared, String);

impl Object {
    fn get<T>(&self, what: impl FnOnce(&mut Device) -> T) -> zbus::fdo::Result<T> {
        with(&self.0, &self.1, what).ok_or_else(|| zbus::fdo::Error::UnknownObject(self.1.clone()))
    }
}

#[interface(name = "org.kde.KWin.InputDevice")]
impl Object {
    #[zbus(property, name = "name")]
    fn name(&self) -> zbus::fdo::Result<String> {
        self.get(|device| device.name.clone())
    }

    #[zbus(property, name = "vendor")]
    fn vendor(&self) -> zbus::fdo::Result<u32> {
        self.get(|device| device.vendor)
    }

    #[zbus(property, name = "product")]
    fn product(&self) -> zbus::fdo::Result<u32> {
        self.get(|device| device.product)
    }

    #[zbus(property, name = "touch")]
    fn touch(&self) -> zbus::fdo::Result<bool> {
        self.get(|device| device.touch)
    }

    #[zbus(property, name = "tabletTool")]
    fn tablet_tool(&self) -> zbus::fdo::Result<bool> {
        self.get(|device| device.tablet_tool)
    }

    #[zbus(property, name = "outputName")]
    fn output_name(&self) -> zbus::fdo::Result<String> {
        self.get(|device| device.output_name.clone())
    }

    #[zbus(property, name = "outputName")]
    fn set_output_name(&mut self, output: String) -> zbus::fdo::Result<()> {
        self.get(|device| {
            if device.locked {
                Err(zbus::fdo::Error::AccessDenied("locked".to_owned()))
            } else {
                device.output_name = output;
                Ok(())
            }
        })?
    }
}

/// The fake, connected to a bus under the name of KWin.
pub struct Input {
    connection: Connection,
    devices: Shared,
}

impl Input {
    /// Serve these devices.
    ///
    /// # Panics
    ///
    /// If the bus does not take the service.
    #[must_use]
    pub fn new(address: &str, devices: Vec<Device>) -> Self {
        let shared: Shared = Arc::new(Mutex::new(Vec::new()));
        let connection = zbus::blocking::connection::Builder::address(address)
            .and_then(|builder| builder.serve_at(MANAGER, Manager(Arc::clone(&shared))))
            .and_then(zbus::blocking::connection::Builder::build)
            .expect("connection");
        let input = Self {
            connection,
            devices: shared,
        };

        for device in devices {
            input.serve(device);
        }
        input
            .connection
            .request_name(SERVICE)
            .expect("the name of KWin");
        input
    }

    fn serve(&self, device: Device) {
        let path = format!("{MANAGER}/{}", device.sys_name);
        let object = Object(Arc::clone(&self.devices), device.sys_name.clone());

        self.devices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(device);
        // A name that is no object path has no object: KWin's own names are
        // always paths.
        let _ = self.connection.object_server().at(path, object);
    }

    /// Add a device, as when a pen comes near the panel the first time, and
    /// say so with `deviceAdded`.
    ///
    /// # Panics
    ///
    /// If the signal cannot be sent.
    pub fn add(&self, device: Device) {
        let sys_name = device.sys_name.clone();

        self.serve(device);

        self.connection
            .emit_signal(
                None::<&str>,
                MANAGER,
                "org.kde.KWin.InputDeviceManager",
                "deviceAdded",
                &(sys_name,),
            )
            .expect("signal");
    }

    /// The output that a device acts on now.
    #[must_use]
    pub fn output_name(&self, sys_name: &str) -> String {
        with(&self.devices, sys_name, |device| device.output_name.clone()).unwrap_or_default()
    }
}

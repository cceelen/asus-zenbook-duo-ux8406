//! KWin's input devices on D-Bus (`org.kde.KWin`, objects below
//! `/org/kde/KWin/InputDevice`): which output each touchscreen and pen acts
//! on.
//!
//! Without a setting KWin gives a touchscreen the first output of its size.
//! The two panels of the UX8406 have one size, so both touchscreens get the
//! upper panel. KWin keeps the output that a client sets (`outputName`) in
//! `kcminputrc`, by the UUID of the output.

use std::collections::HashMap;

use asus_ux8406_kde_common::touch::Assignment;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedValue, Value};

/// The service of KWin.
const KWIN: &str = "org.kde.KWin";
/// The path of the manager; each device is below it, by its event node.
const MANAGER: &str = "/org/kde/KWin/InputDevice";
const MANAGER_INTERFACE: &str = "org.kde.KWin.InputDeviceManager";
const DEVICE_INTERFACE: &str = "org.kde.KWin.InputDevice";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";

/// The manager of the input devices, for its signal.
#[zbus::proxy(
    interface = "org.kde.KWin.InputDeviceManager",
    default_service = "org.kde.KWin",
    default_path = "/org/kde/KWin/InputDevice"
)]
trait Manager {
    /// A device came.
    #[zbus(signal, name = "deviceAdded")]
    fn device_added(&self, sys_name: String) -> zbus::Result<()>;
}

/// What the program needs to know of an input device.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Device {
    /// Its event node, as `event7`.
    pub sys_name: String,
    /// The vendor of its HID id.
    pub vendor: u32,
    /// The product of its HID id.
    pub product: u32,
    /// Whether it is a touchscreen or a pen.
    pub touch: bool,
    /// The connector of the output it acts on; empty for none.
    pub output_name: String,
}

impl Device {
    /// Its HID id as the controllers have it: `04f3:4447`.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor, self.product)
    }

    /// The device from the properties that KWin gives. `vendor` and `product`
    /// must be there; the flags and `outputName` may be missing.
    ///
    /// # Errors
    ///
    /// A property is missing that must be there, or one has another type
    /// than KWin documents: the device would be taken for another one
    /// otherwise.
    pub fn from_properties(
        sys_name: &str,
        properties: &HashMap<String, OwnedValue>,
    ) -> Result<Self, String> {
        let required = |key: &str| {
            property::<u32>(properties, key)
                .and_then(|value| value.ok_or_else(|| format!("no property {key}")))
        };
        let flag = |key: &str| property::<bool>(properties, key).map(Option::unwrap_or_default);

        Ok(Self {
            sys_name: sys_name.to_owned(),
            vendor: required("vendor")?,
            product: required("product")?,
            touch: flag("touch")? || flag("tabletTool")?,
            output_name: property::<String>(properties, "outputName")?.unwrap_or_default(),
        })
    }
}

/// A property of a device: none where KWin does not give it, an error where
/// it has another type.
fn property<T: TryFrom<OwnedValue>>(
    properties: &HashMap<String, OwnedValue>,
    key: &str,
) -> Result<Option<T>, String> {
    properties
        .get(key)
        .map(|value| {
            T::try_from(value.clone())
                .map_err(|_| format!("property {key} has the type {}", value.value_signature()))
        })
        .transpose()
}

/// The devices that get a panel: each touchscreen and pen of a controller
/// in `assignments` that acts on no output yet, as `(sys_name, connector)`.
/// A device that has an output keeps it: that is KWin's saved setting, or
/// the user's.
#[must_use]
pub fn plan(devices: &[Device], assignments: &[Assignment]) -> Vec<(String, String)> {
    devices
        .iter()
        .filter(|device| device.touch && device.output_name.is_empty())
        .filter_map(|device| {
            let id = device.id();

            assignments
                .iter()
                .find(|assignment| assignment.id == id)
                .map(|assignment| (device.sys_name.clone(), assignment.connector.clone()))
        })
        .collect()
}

/// One input device, or why it could not be read.
fn device(connection: &Connection, sys_name: &str) -> zbus::Result<Device> {
    connection
        .call_method(
            Some(KWIN),
            format!("{MANAGER}/{sys_name}"),
            Some(PROPERTIES),
            "GetAll",
            &(DEVICE_INTERFACE,),
        )
        .and_then(|reply| reply.body().deserialize::<HashMap<String, OwnedValue>>())
        .and_then(|properties| {
            Device::from_properties(sys_name, &properties).map_err(zbus::Error::Failure)
        })
}

/// The input devices that KWin has now. A device that cannot be read is
/// left out.
///
/// # Errors
///
/// KWin does not list its devices.
pub fn devices(connection: &Connection) -> zbus::Result<Vec<Device>> {
    connection
        .call_method(
            Some(KWIN),
            MANAGER,
            Some(PROPERTIES),
            "Get",
            &(MANAGER_INTERFACE, "devicesSysNames"),
        )
        .and_then(|reply| reply.body().deserialize::<OwnedValue>())
        .and_then(|value| Vec::<String>::try_from(value).map_err(zbus::Error::from))
        .map(|names| {
            names
                .iter()
                .filter_map(|name| {
                    device(connection, name)
                        .inspect_err(|error| log::warn!("input device {name}: {error}"))
                        .ok()
                })
                .collect()
        })
}

/// Have a device act on the output of this connector.
///
/// # Errors
///
/// KWin did not take it.
pub fn set_output(connection: &Connection, sys_name: &str, connector: &str) -> zbus::Result<()> {
    connection
        .call_method(
            Some(KWIN),
            format!("{MANAGER}/{sys_name}"),
            Some(PROPERTIES),
            "Set",
            &(DEVICE_INTERFACE, "outputName", Value::from(connector)),
        )
        .map(drop)
}

/// Call `added` in a thread of its own each time KWin has a new device.
///
/// # Errors
///
/// The signal cannot be followed.
pub fn on_added(connection: &Connection, added: impl Fn() + Send + 'static) -> zbus::Result<()> {
    ManagerProxyBlocking::new(connection)
        .and_then(|manager| manager.receive_device_added())
        .map(|signals| {
            std::thread::spawn(move || signals.for_each(|_| added()));
        })
}

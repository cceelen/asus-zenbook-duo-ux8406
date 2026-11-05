//! Small D-Bus calls with the error already as text for the report.

use zbus::blocking::Connection;
use zbus::export::serde::Serialize;
use zbus::export::serde::de::DeserializeOwned;
use zbus::zvariant::{DynamicType, OwnedValue, Type};

/// Call a method and read its answer.
///
/// # Errors
///
/// The error of the call, with the method named.
pub fn call<B, R>(
    bus: &Connection,
    destination: &str,
    path: &str,
    interface: &str,
    method: &str,
    body: &B,
) -> Result<R, String>
where
    B: Serialize + DynamicType,
    R: DeserializeOwned + Type,
{
    bus.call_method(Some(destination), path, Some(interface), method, body)
        .and_then(|reply| reply.body().deserialize::<R>())
        .map_err(|error| format!("{destination} {interface}.{method}: {error}"))
}

/// Read a property.
///
/// # Errors
///
/// The error of the call, or a value of another type.
pub fn property<R>(
    bus: &Connection,
    destination: &str,
    path: &str,
    interface: &str,
    name: &str,
) -> Result<R, String>
where
    R: TryFrom<OwnedValue>,
    R::Error: Into<zbus::Error>,
{
    call::<_, OwnedValue>(
        bus,
        destination,
        path,
        "org.freedesktop.DBus.Properties",
        "Get",
        &(interface, name),
    )
    .and_then(|value| {
        R::try_from(value)
            .map_err(|error| format!("{destination} {interface}.{name}: {}", error.into()))
    })
}

/// Whether a program owns a name on the bus.
///
/// # Errors
///
/// The bus does not answer.
pub fn has_owner(bus: &Connection, name: &str) -> Result<bool, String> {
    call(
        bus,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        &(name,),
    )
}

/// Wait until a program owns a name, at most `patience`.
#[must_use]
pub fn wait_owner(
    bus: &Connection,
    name: &str,
    patience: std::time::Duration,
    poll: std::time::Duration,
) -> bool {
    let end = std::time::Instant::now() + patience;

    loop {
        if has_owner(bus, name) == Ok(true) {
            return true;
        }
        if std::time::Instant::now() >= end {
            return false;
        }
        std::thread::sleep(poll);
    }
}

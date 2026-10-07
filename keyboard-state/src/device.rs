//! The HID devices of the keyboard, by the names the kernel gives them.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Bus, vendor and product of the keyboard's connections, as they start the
/// kernel's name of a HID device: USB and Bluetooth of the UX8406CA, and USB
/// of the UX8406MA.
const CONNECTIONS: [&str; 3] = ["0003:0B05:1BF2.", "0005:0B05:1BF3.", "0003:0B05:1B2C."];
/// The kernel numbers HID devices with four hexadecimal digits, and with more
/// once those are used up.
const INSTANCE_DIGITS: std::ops::RangeInclusive<usize> = 4..=8;

/// A HID device of the keyboard, such as `0003:0B05:1BF2.005D`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device(String);

impl Device {
    /// The name of the directory that udev-hid-bpf pins the programs and maps
    /// of this device in: the device's name with `_` for `:` and `.`.
    #[must_use]
    pub fn pin_name(&self) -> String {
        self.0.replace([':', '.'], "_")
    }

    /// The device that udev-hid-bpf named a directory of pins after, if it
    /// is one of the keyboard.
    #[must_use]
    pub fn from_pin_name(name: &str) -> Option<Self> {
        let mut parts = name.splitn(4, '_');
        let (bus, vendor, product, instance) =
            (parts.next()?, parts.next()?, parts.next()?, parts.next()?);

        format!("{bus}:{vendor}:{product}.{instance}").parse().ok()
    }
}

impl FromStr for Device {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let instance = CONNECTIONS
            .iter()
            .find_map(|connection| name.strip_prefix(connection));

        match instance {
            Some(instance)
                if INSTANCE_DIGITS.contains(&instance.len())
                    && instance
                        .bytes()
                        .all(|digit| matches!(digit, b'0'..=b'9' | b'A'..=b'F')) =>
            {
                Ok(Self(name.to_owned()))
            }
            _ => Err(Error::NotTheKeyboard(name.to_owned())),
        }
    }
}

impl fmt::Display for Device {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

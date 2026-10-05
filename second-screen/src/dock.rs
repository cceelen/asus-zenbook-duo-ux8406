//! Recognising the dock port and the lower panel by name.
//!
//! Nothing here touches the file system, so every rule can be tested with
//! plain texts.
//!
//! The detachable keyboard can lie on the lower panel. It is then connected
//! through the pogo pins, which the kernel shows as a USB device on one fixed
//! port of one controller. On the cable the same keyboard is on another port,
//! and over Bluetooth it is not on USB at all.

/// A USB device id as sysfs prints it: four lower-case hex digits each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsbId<'a> {
    /// `idVendor`
    pub vendor: &'a str,
    /// `idProduct`
    pub product: &'a str,
}

/// The detachable keyboard on USB: that of the UX8406CA, and that of the
/// UX8406MA.
pub const KEYBOARDS: [UsbId<'static>; 2] = [
    UsbId {
        vendor: "0b05",
        product: "1bf2",
    },
    UsbId {
        vendor: "0b05",
        product: "1b2c",
    },
];

/// The root hub of the USB controller the dock port belongs to.
const DOCK_CONTROLLER: &str = "/0000:00:14.0/usb";
/// The port of that root hub.
const DOCK_PORT: &str = "6";

/// One of the two built-in panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    /// The panel in the lid.
    Upper,
    /// The panel the keyboard can lie on.
    Lower,
}

impl Panel {
    /// How the panel's DRM connector name ends.
    const fn connector_suffix(self) -> &'static str {
        match self {
            Self::Upper => "-eDP-1",
            Self::Lower => "-eDP-2",
        }
    }
}

impl std::fmt::Display for Panel {
    /// The panel's position, as in "upper panel".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
        })
    }
}

/// A DRM connector is `cardN-eDP-1` or `cardN-eDP-2`; N depends on the order
/// in which the graphics drivers came up.
const CONNECTOR_PREFIX: &str = "card";

/// Whether the text is one or more decimal digits and nothing else.
fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether a resolved sysfs path of a USB device is the dock port.
///
/// The path has to end in the controller, its root hub `usbN` and the device
/// `N-6`, as in `/sys/devices/pci0000:00/0000:00:14.0/usb3/3-6`.
///
/// ```
/// use asus_ux8406_second_screen::dock::is_dock_port;
///
/// assert!(is_dock_port("/sys/devices/pci0000:00/0000:00:14.0/usb3/3-6"));
/// // The same keyboard on the cable.
/// assert!(!is_dock_port("/sys/devices/pci0000:00/0000:00:14.0/usb3/3-2"));
/// ```
#[must_use]
pub fn is_dock_port(path: &str) -> bool {
    let Some((_, after_controller)) = path.split_once(DOCK_CONTROLLER) else {
        return false;
    };
    let Some((bus, device)) = after_controller.split_once('/') else {
        return false;
    };
    let port = device
        .strip_prefix(bus)
        .and_then(|rest| rest.strip_prefix('-'));

    is_number(bus) && port == Some(DOCK_PORT)
}

/// Which panel a name in `/sys/class/drm` is the connector of, if any.
///
/// ```
/// use asus_ux8406_second_screen::dock::{Panel, panel_of_connector};
///
/// assert_eq!(panel_of_connector("card1-eDP-1"), Some(Panel::Upper));
/// assert_eq!(panel_of_connector("card1-eDP-2"), Some(Panel::Lower));
/// assert_eq!(panel_of_connector("card1-DP-1"), None);
/// ```
#[must_use]
pub fn panel_of_connector(name: &str) -> Option<Panel> {
    let rest = name.strip_prefix(CONNECTOR_PREFIX)?;

    [Panel::Upper, Panel::Lower].into_iter().find(|panel| {
        rest.strip_suffix(panel.connector_suffix())
            .is_some_and(is_number)
    })
}

/// Which panel a backlight device belongs to, from its resolved sysfs path.
///
/// The kernel hangs a panel's backlight below the panel's connector, as in
/// `/sys/devices/pci0000:00/0000:00:02.0/drm/card1/card1-eDP-1/intel_backlight`.
/// A backlight that hangs anywhere else belongs to neither panel.
#[must_use]
pub fn panel_of_backlight(path: &str) -> Option<Panel> {
    let (parent, _device) = path.rsplit_once('/')?;
    let (_, connector) = parent.rsplit_once('/')?;

    panel_of_connector(connector)
}

/// A brightness on one panel's scale as the same brightness on another's.
///
/// `None` if the value is not on the scale it is said to be on.
///
/// ```
/// use asus_ux8406_second_screen::dock::scale_brightness;
///
/// assert_eq!(scale_brightness(182, 400, 400), Some(182));
/// assert_eq!(scale_brightness(100, 400, 255), Some(64));
/// assert_eq!(scale_brightness(401, 400, 400), None);
/// ```
#[must_use]
pub fn scale_brightness(value: u32, from_max: u32, to_max: u32) -> Option<u32> {
    if from_max == 0 || value > from_max {
        return None;
    }

    let scaled =
        (u64::from(value) * u64::from(to_max) + u64::from(from_max) / 2) / u64::from(from_max);

    u32::try_from(scaled).ok()
}

/// Whether a name in `/sys/class/drm` is the lower panel's connector.
///
/// ```
/// use asus_ux8406_second_screen::dock::is_lower_panel;
///
/// assert!(is_lower_panel("card1-eDP-2"));
/// assert!(!is_lower_panel("card1-eDP-1"));
/// assert!(!is_lower_panel("card1-eDP-2-backlight"));
/// ```
#[must_use]
pub fn is_lower_panel(name: &str) -> bool {
    panel_of_connector(name) == Some(Panel::Lower)
}

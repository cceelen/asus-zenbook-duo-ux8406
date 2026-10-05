//! What asus-ux8406-second-screen reads from and writes to sysfs.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::dock::{self, Panel, UsbId};
use crate::error::Error;

/// What the product name of a supported machine contains: every UX8406.
pub(crate) const MODEL: &str = "UX8406";
/// No attribute read here is longer than this many bytes.
pub const ATTRIBUTE_LIMIT: usize = 96;

/// How the kernel decides whether a display connector is connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detection {
    /// Reported as disconnected, whatever the hardware says.
    ForcedOff,
    /// Left to the hardware.
    Automatic,
}

impl Detection {
    /// What the lower panel's connector should be given for a dock state.
    #[must_use]
    pub const fn for_keyboard(docked: bool) -> Self {
        if docked {
            Self::ForcedOff
        } else {
            Self::Automatic
        }
    }

    /// The word the connector's `status` attribute takes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ForcedOff => "off",
            Self::Automatic => "detect",
        }
    }
}

impl fmt::Display for Detection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The sysfs tree, or a directory that stands in for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sysfs {
    root: PathBuf,
}

impl Default for Sysfs {
    fn default() -> Self {
        Self::new("/sys")
    }
}

impl Sysfs {
    /// A tree rooted somewhere other than `/sys`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Whether the machine is the model this program knows.
    #[must_use]
    pub fn is_supported_model(&self) -> bool {
        read_attribute(&self.root.join("class/dmi/id/product_name"))
            .is_some_and(|name| name.contains(MODEL))
    }

    /// Whether the keyboard lies on the lower panel right now.
    ///
    /// A tree without any USB devices counts as not docked.
    #[must_use]
    pub fn keyboard_docked(&self) -> bool {
        let Ok(entries) = fs::read_dir(self.root.join("bus/usb/devices")) else {
            return false;
        };

        entries
            .filter_map(Result::ok)
            .any(|entry| is_usb_device(&entry.file_name()) && is_docked_keyboard(&entry.path()))
    }

    /// The `status` attribute of every connector of the lower panel.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the DRM directory cannot be listed, and
    /// [`Error::NoLowerPanel`] if it holds no such connector.
    pub fn lower_panel_status_files(&self) -> Result<Vec<PathBuf>, Error> {
        let directory = self.root.join("class/drm");
        let io_error = |source| Error::Io {
            path: directory.clone(),
            source,
        };
        let mut files = Vec::new();

        for entry in fs::read_dir(&directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;

            if entry.file_name().to_str().is_some_and(dock::is_lower_panel) {
                files.push(entry.path().join("status"));
            }
        }
        if files.is_empty() {
            return Err(Error::NoLowerPanel);
        }
        files.sort();
        Ok(files)
    }
}

impl Sysfs {
    /// The backlight device of one of the built-in panels.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the backlight directory cannot be listed, and
    /// [`Error::NoBacklight`] if no backlight hangs below the panel's
    /// connector.
    pub fn backlight(&self, panel: Panel) -> Result<Backlight, Error> {
        let directory = self.root.join("class/backlight");
        let io_error = |source| Error::Io {
            path: directory.clone(),
            source,
        };
        let mut found = Vec::new();

        for entry in fs::read_dir(&directory).map_err(io_error)? {
            let Ok(device) = entry.map_err(io_error)?.path().canonicalize() else {
                continue;
            };

            if device.to_str().and_then(dock::panel_of_backlight) == Some(panel) {
                found.push(device);
            }
        }
        found.sort();
        found
            .into_iter()
            .next()
            .map(|device| Backlight { device })
            .ok_or(Error::NoBacklight(panel))
    }
}

/// A backlight device of the kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backlight {
    device: PathBuf,
}

impl Backlight {
    /// The attribute the brightness is read from and written to.
    #[must_use]
    pub fn brightness_file(&self) -> PathBuf {
        self.device.join("brightness")
    }

    /// The brightness the device was last given.
    ///
    /// # Errors
    ///
    /// [`Error::BadBrightness`] if the attribute does not hold a number.
    pub fn brightness(&self) -> Result<u32, Error> {
        read_number(&self.brightness_file())
    }

    /// The highest brightness the device takes.
    ///
    /// # Errors
    ///
    /// [`Error::BadBrightness`] if the attribute does not hold a number.
    pub fn max_brightness(&self) -> Result<u32, Error> {
        read_number(&self.device.join("max_brightness"))
    }

    /// Give the device a brightness.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the attribute cannot be opened or written.
    pub fn set_brightness(&self, value: u32) -> Result<(), Error> {
        write_attribute(&self.brightness_file(), &value.to_string())
    }
}

/// A sysfs attribute that holds one decimal number.
fn read_number(path: &Path) -> Result<u32, Error> {
    read_attribute(path)
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| Error::BadBrightness(path.to_owned()))
}

/// Write a short text to a sysfs attribute.
fn write_attribute(path: &Path, text: &str) -> Result<(), Error> {
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })
}

/// Give a connector its detection through its `status` attribute.
///
/// # Errors
///
/// [`Error::Io`] if the attribute cannot be opened or written.
pub fn write_detection(status: &Path, detection: Detection) -> Result<(), Error> {
    write_attribute(status, detection.as_str())
}

/// The first line of a sysfs attribute.
///
/// `None` if the file cannot be read, is not text, or is longer than an
/// attribute can be.
#[must_use]
pub fn read_attribute(path: &Path) -> Option<String> {
    let limit = u64::try_from(ATTRIBUTE_LIMIT).ok()?;
    let mut text = String::new();

    File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_string(&mut text)
        .ok()?;
    if text.len() > ATTRIBUTE_LIMIT {
        return None;
    }
    Some(text.lines().next().unwrap_or_default().to_owned())
}

/// Whether a name in `bus/usb/devices` is a device, such as `3-6`.
///
/// Interfaces (`3-6:1.0`) and root hubs (`usb3`) are not.
fn is_usb_device(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| name.contains('-') && !name.contains(':'))
}

/// Whether an entry of `bus/usb/devices` is the keyboard on the dock port.
fn is_docked_keyboard(link: &Path) -> bool {
    let Ok(device) = link.canonicalize() else {
        return false;
    };
    let id = (
        read_attribute(&device.join("idVendor")),
        read_attribute(&device.join("idProduct")),
    );

    device.to_str().is_some_and(dock::is_dock_port)
        && matches!(
            id,
            (Some(vendor), Some(product))
                if dock::KEYBOARDS.contains(&UsbId { vendor: &vendor, product: &product })
        )
}

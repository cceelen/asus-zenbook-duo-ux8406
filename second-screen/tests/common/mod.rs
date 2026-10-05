//! A stand-in for `/sys` that every test program of this crate shares.
#![allow(
    dead_code,
    reason = "each test program uses a different part of the shared helpers"
)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::fixture::PathCopy;
use asus_ux8406_second_screen::Sysfs;
use rstest::fixture;

/// The directory of the fixture trees.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
/// Where the kernel keeps the devices of the dock port's root hub.
const HUB: &str = "devices/pci0000:00/0000:00:14.0/usb3";

/// The lower panel's connector.
pub const LOWER_STATUS: &str = "class/drm/card1-eDP-2/status";
/// The upper panel's connector.
pub const UPPER_STATUS: &str = "class/drm/card1-eDP-1/status";
/// What a connector holds before asus-ux8406-second-screen has written to it.
pub const UNTOUCHED: &str = "connected\n";
/// The product name of the machine's model.
pub const PRODUCT_NAME: &str = "class/dmi/id/product_name";
/// A product name of another model of the same family.
pub const OTHER_MODEL: &str = "ASUS Zenbook 14 UX3405MA_UX3405MA\n";
/// The product id of the keyboard on the dock port.
pub const DOCKED_KEYBOARD_PRODUCT: &str = "devices/pci0000:00/0000:00:14.0/usb3/3-6/idProduct";
/// The product name of the other UX8406.
pub const SISTER_MODEL: &str = "ASUS Zenbook Duo UX8406MA_UX8406MA\n";
/// The upper panel's backlight.
pub const UPPER_BRIGHTNESS: &str =
    "devices/pci0000:00/0000:00:02.0/drm/card1/card1-eDP-1/intel_backlight/brightness";
/// The lower panel's backlight.
pub const LOWER_BRIGHTNESS: &str =
    "devices/pci0000:00/0000:00:02.0/drm/card1/card1-eDP-2/card1-eDP-2-backlight/brightness";
/// The backlight of the firmware driver, which belongs to neither panel.
pub const SCREENPAD_BRIGHTNESS: &str =
    "devices/platform/asus-nb-wmi/backlight/asus_screenpad/brightness";

/// A copy of a fixture tree in a temporary directory, removed again when it
/// goes out of scope.
pub struct Sys {
    dir: TempDir,
}

impl Sys {
    /// A copy of the named tree from `tests/fixtures`, with the links the
    /// kernel keeps in `class/backlight` and `bus/usb/devices`.
    pub fn from_fixture(name: &str) -> Self {
        let sys = Self::empty();

        sys.dir
            .copy_from(Path::new(FIXTURES).join(name), &["**"])
            .unwrap();
        link_backlights(sys.root(), &sys.root().join("devices"));
        link_usb_devices(sys.root());
        sys
    }

    /// A tree without any file.
    pub fn empty() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    /// The directory that stands in for `/sys`.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// A path below the root.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    /// The tree as the library takes it.
    pub fn sysfs(&self) -> Sysfs {
        Sysfs::new(self.root())
    }

    /// The text of a file below the root.
    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    /// Replace the text of a file below the root, creating it and the
    /// directories above it if need be.
    pub fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// The program, set to run on this tree.
    pub fn program(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asus-ux8406-second-screen"));

        command.arg("-r").arg(self.root());
        command
    }

    /// Make the machine another model.
    pub fn make_other_model(&self) {
        self.write(PRODUCT_NAME, OTHER_MODEL);
    }

    /// Take the keyboard off the dock port.
    pub fn undock(&self) {
        fs::remove_file(self.path("bus/usb/devices/3-6")).unwrap();
    }
}

/// Link every backlight device below `directory` in `class/backlight`.
fn link_backlights(root: &Path, directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries {
        let path = entry.unwrap().path();

        if !path.is_dir() {
            continue;
        }
        if path.join("max_brightness").is_file() {
            link(root, &path, "class/backlight");
        } else {
            link_backlights(root, &path);
        }
    }
}

/// Link every USB device of the dock port's root hub in `bus/usb/devices`.
fn link_usb_devices(root: &Path) {
    let Ok(entries) = fs::read_dir(root.join(HUB)) else {
        return;
    };

    for entry in entries {
        link(root, &entry.unwrap().path(), "bus/usb/devices");
    }
}

/// A link to `device` in `directory`, named like the device and relative like
/// the kernel's.
fn link(root: &Path, device: &Path, directory: &str) {
    let up = "../".repeat(directory.matches('/').count() + 1);
    let target = Path::new(&up).join(device.strip_prefix(root).unwrap());

    fs::create_dir_all(root.join(directory)).unwrap();
    symlink(
        target,
        root.join(directory).join(device.file_name().unwrap()),
    )
    .unwrap();
}

/// A UX8406CA with both panels, the upper one at 253 and the lower one at
/// 170, and the keyboard on the dock port.
#[fixture]
pub fn docked() -> Sys {
    Sys::from_fixture("ux8406ca-docked")
}

/// A UX8406CA with both connectors, and nothing on USB.
#[fixture]
pub fn panels_only() -> Sys {
    Sys::from_fixture("ux8406ca-panels-only")
}

/// A UX8406CA with both connectors and the keyboard on the cable's port.
#[fixture]
pub fn keyboard_on_cable() -> Sys {
    Sys::from_fixture("ux8406ca-keyboard-on-cable")
}

/// A UX8406CA with no backlight but the firmware's.
#[fixture]
pub fn screenpad_only() -> Sys {
    Sys::from_fixture("ux8406ca-screenpad-only")
}

/// A UX8406CA without the lower panel's connector, the keyboard docked.
#[fixture]
pub fn upper_panel_only() -> Sys {
    Sys::from_fixture("ux8406ca-upper-panel-only")
}

/// A tree that has no file at all.
#[fixture]
pub fn empty() -> Sys {
    Sys::empty()
}

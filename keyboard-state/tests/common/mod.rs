//! A stand-in for the BPF file system and the state file that every test
//! program of this crate shares.
#![allow(
    dead_code,
    reason = "each test program uses a different part of the shared helpers"
)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use asus_ux8406_keyboard_state::pins::STATE_MAP;
use asus_ux8406_keyboard_state::state::MapValue;
use asus_ux8406_keyboard_state::{Bpffs, Device, Error, MapReader};
use rstest::fixture;

/// The keyboard's HID device with the program, on the dock.
pub const USB: &str = "0003:0B05:1BF2.005D";
/// A HID device of the keyboard on the dock that has no program.
pub const USB_OTHER_INTERFACE: &str = "0003:0B05:1BF2.005E";
/// The keyboard's HID device with the program, over Bluetooth.
pub const BLUETOOTH: &str = "0005:0B05:1BF3.0060";
/// A HID device of something else that has a program of udev-hid-bpf.
pub const TABLET: &str = "0003:056A:0357.0007";
/// The directory udev-hid-bpf names after the program's object file.
pub const OBJECT: &str = "0010-ASUS__Zenbook-Duo-UX8406-Keyboard_bpf";
/// What a state file holds for the brightest backlight and the hotkeys.
pub const BRIGHT_HOTKEYS: &str = "ASUS_UX8406_KBD_BACKLIGHT=3\nASUS_UX8406_KBD_FN_LOCK=0\n";

/// A device of the keyboard from its name.
pub fn device(name: &str) -> Device {
    name.parse().unwrap()
}

/// A temporary directory with a tree like `/sys/fs/bpf` and a place for the
/// state file, removed again when it goes out of scope.
pub struct Tree {
    dir: TempDir,
}

impl Tree {
    /// The directory that stands in for `/sys/fs/bpf`.
    pub fn bpffs_root(&self) -> PathBuf {
        self.dir.path().join("bpf")
    }

    /// The tree as the library takes it.
    pub fn bpffs(&self) -> Bpffs {
        Bpffs::new(&self.bpffs_root())
    }

    /// The state file. It is not there until something writes it.
    pub fn state_file(&self) -> PathBuf {
        self.dir.path().join("state")
    }

    /// Where udev-hid-bpf would pin the state map of the HID device `name`.
    pub fn pin_path(&self, name: &str) -> PathBuf {
        self.bpffs_root()
            .join("hid")
            .join(name.replace([':', '.'], "_"))
            .join(OBJECT)
            .join(STATE_MAP)
    }

    /// Put a file with `value` where the state map of the HID device `name`
    /// would be pinned. [`FileMaps`] reads it as the map; the kernel does not
    /// take it for one.
    pub fn pin(&self, name: &str, value: &[u8]) -> PathBuf {
        let path = self.pin_path(name);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, value).unwrap();
        path
    }

    /// Give the HID device `name` a program of udev-hid-bpf without the
    /// state map.
    pub fn pin_other_program(&self, name: &str) {
        let path = self.pin_path(name).with_file_name("some_other_map");

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, [0]).unwrap();
    }

    /// Write the state file.
    pub fn write_state(&self, text: &str) {
        fs::write(self.state_file(), text).unwrap();
    }

    /// The text of the state file, if it is there.
    pub fn state(&self) -> Option<String> {
        fs::read_to_string(self.state_file()).ok()
    }

    /// The program, set to run on this tree.
    pub fn program(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asus-ux8406-keyboard-state"));

        command
            .arg("-b")
            .arg(self.bpffs_root())
            .arg("-s")
            .arg(self.state_file());
        command
    }
}

/// A tree without any pin and without a state file.
#[fixture]
pub fn tree() -> Tree {
    Tree {
        dir: TempDir::new().unwrap(),
    }
}

/// Reads the files that [`Tree::pin`] writes in place of maps.
pub struct FileMaps;

impl MapReader for FileMaps {
    fn read(&self, map: &Path) -> Result<MapValue, Error> {
        let failed = |reason: &str| Error::Map {
            path: map.to_path_buf(),
            reason: reason.to_owned(),
        };
        let bytes = fs::read(map).map_err(|error| failed(&error.to_string()))?;

        bytes
            .try_into()
            .map_err(|_| failed("not the size of a state"))
    }
}

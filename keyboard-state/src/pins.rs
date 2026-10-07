//! The maps that udev-hid-bpf pins for the keyboard's program.
//!
//! udev-hid-bpf pins what it loads for a HID device in
//! `hid/<device>/<object>/` of the BPF file system, and removes the
//! directory again when the device goes.

use std::fs;
use std::path::{Path, PathBuf};

use aya::maps::{Array, Map, MapData};

use crate::state::MapValue;
use crate::{Device, Error};

/// The name of the map with the state, as the program declares it.
pub const STATE_MAP: &str = "zenbook_duo_kbd_state";

/// The BPF file system, or a directory that stands in for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bpffs {
    root: PathBuf,
}

impl Bpffs {
    /// The file system mounted at `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// The pin of the state map of `device`, if the device has the program.
    #[must_use]
    pub fn map_of(&self, device: &Device) -> Option<PathBuf> {
        state_map_in(&self.root.join("hid").join(device.pin_name()))
    }

    /// The pins of the state maps of the keyboard's other devices.
    #[must_use]
    pub fn maps_of_others(&self, device: &Device) -> Vec<PathBuf> {
        let mut maps: Vec<PathBuf> = directories(&self.root.join("hid"))
            .filter(|directory| {
                directory
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(Device::from_pin_name)
                    .is_some_and(|other| other != *device)
            })
            .filter_map(|directory| state_map_in(&directory))
            .collect();

        maps.sort();
        maps
    }
}

/// The directories in `directory`; none if it cannot be read.
fn directories(directory: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
}

/// The state map among the pins of one device. The directory between is
/// named after the object file, which this program does not need to know.
fn state_map_in(device: &Path) -> Option<PathBuf> {
    let mut maps: Vec<PathBuf> = directories(device)
        .map(|object| object.join(STATE_MAP))
        .filter(|map| map.exists())
        .collect();

    maps.sort();
    maps.into_iter().next()
}

/// A way to read the value of a state map.
pub trait MapReader {
    /// The value of the one entry of the map pinned at `map`.
    ///
    /// # Errors
    ///
    /// If the pin is not a map of the program's kind, or cannot be read.
    fn read(&self, map: &Path) -> Result<MapValue, Error>;
}

/// The maps of the kernel, read with the `bpf` system call. Needs root.
#[derive(Clone, Copy, Debug, Default)]
pub struct PinnedMap;

impl MapReader for PinnedMap {
    fn read(&self, map: &Path) -> Result<MapValue, Error> {
        let failed = |reason: String| Error::Map {
            path: map.to_path_buf(),
            reason,
        };
        let data = MapData::from_pin(map).map_err(|error| failed(error.to_string()))?;
        let array: Array<MapData, MapValue> =
            Array::try_from(Map::Array(data)).map_err(|error| failed(error.to_string()))?;

        array.get(&0, 0).map_err(|error| failed(error.to_string()))
    }
}

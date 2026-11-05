//! The state map as the kernel keeps it: a real BPF array map, pinned on the
//! BPF file system and read with `PinnedMap`. Creating a BPF map and pinning it
//! needs root, and a mounted BPF file system; without them the tests say why
//! they do nothing, and pass. Run them as root (refer to CONTRIBUTING.md) so
//! that they count.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use asus_ux8406_keyboard_state::state::MapValue;
use asus_ux8406_keyboard_state::{Error, MapReader, PinnedMap};
use aya::maps::{Array, Map, MapData};
use aya_obj::maps::{LegacyMap, bpf_map_def};
use aya_obj::{EbpfSectionKind, Map as MapDescription};

/// Where the kernel's BPF file system is mounted.
const BPFFS: &str = "/sys/fs/bpf";
/// `BPF_MAP_TYPE_ARRAY`.
const ARRAY: u32 = 2;

/// A directory on the BPF file system, removed with its pins at the end.
struct Pins(PathBuf);

impl Pins {
    fn new() -> Self {
        let directory = Path::new(BPFFS).join(format!("asus-ux8406-test-{}", std::process::id()));

        fs::create_dir(&directory).expect("a directory on the BPF file system");
        Self(directory)
    }

    fn pin(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Pins {
    fn drop(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_dir(&self.0);
    }
}

/// Why the tests do nothing here, if they cannot run.
fn cannot_run() -> Option<String> {
    if !fs::metadata("/proc/self").is_ok_and(|own| own.uid() == 0) {
        return Some("not root: a BPF map can be made only by root".to_owned());
    }
    let mounted = fs::read_to_string("/proc/mounts").is_ok_and(|mounts| {
        mounts
            .lines()
            .any(|line| line.split(' ').nth(1) == Some(BPFFS))
    });

    if !mounted {
        return Some(format!("{BPFFS} is not a mounted BPF file system"));
    }
    None
}

/// An array map with one entry of `value_size` bytes.
fn array(value_size: u32) -> MapData {
    let description = MapDescription::Legacy(LegacyMap {
        def: bpf_map_def {
            map_type: ARRAY,
            key_size: 4,
            value_size,
            max_entries: 1,
            ..bpf_map_def::default()
        },
        section_index: 0,
        section_kind: EbpfSectionKind::Maps,
        symbol_index: None,
        data: Vec::new(),
    });

    MapData::create(description, "asus_ux8406_t", None).expect("a BPF map")
}

#[test]
fn a_pinned_array_map_gives_its_value() {
    if let Some(reason) = cannot_run() {
        eprintln!("skipped, {reason}");
        return;
    }

    let pins = Pins::new();
    let pin = pins.pin("state");
    let data = array(2);

    data.pin(&pin).expect("a pin");

    let mut map: Array<MapData, MapValue> = Array::try_from(Map::Array(data)).expect("an array");

    map.set(0, [1, 3], 0).expect("a value");
    assert_eq!(PinnedMap.read(&pin).unwrap(), [1, 3]);
}

#[test]
fn a_pinned_map_with_another_value_size_is_refused() {
    if let Some(reason) = cannot_run() {
        eprintln!("skipped, {reason}");
        return;
    }

    let pins = Pins::new();
    let pin = pins.pin("wrong");

    array(4).pin(&pin).expect("a pin");

    let error = PinnedMap.read(&pin).unwrap_err();

    assert!(matches!(error, Error::Map { path, .. } if path == pin));
}

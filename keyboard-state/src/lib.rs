//! The backlight and the function row of the detachable keyboard of the ASUS
//! Zenbook Duo UX8406, kept from one connection to the next.
//!
//! The keyboard is a different HID device on the dock, on the cable and over
//! Bluetooth, and forgets its backlight and the mode of its function row when
//! it changes from one to the other. The HID-BPF program of the keyboard
//! holds both for the connection it is loaded for, in a map that goes away
//! with the connection, and takes its start values from two udev properties
//! of the HID device.
//!
//! [`save`] writes the values of a connection that ends to a file, and
//! [`load`] gives the values for a connection that starts: those of another
//! connection of the keyboard that is still there, else those in the file.
//! udev runs the one when a HID device of the keyboard goes and the other
//! when one comes.

pub mod cli;
pub mod device;
mod error;
pub mod pins;
pub mod state;

use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub use cli::{Action, Cli};
pub use device::Device;
pub use error::Error;
pub use pins::{Bpffs, MapReader, PinnedMap};
pub use state::State;
use tempfile::NamedTempFile;

/// Write the state that the program of `device` holds to `file`.
///
/// Most HID devices of the keyboard have no program, and so no state: for
/// those nothing is written, and the result is `None`.
///
/// # Errors
///
/// If the map cannot be read or does not hold a state, or if the file cannot
/// be written. The file is then as it was.
pub fn save(
    bpffs: &Bpffs,
    reader: &dyn MapReader,
    device: &Device,
    file: &Path,
) -> Result<Option<State>, Error> {
    let Some(map) = bpffs.map_of(device) else {
        return Ok(None);
    };
    let state = State::from_map(&map, reader.read(&map)?)?;

    write(file, &state.properties())?;
    Ok(Some(state))
}

/// The state for the connection `device`, which is about to get its program.
///
/// A connection that ends is not always gone before the next one is there.
/// The state of another connection that still has its program is therefore
/// taken first; what was written to `file` is for when there is none, or
/// when it went away while it was read.
///
/// # Errors
///
/// If the file is there and cannot be read or does not hold a state.
pub fn load(
    bpffs: &Bpffs,
    reader: &dyn MapReader,
    device: &Device,
    file: &Path,
) -> Result<Option<State>, Error> {
    let live = bpffs.maps_of_others(device).into_iter().find_map(|map| {
        let values = reader.read(&map).ok()?;

        State::from_map(&map, values).ok()
    });

    if live.is_some() {
        return Ok(live);
    }
    match fs::read_to_string(file) {
        Ok(text) => State::from_properties(file, &text).map(Some),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::io(file, source)),
    }
}

/// Replace `file` by one with `text`, in one step: a reader sees the old text
/// or the new one, never a part.
fn write(file: &Path, text: &str) -> Result<(), Error> {
    let directory = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut new = NamedTempFile::new_in(directory).map_err(|source| Error::io(file, source))?;

    new.write_all(text.as_bytes())
        .map_err(|source| Error::io(file, source))?;
    new.persist(file)
        .map_err(|error| Error::io(file, error.error))?;
    Ok(())
}

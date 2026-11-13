//! The backlight while the laptop sleeps.
//!
//! The keyboard keeps its backlight on when the laptop suspends. Before the
//! sleep, [`sleep`] switches it off. After the sleep, it sets the level that
//! the program of the HID device holds. The map of the program is not
//! changed: [`crate::save`] writes the true level if the keyboard goes while
//! the laptop sleeps, and the next connection starts with it.
//!
//! The report is the one that the HID-BPF program sends. It goes to the
//! hidraw node of the HID device, which the program cannot do for a laptop
//! that is about to sleep.

use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use hidapi::HidApi;

use crate::{Bpffs, Device, Error, MapReader, State};

/// The size of the report: the report ID and fifteen bytes.
pub const REPORT_SIZE: usize = 16;
/// The ID of the report.
pub const REPORT_ID: u8 = 0x5a;
/// The command byte: set the backlight.
pub const BACKLIGHT_COMMAND: u8 = 0xba;
/// The first parameter of the command.
pub const BACKLIGHT_PARAMETER_1: u8 = 0xc5;
/// The second parameter of the command.
pub const BACKLIGHT_PARAMETER_2: u8 = 0xc4;
/// The place of the level in the report.
pub const LEVEL_INDEX: usize = 4;

/// The time of a sleep that the system gives to a hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Phase {
    /// The system is about to sleep.
    Pre,
    /// The system has woken up.
    Post,
}

/// The report that sets the backlight to `level`.
#[must_use]
pub fn report(level: u8) -> [u8; REPORT_SIZE] {
    let mut report = [0; REPORT_SIZE];

    report[0] = REPORT_ID;
    report[1] = BACKLIGHT_COMMAND;
    report[2] = BACKLIGHT_PARAMETER_1;
    report[3] = BACKLIGHT_PARAMETER_2;
    report[LEVEL_INDEX] = level;
    report
}

/// A way to send a feature report to a HID device.
pub trait ReportSender {
    /// Send `report` as a feature report to the hidraw node `hidraw`.
    ///
    /// # Errors
    ///
    /// The reason, if the node cannot be opened or does not take the report.
    fn send(&self, hidraw: &Path, report: &[u8]) -> Result<(), String>;
}

/// The hidraw nodes of the kernel, with the `hidapi` crate.
#[derive(Clone, Copy, Debug, Default)]
pub struct HidrawSender;

impl ReportSender for HidrawSender {
    fn send(&self, hidraw: &Path, report: &[u8]) -> Result<(), String> {
        let path =
            CString::new(hidraw.as_os_str().as_bytes()).map_err(|error| error.to_string())?;

        HidApi::new()
            .and_then(|api| api.open_path(&path))
            .and_then(|device| device.send_feature_report(report))
            .map_err(|error| error.to_string())
    }
}

/// The hidraw node of `device`: `hidraw/hidrawN` of the device in `sysfs`,
/// and the node of that name in `dev`.
fn hidraw_of(sysfs: &Path, dev: &Path, device: &Device) -> Result<PathBuf, Error> {
    let directory = sysfs
        .join("bus/hid/devices")
        .join(device.to_string())
        .join("hidraw");
    let mut names: Vec<_> = fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();

    names.sort();
    names
        .into_iter()
        .next()
        .map(|name| dev.join(name))
        .ok_or_else(|| Error::NoHidraw(device.clone()))
}

/// Set the backlight of one device for `phase`.
fn set(
    reader: &dyn MapReader,
    sender: &dyn ReportSender,
    (device, map): &(Device, PathBuf),
    hidraw: &Path,
    phase: Phase,
) -> Result<(), Error> {
    let level = match phase {
        Phase::Pre => 0,
        Phase::Post => State::from_map(map, reader.read(map)?)?.backlight,
    };

    sender
        .send(hidraw, &report(level))
        .map_err(|reason| Error::Send {
            device: device.clone(),
            reason,
        })
}

/// Set the backlight of every HID device of the keyboard that has the
/// program, for `phase`: off before the sleep, and the level that the map of
/// the device holds after it.
///
/// All devices are tried, also after one failed.
///
/// # Errors
///
/// The first error, after the last device: a device without a hidraw node, a
/// map that cannot be read or does not hold a state, or a report that cannot
/// be sent.
pub fn sleep(
    bpffs: &Bpffs,
    reader: &dyn MapReader,
    sender: &dyn ReportSender,
    sysfs: &Path,
    dev: &Path,
    phase: Phase,
) -> Result<(), Error> {
    let mut first = None;

    for keyboard in bpffs.keyboards() {
        let result = hidraw_of(sysfs, dev, &keyboard.0)
            .and_then(|hidraw| set(reader, sender, &keyboard, &hidraw, phase));

        if let Err(error) = result {
            first.get_or_insert(error);
        }
    }
    first.map_or(Ok(()), Err)
}

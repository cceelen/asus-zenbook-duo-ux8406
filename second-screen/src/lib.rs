//! Whether the lower built-in panel of the ASUS Zenbook Duo UX8406CA counts
//! as connected.
//!
//! While the detachable keyboard lies on the lower panel, the panel's DRM
//! connector is forced to "disconnected"; otherwise it is left to normal
//! detection. The desktop then sees a monitor that was unplugged or plugged
//! in and uses the layout it has saved for the monitors that are left, as it
//! does for any other monitor.
//!
//! [`sync_panel`] looks at the keyboard's dock port and writes the matching
//! state.
//!
//! The desktop sets the brightness of the upper panel only. [`sync_brightness`]
//! gives the lower panel the brightness the upper one has.
//!
//! Neither keeps anything between calls, so both can be run on every event
//! that may have changed something.

pub mod cli;
pub mod dock;
mod error;
pub mod sysfs;

use std::path::PathBuf;

pub use cli::{Action, Cli};
pub use dock::Panel;
pub use error::Error;
pub use sysfs::{Backlight, Detection, Sysfs};

/// Whether to write, or only to say what would be written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Write the detection to the connector.
    Apply,
    /// Write nothing.
    DryRun,
}

/// What [`sync_panel`] decided, and where it applies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The detection the dock state asks for.
    pub detection: Detection,
    /// The `status` attributes it was, or would be, written to.
    pub status_files: Vec<PathBuf>,
}

/// Bring the lower panel's connector in line with where the keyboard is.
///
/// # Errors
///
/// [`Error::UnsupportedModel`] on any machine but a UX8406,
/// [`Error::NoLowerPanel`] if the connector is missing, and [`Error::Io`]
/// if it cannot be found or written. Nothing is written in the first two
/// cases.
pub fn sync_panel(sysfs: &Sysfs, mode: Mode) -> Result<Outcome, Error> {
    if !sysfs.is_supported_model() {
        return Err(Error::UnsupportedModel);
    }

    let outcome = Outcome {
        detection: Detection::for_keyboard(sysfs.keyboard_docked()),
        status_files: sysfs.lower_panel_status_files()?,
    };

    if mode == Mode::Apply {
        for status in &outcome.status_files {
            sysfs::write_detection(status, outcome.detection)?;
        }
    }
    Ok(outcome)
}

/// Hand the lower panel's connector back to normal detection, wherever the
/// keyboard is.
///
/// For removal of this program: a connector left forced off stays off until
/// the next boot.
///
/// # Errors
///
/// As [`sync_panel`].
pub fn release_panel(sysfs: &Sysfs, mode: Mode) -> Result<Outcome, Error> {
    if !sysfs.is_supported_model() {
        return Err(Error::UnsupportedModel);
    }

    let outcome = Outcome {
        detection: Detection::Automatic,
        status_files: sysfs.lower_panel_status_files()?,
    };

    if mode == Mode::Apply {
        for status in &outcome.status_files {
            sysfs::write_detection(status, outcome.detection)?;
        }
    }
    Ok(outcome)
}

/// What [`sync_brightness`] decided, and where it applies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brightness {
    /// The brightness the lower panel is, or would be, given.
    pub value: u32,
    /// The attribute it is written to.
    pub file: PathBuf,
}

/// Give the lower panel the brightness of the upper one.
///
/// The value is carried over on the lower panel's own scale, should the two
/// differ.
///
/// # Errors
///
/// [`Error::UnsupportedModel`] on any machine but a UX8406,
/// [`Error::NoBacklight`] if a panel has no backlight device,
/// [`Error::BadBrightness`] if a brightness cannot be read or is off its
/// scale, and [`Error::Io`] if the lower panel's cannot be written.
pub fn sync_brightness(sysfs: &Sysfs, mode: Mode) -> Result<Brightness, Error> {
    if !sysfs.is_supported_model() {
        return Err(Error::UnsupportedModel);
    }

    let upper = sysfs.backlight(Panel::Upper)?;
    let lower = sysfs.backlight(Panel::Lower)?;
    let value = dock::scale_brightness(
        upper.brightness()?,
        upper.max_brightness()?,
        lower.max_brightness()?,
    )
    .ok_or_else(|| Error::BadBrightness(upper.brightness_file()))?;

    if mode == Mode::Apply {
        lower.set_brightness(value)?;
    }
    Ok(Brightness {
        value,
        file: lower.brightness_file(),
    })
}

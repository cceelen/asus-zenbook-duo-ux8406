//! The configuration file: TOML, read from a file that only the
//! administrator can change. Every setting is optional; what the file names
//! has to be known and within its range.

use std::path::Path;

use serde::Deserialize;

use crate::error::Error;
use crate::input::{Number, Rejected, Settings};
use crate::trusted;

/// A configuration file is at most this big.
const SIZE_MAX: usize = 16_384;

/// The file as it is written.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    hold_temp: Option<i64>,
    step_up: Option<i64>,
    max_offset: Option<i64>,
    interval: Option<i64>,
    settle: Option<i64>,
    probe_interval: Option<i64>,
    probe_backoff: Option<i64>,
    probe_max: Option<i64>,
    tjmax: Option<i64>,
    use_margin: Option<i64>,
    idle_release: Option<i64>,
    event_text: Option<String>,
    model: Option<String>,
    #[serde(default)]
    package: Vec<Package>,
}

/// Where the offset of one processor package is written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    id: i64,
    offset: String,
}

impl File {
    const fn numbers(&self) -> [(Number, Option<i64>); 11] {
        [
            (Number::HoldTemp, self.hold_temp),
            (Number::StepUp, self.step_up),
            (Number::MaxOffset, self.max_offset),
            (Number::Interval, self.interval),
            (Number::Settle, self.settle),
            (Number::ProbeInterval, self.probe_interval),
            (Number::ProbeBackoff, self.probe_backoff),
            (Number::ProbeMax, self.probe_max),
            (Number::Tjmax, self.tjmax),
            (Number::UseMargin, self.use_margin),
            (Number::IdleRelease, self.idle_release),
        ]
    }
}

/// Apply the text of a configuration file to the settings.
///
/// # Errors
///
/// Why the text is not acceptable: not TOML, a name that is not a setting,
/// a name given twice, a value of the wrong kind or outside its range.
pub fn apply(settings: &mut Settings, text: &str) -> Result<(), Rejected> {
    let file: File = toml::from_str(text).map_err(|error| Rejected(error.to_string()))?;

    for (number, value) in file.numbers() {
        if let Some(value) = value {
            number.set(settings, value)?;
        }
    }
    if let Some(event_text) = &file.event_text {
        settings.set_event_text(event_text)?;
    }
    if let Some(model) = &file.model {
        settings.set_model(model)?;
    }
    for package in &file.package {
        settings.add_control(package.id, &package.offset)?;
    }
    Ok(())
}

/// Read a configuration file into the settings.
///
/// # Errors
///
/// [`Error::Io`] if it cannot be read, [`Error::NotTrusted`] if it is not a
/// file that only the administrator can change, and [`Error::BadConfig`] if
/// what it holds is not acceptable.
pub fn load(settings: &mut Settings, path: &Path) -> Result<(), Error> {
    let rejected = |reason: String| Error::BadConfig {
        path: path.to_owned(),
        reason,
    };
    let bytes = trusted::read(path, SIZE_MAX)?;
    let text = std::str::from_utf8(&bytes).map_err(|error| rejected(error.to_string()))?;

    apply(settings, text).map_err(|Rejected(reason)| rejected(reason))
}

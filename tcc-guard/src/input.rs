//! Everything asus-ux8406-tcc-guard takes from outside, as text or values in and checked
//! values out: numbers, settings, kernel log records and sysfs values.
//! The program runs as root, so nothing here is lenient: what is not exactly
//! the expected form, or not within the stated range, is rejected.
//!
//! No I/O happens here, so that every rule can be tested with plain strings.

use crate::guard;

/// The most processor packages one guard looks after.
pub const MAX_PACKAGES: usize = 8;
/// A file name below `/sys` is shorter than this.
const PATH_SIZE: usize = 160;
/// A text setting is shorter than this.
const TEXT_SIZE: usize = 96;

/// Ranges of what sysfs may report; anything else is treated as a failure.
const TEMP_MILLI: (i32, i32) = (-40_000, 150_000);
const TJMAX: (i32, i32) = (70, 130);
/// The highest offset the register holds.
pub const OFFSET_MAX: i32 = 63;
const PACKAGE_MAX: i32 = 1023;

/// A number that fits an `i32` has at most this many digits.
const MAX_DIGITS: usize = 9;
/// sysfs reports millidegrees.
const MILLI: i32 = 1000;
/// A kernel log record starts with its priority: the facility times eight
/// plus the level, at most five digits.
const PRIORITY_DIGITS: usize = 5;
const FACILITY_SHIFT: u32 = 3;

const SYSFS: &str = "/sys";
const LABEL: &str = "Package id ";
/// The kernel log message that announces the EC's warning on the UX8406CA.
const EVENT_TEXT: &str = "asus_wmi: Unknown key code 0x6d";
/// The machine the defaults were found on, as its DMI product name has it.
const MODEL: &str = "UX8406";
/// The value of `model` that lets the guard run on any machine.
pub const ANY_MODEL: &str = "any";

/// Defaults, found on an ASUS Zenbook Duo UX8406CA; every value can be set
/// in the configuration file.
const GUARD_DEFAULTS: guard::Config = guard::Config {
    hold_temp: 87,
    step_up: 2,
    max_offset: 30,
    interval: 4,
    settle: 20,
    // The EC raises the event about 120 s after its alert switches on.
    probe_interval: 180,
    probe_backoff: 2,
    probe_max: 3600,
    use_margin: 5,
    // Long enough to bridge the pauses of ordinary work.
    idle_release: 900,
};

/// A setting that is not acceptable, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejected(pub String);

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where the thermal offset of one package is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    /// The package's number.
    pub package: i32,
    /// The file, below `/sys` and without that prefix.
    pub path: String,
}

/// Everything that can be configured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The settings of the control logic.
    pub guard: guard::Config,
    /// The limit of all packages at offset 0; 0 = ask the kernel for each.
    pub tjmax: i32,
    /// How the kernel log message of the event starts.
    pub event_text: String,
    /// What the DMI product name has to contain, or [`ANY_MODEL`].
    pub model: String,
    /// Where the offsets are written, for machines with several packages.
    pub controls: Vec<Control>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            guard: GUARD_DEFAULTS,
            tjmax: 0,
            event_text: EVENT_TEXT.to_owned(),
            model: MODEL.to_owned(),
            controls: Vec::new(),
        }
    }
}

/// A numeric setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Number {
    /// `hold_temp`
    HoldTemp,
    /// `step_up`
    StepUp,
    /// `max_offset`
    MaxOffset,
    /// `interval`
    Interval,
    /// `settle`
    Settle,
    /// `probe_interval`
    ProbeInterval,
    /// `probe_backoff`
    ProbeBackoff,
    /// `probe_max`
    ProbeMax,
    /// `tjmax`
    Tjmax,
    /// `use_margin`
    UseMargin,
    /// `idle_release`
    IdleRelease,
}

impl Number {
    /// Every numeric setting.
    pub const ALL: [Self; 11] = [
        Self::HoldTemp,
        Self::StepUp,
        Self::MaxOffset,
        Self::Interval,
        Self::Settle,
        Self::ProbeInterval,
        Self::ProbeBackoff,
        Self::ProbeMax,
        Self::Tjmax,
        Self::UseMargin,
        Self::IdleRelease,
    ];

    /// The setting's name in the configuration file.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::HoldTemp => "hold_temp",
            Self::StepUp => "step_up",
            Self::MaxOffset => "max_offset",
            Self::Interval => "interval",
            Self::Settle => "settle",
            Self::ProbeInterval => "probe_interval",
            Self::ProbeBackoff => "probe_backoff",
            Self::ProbeMax => "probe_max",
            Self::Tjmax => "tjmax",
            Self::UseMargin => "use_margin",
            Self::IdleRelease => "idle_release",
        }
    }

    /// The range the setting must lie in.
    #[must_use]
    pub const fn range(self) -> (i32, i32) {
        match self {
            Self::HoldTemp => (50, 105),
            Self::StepUp => (1, 10),
            Self::MaxOffset => (1, OFFSET_MAX),
            Self::Interval => (1, 60),
            Self::Settle => (0, 600),
            Self::ProbeInterval => (1, 86_400),
            Self::ProbeBackoff => (1, 16),
            Self::ProbeMax => (1, 604_800),
            Self::Tjmax => (0, TJMAX.1),
            Self::UseMargin => (1, 40),
            Self::IdleRelease => (0, 86_400),
        }
    }

    /// The numeric setting of that name.
    #[must_use]
    pub fn find(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|number| number.name() == name)
    }

    /// The setting's value.
    #[must_use]
    pub const fn get(self, settings: &Settings) -> i32 {
        match self {
            Self::HoldTemp => settings.guard.hold_temp,
            Self::StepUp => settings.guard.step_up,
            Self::MaxOffset => settings.guard.max_offset,
            Self::Interval => settings.guard.interval,
            Self::Settle => settings.guard.settle,
            Self::ProbeInterval => settings.guard.probe_interval,
            Self::ProbeBackoff => settings.guard.probe_backoff,
            Self::ProbeMax => settings.guard.probe_max,
            Self::Tjmax => settings.tjmax,
            Self::UseMargin => settings.guard.use_margin,
            Self::IdleRelease => settings.guard.idle_release,
        }
    }

    const fn slot(self, settings: &mut Settings) -> &mut i32 {
        match self {
            Self::HoldTemp => &mut settings.guard.hold_temp,
            Self::StepUp => &mut settings.guard.step_up,
            Self::MaxOffset => &mut settings.guard.max_offset,
            Self::Interval => &mut settings.guard.interval,
            Self::Settle => &mut settings.guard.settle,
            Self::ProbeInterval => &mut settings.guard.probe_interval,
            Self::ProbeBackoff => &mut settings.guard.probe_backoff,
            Self::ProbeMax => &mut settings.guard.probe_max,
            Self::Tjmax => &mut settings.tjmax,
            Self::UseMargin => &mut settings.guard.use_margin,
            Self::IdleRelease => &mut settings.guard.idle_release,
        }
    }

    /// Give the setting a value.
    ///
    /// # Errors
    ///
    /// [`Rejected`] if the value is not within the setting's range.
    pub fn set(self, settings: &mut Settings, value: i64) -> Result<(), Rejected> {
        let (min, max) = self.range();

        *self.slot(settings) = i32::try_from(value)
            .ok()
            .filter(|value| (min..=max).contains(value))
            .ok_or_else(|| Rejected(format!("{}: has to be from {min} to {max}", self.name())))?;
        Ok(())
    }
}

/// A whole decimal number within `[min, max]`: an optional minus sign and one
/// to nine digits, and nothing else. No white space, no plus sign, no suffix.
#[must_use]
pub fn parse_int(text: &str, min: i32, max: i32) -> Option<i32> {
    let digits = text.strip_prefix('-').unwrap_or(text);

    if digits.is_empty() || digits.len() > MAX_DIGITS || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    // Nine digits and a sign always fit.
    let magnitude = digits
        .bytes()
        .fold(0_i32, |value, digit| value * 10 + i32::from(digit - b'0'));
    let value = if digits.len() < text.len() {
        -magnitude
    } else {
        magnitude
    };

    (min..=max).contains(&value).then_some(value)
}

/// A sysfs value: one number, followed by nothing but newlines.
fn parse_value(text: &str, min: i32, max: i32) -> Option<i32> {
    parse_int(text.trim_end_matches('\n'), min, max)
}

/// A temperature in millidegrees, as sysfs reports it; the result in degrees.
#[must_use]
pub fn parse_temp(text: &str) -> Option<i32> {
    parse_value(text, TEMP_MILLI.0, TEMP_MILLI.1).map(|milli| milli / MILLI)
}

/// The limit of a package at offset 0, in millidegrees; the result in
/// degrees.
#[must_use]
pub fn parse_tjmax(text: &str) -> Option<i32> {
    parse_value(text, TJMAX.0 * MILLI, TJMAX.1 * MILLI).map(|milli| milli / MILLI)
}

/// A thermal offset as sysfs reports it.
#[must_use]
pub fn parse_offset(text: &str) -> Option<i32> {
    parse_value(text, 0, OFFSET_MAX)
}

/// The label of a package temperature sensor: "Package id N".
#[must_use]
pub fn parse_package_label(text: &str) -> Option<i32> {
    parse_value(text.strip_prefix(LABEL)?, 0, PACKAGE_MAX)
}

/// Whether a kernel log record, as `/dev/kmsg` delivers it, is the event.
///
/// It must come from the kernel itself (facility 0; records written by user
/// space carry another facility) and its message must start with the event
/// text. Text in the lines that follow the message does not count.
#[must_use]
pub fn is_event(event_text: &str, record: &str) -> bool {
    let digits = record.bytes().take_while(u8::is_ascii_digit).count();

    if digits == 0 || digits > PRIORITY_DIGITS || !record[digits..].starts_with(',') {
        return false;
    }
    let from_kernel = record[..digits]
        .parse::<u32>()
        .is_ok_and(|priority| priority >> FACILITY_SHIFT == 0);
    let message = record[digits..].split_once(';').map(|(_, message)| message);

    from_kernel
        && !event_text.is_empty()
        && message.is_some_and(|message| message.starts_with(event_text))
}

/// A path below `/sys` made of plain characters, without "..".
fn path_ok(path: &str) -> bool {
    const ALLOWED: &str = "_-.:/";

    path.strip_prefix(SYSFS).is_some_and(|below| {
        below.len() > 1
            && below.len() < PATH_SIZE
            && below.starts_with('/')
            && below
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ALLOWED.contains(c))
            && !below.contains("..")
    })
}

/// A text setting: not empty, short, and nothing but printable ASCII.
fn text_ok(value: &str) -> bool {
    !value.is_empty() && value.len() < TEXT_SIZE && value.chars().all(|c| (' '..='~').contains(&c))
}

fn text(name: &str, value: &str) -> Result<String, Rejected> {
    if text_ok(value) {
        Ok(value.to_owned())
    } else {
        Err(Rejected(format!(
            "{name}: has to be 1 to {} printable ASCII characters",
            TEXT_SIZE - 1
        )))
    }
}

impl Settings {
    /// Say where the offset of a package is written: a file below `/sys`.
    ///
    /// # Errors
    ///
    /// [`Rejected`] if the package's number is not one, the file is not a
    /// plain path below `/sys`, the package has a file already, or there
    /// are more packages than one guard looks after.
    pub fn add_control(&mut self, package: i64, path: &str) -> Result<(), Rejected> {
        let refuse = |why: &str| Rejected(format!("package {package}: {why}"));
        let id = i32::try_from(package)
            .ok()
            .filter(|id| (0..=PACKAGE_MAX).contains(id))
            .ok_or_else(|| refuse(&format!("its id has to be from 0 to {PACKAGE_MAX}")))?;

        if !path_ok(path) {
            return Err(refuse(
                "its offset has to be a file below /sys, named in plain characters",
            ));
        }
        if self.controls.iter().any(|control| control.package == id) {
            return Err(refuse("is named twice"));
        }
        if self.controls.len() >= MAX_PACKAGES {
            return Err(refuse(&format!("more than {MAX_PACKAGES} packages")));
        }
        self.controls.push(Control {
            package: id,
            path: path[SYSFS.len()..].to_owned(),
        });
        Ok(())
    }

    /// Set how the kernel log message of the event starts.
    ///
    /// # Errors
    ///
    /// [`Rejected`] if the text is empty, too long, or not plain.
    pub fn set_event_text(&mut self, value: &str) -> Result<(), Rejected> {
        self.event_text = text("event_text", value)?;
        Ok(())
    }

    /// Set what the product name of the machine has to contain.
    ///
    /// # Errors
    ///
    /// [`Rejected`] if the text is empty, too long, or not plain.
    pub fn set_model(&mut self, value: &str) -> Result<(), Rejected> {
        self.model = text("model", value)?;
        Ok(())
    }

    /// The rules that involve more than one setting.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.guard.probe_interval >= self.guard.interval
            && self.guard.probe_max >= self.guard.probe_interval
            && (self.tjmax == 0 || self.tjmax >= TJMAX.0)
            && !self.event_text.is_empty()
    }

    /// Whether the guard may run on a machine with this DMI product name.
    #[must_use]
    pub fn model_matches(&self, product_name: &str) -> bool {
        self.model == ANY_MODEL || product_name.contains(&self.model)
    }
}

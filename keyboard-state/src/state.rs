//! What the keyboard is asked for: the level of its backlight and the mode of
//! its function row.

use std::path::Path;

use crate::Error;

/// The udev property that the program takes the backlight level from.
pub const BACKLIGHT_PROPERTY: &str = "ASUS_UX8406_KBD_BACKLIGHT";
/// The udev property that the program takes the function row's mode from.
pub const FN_LOCK_PROPERTY: &str = "ASUS_UX8406_KBD_FN_LOCK";
/// The brightest backlight level; 0 is off.
pub const MAX_BACKLIGHT: u8 = 3;
/// The function row gives F1 to F12; with 0 it gives the hotkeys.
pub const MAX_FN_LOCK: u8 = 1;

/// The value of the program's map: `struct KbdState` of the program.
pub type MapValue = [u8; 2];

/// The backlight level and the function row's mode of one connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    /// 0 for the hotkeys, 1 for F1 to F12.
    pub fn_lock: u8,
    /// 0 for off, up to 3 for the brightest.
    pub backlight: u8,
}

impl State {
    /// The state in the value of the program's map at `map`.
    ///
    /// # Errors
    ///
    /// If a value is one that the keyboard does not know.
    pub fn from_map(map: &Path, [fn_lock, backlight]: MapValue) -> Result<Self, Error> {
        if fn_lock > MAX_FN_LOCK || backlight > MAX_BACKLIGHT {
            return Err(Error::BadState(map.to_path_buf()));
        }
        Ok(Self { fn_lock, backlight })
    }

    /// The state as udev properties, one `NAME=digit` to a line: what
    /// [`crate::save`] writes and what udev imports.
    #[must_use]
    pub fn properties(&self) -> String {
        format!(
            "{BACKLIGHT_PROPERTY}={}\n{FN_LOCK_PROPERTY}={}\n",
            self.backlight, self.fn_lock
        )
    }

    /// The state in `text`, which was read from `file`.
    ///
    /// # Errors
    ///
    /// If the text is not what [`State::properties`] gives for a state.
    pub fn from_properties(file: &Path, text: &str) -> Result<Self, Error> {
        let bad = || Error::BadState(file.to_path_buf());
        let mut lines = text.lines();
        let mut digit = |property: &str| -> Option<u8> {
            let (name, value) = lines.next()?.split_once('=')?;

            (name == property && value.len() == 1)
                .then(|| value.parse().ok())
                .flatten()
        };
        let backlight = digit(BACKLIGHT_PROPERTY).ok_or_else(bad)?;
        let fn_lock = digit(FN_LOCK_PROPERTY).ok_or_else(bad)?;

        if lines.next().is_some() {
            return Err(bad());
        }
        Self::from_map(file, [fn_lock, backlight])
    }
}

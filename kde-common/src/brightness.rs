//! What to do with the brightness that KWin keeps for each output.

/// The brightness of one output, as KWin tells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    /// Whether the output is on.
    pub enabled: bool,
    /// The brightness from 0 to 10000. None where KWin cannot set the
    /// brightness of the output, or has not told it.
    pub level: Option<u32>,
}

/// The brightness to set on the lower output so that it equals the upper one:
/// none where there is nothing to do. Nothing is to do if an output is
/// missing or off, if one cannot set its brightness, or if they are equal.
///
/// ```
/// use asus_ux8406_kde_common::brightness::{Panel, follow};
///
/// let on = |level| Some(Panel { enabled: true, level: Some(level) });
///
/// assert_eq!(follow(on(3000), on(10000)), Some(3000));
/// assert_eq!(follow(on(3000), on(3000)), None);
/// ```
#[must_use]
pub fn follow(upper: Option<Panel>, lower: Option<Panel>) -> Option<u32> {
    match (upper, lower) {
        (
            Some(Panel {
                enabled: true,
                level: Some(wanted),
            }),
            Some(Panel {
                enabled: true,
                level: Some(level),
            }),
        ) if wanted != level => Some(wanted),
        _ => None,
    }
}

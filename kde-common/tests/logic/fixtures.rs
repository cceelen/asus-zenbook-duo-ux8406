//! The layouts seen on the UX8406CA, as plain data.

use std::collections::BTreeMap;

use asus_ux8406_kde_common::geometry::{Monitor, Size, State};

pub const UPPER: &str = "eDP-1";
pub const LOWER: &str = "eDP-2";

/// The mode of both panels.
pub const PANEL: Size = Size {
    width: 2880,
    height: 1800,
};

/// A logical monitor with one connector, upright.
pub fn logical(connector: &str, x: i32, y: i32, primary: bool, scale: f64) -> Monitor {
    Monitor {
        x,
        y,
        scale,
        transform: 0,
        primary,
        connectors: vec![connector.to_owned()],
    }
}

/// A panel in the layout: scale 1.25.
pub fn panel(connector: &str, x: i32, y: i32, primary: bool) -> Monitor {
    logical(connector, x, y, primary, 1.25)
}

/// A state with the panels built in, the given monitors connected and the layout.
pub fn state(connected: &[(&str, Option<Size>)], logical: Vec<Monitor>) -> State {
    State {
        logical_mode: true,
        connected: connected
            .iter()
            .map(|(connector, mode)| ((*connector).to_owned(), *mode))
            .collect::<BTreeMap<_, _>>(),
        builtin: vec![UPPER.to_owned(), LOWER.to_owned()],
        logical,
    }
}

/// Both panels on: the lower one below the upper one, and the primary.
pub fn two_panels() -> State {
    state(
        &[(UPPER, Some(PANEL)), (LOWER, Some(PANEL))],
        vec![panel(UPPER, 0, 0, false), panel(LOWER, 0, 1440, true)],
    )
}

/// The lower panel switched off; it is still connected.
pub fn lower_off() -> State {
    state(
        &[(UPPER, Some(PANEL)), (LOWER, Some(PANEL))],
        vec![panel(UPPER, 0, 0, true)],
    )
}

/// The keyboard lies on the lower panel: it is not connected.
pub fn docked() -> State {
    state(&[(UPPER, Some(PANEL))], vec![panel(UPPER, 0, 0, true)])
}

/// A size.
pub const fn size(width: i32, height: i32) -> Size {
    Size { width, height }
}

/// The monitor that shows this connector, to change.
pub fn at<'a>(layout: &'a mut [Monitor], connector: &str) -> &'a mut Monitor {
    layout
        .iter_mut()
        .find(|monitor| monitor.connectors.iter().any(|name| name == connector))
        .expect("the connector is in the layout")
}

/// The place and the transform of a monitor.
pub fn place(layout: &[Monitor], connector: &str) -> (i32, i32, u32) {
    let monitor = layout
        .iter()
        .find(|monitor| monitor.connectors.iter().any(|name| name == connector))
        .expect("the connector is in the layout");

    (monitor.x, monitor.y, monitor.transform)
}

//! The display layout as plain data, and what the layout logic does with it.
//!
//! This is `gnome-common/state.js` without the parts for Mutter.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// A size in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    /// The width.
    pub width: i32,
    /// The height.
    pub height: i32,
}

/// A place in the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    /// The left edge.
    pub x: i32,
    /// The top edge.
    pub y: i32,
    /// The width.
    pub width: i32,
    /// The height.
    pub height: i32,
}

/// A logical monitor: one place in the layout, shown on one or more connectors.
#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    /// The left edge in the layout.
    pub x: i32,
    /// The top edge in the layout.
    pub y: i32,
    /// The scale.
    pub scale: f64,
    /// The transform, 0 to 3 turned, 4 to 7 mirrored.
    pub transform: u32,
    /// Whether this is the primary monitor.
    pub primary: bool,
    /// The connectors that show this monitor. More than one means a mirror.
    pub connectors: Vec<String>,
}

/// What the compositor says about the monitors.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Whether the layout is in logical pixels (scaled) and not in panel pixels.
    pub logical_mode: bool,
    /// The connected monitors, each with its mode. `None` means no mode.
    pub connected: BTreeMap<String, Option<Size>>,
    /// The connectors of the panels that are part of the machine.
    pub builtin: Vec<String>,
    /// The layout.
    pub logical: Vec<Monitor>,
}

/// The direction in the layout that goes down the laptop, as (x, y), for each
/// transform of a panel: down when upright, left with the left side up, up
/// when upside down, right with the right side up.
pub const DOWN: [(i32, i32); 4] = [(0, 1), (-1, 0), (0, -1), (1, 0)];

/// The position of the logical monitor that shows this connector.
pub(crate) fn index_of(logical: &[Monitor], connector: &str) -> Option<usize> {
    logical
        .iter()
        .position(|monitor| monitor.connectors.iter().any(|name| name == connector))
}

/// The mode of the first connector of a monitor. A monitor without one has no
/// mode.
pub(crate) fn mode_of(state: &State, monitor: &Monitor) -> Option<Size> {
    monitor
        .connectors
        .first()
        .and_then(|connector| state.connected.get(connector).copied().flatten())
}

/// Whether two numbers are equal, as the `==` of JavaScript says.
pub(crate) fn same(one: f64, other: f64) -> bool {
    one.partial_cmp(&other) == Some(Ordering::Equal)
}

/// Round half up, as `Math.round` of JavaScript does. A number out of range
/// goes to the nearest limit.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the cast saturates; this is the wanted result"
)]
fn round(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

/// The logical monitor that shows this connector.
#[must_use]
pub fn find<'a>(logical: &'a [Monitor], connector: &str) -> Option<&'a Monitor> {
    index_of(logical, connector).map(|at| &logical[at])
}

/// Move the layout so that its top left corner is at (0, 0).
#[must_use]
pub fn to_origin(logical: &[Monitor]) -> Vec<Monitor> {
    let left = logical.iter().map(|monitor| monitor.x).min().unwrap_or(0);
    let top = logical.iter().map(|monitor| monitor.y).min().unwrap_or(0);

    logical
        .iter()
        .map(|monitor| Monitor {
            x: monitor.x - left,
            y: monitor.y - top,
            ..monitor.clone()
        })
        .collect()
}

/// The size a logical monitor takes up in the layout, for a mode.
#[must_use]
pub fn size_of(monitor: &Monitor, mode: Size, logical_mode: bool) -> Size {
    let (width, height) = if monitor.transform % 2 == 1 {
        (mode.height, mode.width)
    } else {
        (mode.width, mode.height)
    };

    if logical_mode {
        Size {
            width: round(f64::from(width) / monitor.scale),
            height: round(f64::from(height) / monitor.scale),
        }
    } else {
        Size { width, height }
    }
}

/// The place a monitor takes up in the layout. A monitor whose first connector
/// has no mode takes no room: its size is 0 by 0.
#[must_use]
pub fn area_of(state: &State, monitor: &Monitor) -> Area {
    let mode = mode_of(state, monitor).unwrap_or(Size {
        width: 0,
        height: 0,
    });
    let Size { width, height } = size_of(monitor, mode, state.logical_mode);

    Area {
        x: monitor.x,
        y: monitor.y,
        width,
        height,
    }
}

/// How far two areas reach over each other, along the x axis if `horizontal`
/// is true, else along the y axis. Zero means they touch.
fn span(one: &Area, other: &Area, horizontal: bool) -> i32 {
    let (from, size, from_other, size_other) = if horizontal {
        (one.x, one.width, other.x, other.width)
    } else {
        (one.y, one.height, other.y, other.height)
    };

    (from + size).min(from_other + size_other) - from.max(from_other)
}

/// Whether two areas have room in common.
fn over(one: &Area, other: &Area) -> bool {
    span(one, other, true) > 0 && span(one, other, false) > 0
}

/// Whether two areas share a piece of an edge.
fn touch(one: &Area, other: &Area) -> bool {
    (span(one, other, true) == 0 && span(one, other, false) > 0)
        || (span(one, other, false) == 0 && span(one, other, true) > 0)
}

/// Whether the compositor can take a layout: no monitor lies over another, and
/// each one touches another along an edge, so that the pointer can go from any
/// monitor to any other. An empty layout is not valid.
#[must_use]
pub fn is_valid(state: &State, layout: &[Monitor]) -> bool {
    let areas: Vec<Area> = layout
        .iter()
        .map(|monitor| area_of(state, monitor))
        .collect();

    if areas
        .iter()
        .enumerate()
        .any(|(index, area)| areas[..index].iter().any(|other| over(area, other)))
    {
        return false;
    }

    let mut reached = BTreeSet::from([0]);
    let mut open = vec![0];

    while let Some(from) = open.pop() {
        for (to, area) in areas.iter().enumerate() {
            if touch(&areas[from], area) && reached.insert(to) {
                open.push(to);
            }
        }
    }
    reached.len() == areas.len()
}

/// `logical` with the monitors that lie in line with `area`, at its near edge
/// or beyond it in the direction `direction`, moved by `steps` times the size
/// of `area` in that direction: 1 makes room for a monitor there, -1 closes
/// the hole that one leaves. The direction is one of [`DOWN`].
#[must_use]
pub fn shifted(
    state: &State,
    logical: &[Monitor],
    area: Area,
    direction: (i32, i32),
    steps: i32,
) -> Vec<Monitor> {
    let (dx, dy) = direction;
    let horizontal = dx != 0;
    // The edge, the size, and the edge and size across.
    let axes = |area: &Area| {
        if horizontal {
            (area.x, area.width, area.y, area.height)
        } else {
            (area.y, area.height, area.x, area.width)
        }
    };
    let forward = dx + dy == 1;
    let (from, size, across, along) = axes(&area);
    let distance = (if forward { steps } else { -steps }) * size;

    logical
        .iter()
        .map(|monitor| {
            let (other_from, other_size, other_across, other_along) =
                axes(&area_of(state, monitor));
            let in_line = other_across < across + along && other_across + other_along > across;
            let beyond = if forward {
                other_from >= from
            } else {
                other_from + other_size <= from + size
            };
            let mut moved = monitor.clone();

            if in_line && beyond {
                if horizontal {
                    moved.x += distance;
                } else {
                    moved.y += distance;
                }
            }
            moved
        })
        .collect()
}

/// Whether `logical` shows exactly the connectors of `saved`, without `except`.
#[must_use]
pub fn is_rest(logical: &[Monitor], saved: &[Monitor], except: Option<&str>) -> bool {
    let mut now: Vec<&str> = logical
        .iter()
        .flat_map(|monitor| monitor.connectors.iter().map(String::as_str))
        .collect();
    let mut before: Vec<&str> = saved
        .iter()
        .flat_map(|monitor| monitor.connectors.iter().map(String::as_str))
        .filter(|name| Some(*name) != except)
        .collect();

    now.sort_unstable();
    before.sort_unstable();
    now == before
}

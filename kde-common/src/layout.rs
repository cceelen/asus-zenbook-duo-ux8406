//! What to do with the display layout when the lower panel is switched off or
//! on by its key.
//!
//! This is `gnome-keys/extension/layout.js`.

use crate::geometry::{
    Area, DOWN, Monitor, State, area_of, find, index_of, is_rest, is_valid, shifted, size_of,
    to_origin,
};

/// The layout without the logical monitor that shows `connector`. `None` if
/// the connector is not shown, is mirrored with another monitor, or nothing
/// would be left. If it was the primary monitor, `heir` becomes the primary
/// monitor, or else the first one.
///
/// Where the monitors do not hold together without it, as when it lies between
/// two of them, those right of it or below it come nearer by its size.
#[must_use]
pub fn without(state: &State, connector: &str, heir: &str) -> Option<Vec<Monitor>> {
    let gone = index_of(&state.logical, connector)
        .filter(|&at| state.logical[at].connectors.len() == 1 && state.logical.len() > 1)?;
    let mut rest: Vec<Monitor> = state.logical.clone();
    let gone = rest.remove(gone);

    if gone.primary {
        let heir = index_of(&rest, heir).unwrap_or(0);

        rest[heir].primary = true;
    }

    let hole = area_of(state, &gone);
    let ways = [
        rest.clone(),
        shifted(state, &rest, hole, (1, 0), -1),
        shifted(state, &rest, hole, (0, 1), -1),
    ];

    Some(to_origin(
        &ways
            .into_iter()
            .find(|way| is_valid(state, way))
            .unwrap_or(rest),
    ))
}

/// The layout with `connector` at the bottom of the logical monitor that shows
/// `anchor`, at its scale and turned the same way: directly below it, or, with
/// the anchor turned, on the side where the bottom of the laptop then is. A
/// monitor that lies there moves on to make room. `None` if the anchor is not
/// shown by itself or one of the two has no mode.
#[must_use]
pub fn with_below(state: &State, connector: &str, anchor: &str) -> Option<Vec<Monitor>> {
    let above = find(&state.logical, anchor).filter(|monitor| monitor.connectors.len() == 1);
    let mode = state.connected.get(anchor).copied().flatten();
    let added = state.connected.get(connector).copied().flatten();
    let (Some(above), Some(mode), Some(added)) = (above, mode, added) else {
        return None;
    };

    let down = DOWN[(above.transform % 4) as usize];
    let (dx, dy) = down;
    let upper = size_of(above, mode, state.logical_mode);
    let mut panel = Monitor {
        primary: false,
        connectors: vec![connector.to_owned()],
        ..above.clone()
    };
    let lower = size_of(&panel, added, state.logical_mode);

    panel.x += if dx > 0 {
        upper.width
    } else {
        dx * lower.width
    };
    panel.y += if dy > 0 {
        upper.height
    } else {
        dy * lower.height
    };

    let place = Area {
        x: panel.x,
        y: panel.y,
        width: lower.width,
        height: lower.height,
    };
    let mut ways = [
        state.logical.clone(),
        shifted(state, &state.logical, place, down, 1),
    ];

    for way in &mut ways {
        way.push(panel.clone());
    }

    let [first, second] = ways;

    Some(to_origin(if is_valid(state, &first) {
        &first
    } else if is_valid(state, &second) {
        &second
    } else {
        &first
    }))
}

/// What the key does.
#[derive(Clone, Debug, PartialEq)]
pub struct Toggle {
    /// The layout to give to the compositor, or `None` for nothing.
    pub apply: Option<Vec<Monitor>>,
    /// The layout to remember for the next press, or `None` to forget.
    pub save: Option<Vec<Monitor>>,
}

/// What the key does.
///
/// - The lower panel is shown: take it out and remember the layout.
/// - It is not shown but connected: put the remembered layout back if the other
///   monitors are still the same and the upper panel is turned the same way,
///   else put it below the upper panel.
/// - It is not connected (the keyboard lies on it): nothing.
#[must_use]
pub fn toggle(state: &State, saved: Option<&[Monitor]>, lower: &str, upper: &str) -> Toggle {
    if find(&state.logical, lower).is_some() {
        let apply = without(state, lower, upper);
        let save = if apply.is_some() {
            Some(state.logical.clone())
        } else {
            saved.map(<[Monitor]>::to_vec)
        };

        return Toggle { apply, save };
    }
    if !state.connected.contains_key(lower) {
        return Toggle {
            apply: None,
            save: saved.map(<[Monitor]>::to_vec),
        };
    }

    let turn = |layout: &[Monitor]| find(layout, upper).map(|monitor| monitor.transform);

    match saved {
        Some(saved)
            if is_rest(&state.logical, saved, Some(lower))
                && turn(saved) == turn(&state.logical) =>
        {
            Toggle {
                apply: Some(saved.to_vec()),
                save: None,
            }
        }
        _ => Toggle {
            apply: with_below(state, lower, upper),
            save: None,
        },
    }
}

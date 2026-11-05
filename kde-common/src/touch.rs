//! Which panel each touchscreen and pen of the UX8406 belongs to.
//!
//! This is `gnome-keys/extension/touch.js`. The two panels have one size and say
//! that they are the same monitor, so without a setting both touchscreens get
//! the first panel. The setting of a touch device can name the panel by its
//! connector.

use crate::geometry::State;

/// The touch controller of the upper panel, as the i2c bus names it.
const UPPER_CONTROLLER: &str = "ELAN9008";
/// The touch controller of the lower panel.
const LOWER_CONTROLLER: &str = "ELAN9009";

/// A touch controller of a panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Controller {
    /// The name of the controller, for example `ELAN9008`.
    pub controller: String,
    /// The id of its HID device as the compositor writes it: `vendor:product`
    /// in small letters.
    pub id: String,
}

/// The panel to use for a touch device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    /// The id of the touch device.
    pub id: String,
    /// The connector of its panel.
    pub connector: String,
}

/// The id `vendor:product` of a HID device name, `bus:vendor:product.instance`
/// in hex, in small letters. `None` for another name.
fn hid_id(child: &str) -> Option<String> {
    let bytes = child.as_bytes();
    let fits = bytes.len() > 15
        && bytes[4] == b':'
        && bytes[9] == b':'
        && bytes[14] == b'.'
        && bytes
            .iter()
            .enumerate()
            .all(|(at, byte)| matches!(at, 4 | 9 | 14) || byte.is_ascii_hexdigit());

    // Every byte is ASCII here, so the cuts are on character borders.
    fits.then(|| format!("{}:{}", &child[5..9], &child[10..14]).to_ascii_lowercase())
}

/// The touch controllers among the i2c devices of the machine. `devices` is a
/// list of the name of a directory in `/sys/bus/i2c/devices` and the names in
/// it. A controller counts when it is one of the two known ones and has one HID
/// device.
#[must_use]
pub fn controllers_of(devices: &[(String, Vec<String>)]) -> Vec<Controller> {
    let mut found = Vec::new();

    for (name, children) in devices {
        let controller = name
            .strip_prefix("i2c-")
            .and_then(|rest| rest.split_once(':'))
            .map(|(controller, _)| controller)
            .filter(|controller| matches!(*controller, UPPER_CONTROLLER | LOWER_CONTROLLER));
        let ids: Vec<String> = children.iter().filter_map(|child| hid_id(child)).collect();

        if let (Some(controller), [id]) = (controller, ids.as_slice()) {
            found.push(Controller {
                controller: controller.to_owned(),
                id: id.clone(),
            });
        }
    }
    found
}

/// What to set for each controller whose panel the compositor knows. A panel
/// that is not connected (the keyboard lies on it) has no entry yet. A
/// controller that is not one of the two known ones has none either.
#[must_use]
pub fn assignments(
    controllers: &[Controller],
    state: &State,
    upper: &str,
    lower: &str,
) -> Vec<Assignment> {
    controllers
        .iter()
        .filter_map(|controller| {
            let panel = match controller.controller.as_str() {
                UPPER_CONTROLLER => upper,
                LOWER_CONTROLLER => lower,
                _ => return None,
            };

            state.connected.contains_key(panel).then(|| Assignment {
                id: controller.id.clone(),
                connector: panel.to_owned(),
            })
        })
        .collect()
}

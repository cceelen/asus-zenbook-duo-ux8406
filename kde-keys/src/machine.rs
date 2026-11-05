//! What the program reads from sysfs: the model, and the touch controllers.

use std::fs;
use std::path::Path;

/// What the product name of a supported machine contains: every UX8406.
pub const MODEL: &str = "UX8406";

/// Whether the machine is a Zenbook Duo UX8406.
#[must_use]
pub fn is_model(root: &Path) -> bool {
    fs::read_to_string(root.join("class/dmi/id/product_name"))
        .is_ok_and(|name| name.contains(MODEL))
}

/// The names in a directory, sorted; none if it cannot be read.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();

    names.sort();
    names
}

/// The i2c devices of the machine, each with the names in its directory:
/// what [`asus_ux8406_kde_common::touch::controllers_of`] reads.
#[must_use]
pub fn i2c_devices(root: &Path) -> Vec<(String, Vec<String>)> {
    let dir = root.join("bus/i2c/devices");

    names(&dir)
        .into_iter()
        .map(|name| {
            let children = names(&dir.join(&name));

            (name, children)
        })
        .collect()
}

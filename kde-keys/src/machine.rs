//! What the program reads from sysfs: the model, and the touch controllers.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;

pub(crate) const MODELS: [&str; 2] = ["UX8406", "UX8407"];

/// A product name that is not there or not readable is no match: the program
/// then does nothing, and says so.
#[must_use]
pub fn is_model(root: &Path) -> bool {
    fs::read_to_string(root.join("class/dmi/id/product_name"))
        .is_ok_and(|name| MODELS.iter().any(|model| name.contains(model)))
}

/// The names in a directory, sorted.
fn names(dir: &Path) -> io::Result<Vec<String>> {
    sorted(fs::read_dir(dir)?.map(|entry| entry.map(|entry| entry.file_name())))
}

/// The names of a listing, sorted.
///
/// # Errors
///
/// An entry of the listing could not be read.
pub fn sorted(listing: impl Iterator<Item = io::Result<OsString>>) -> io::Result<Vec<String>> {
    let mut names = listing
        .map(|name| name.map(|name| name.to_string_lossy().into_owned()))
        .collect::<io::Result<Vec<String>>>()?;

    names.sort();
    Ok(names)
}

/// The i2c devices of the machine, each with the names in its directory:
/// what [`asus_ux8406_kde_common::touch::controllers_of`] reads.
///
/// # Errors
///
/// A directory cannot be read, or an entry of it cannot: the touchscreens
/// would be taken for absent otherwise.
pub fn i2c_devices(root: &Path) -> io::Result<Vec<(String, Vec<String>)>> {
    let dir = root.join("bus/i2c/devices");

    names(&dir)?
        .into_iter()
        .map(|name| names(&dir.join(&name)).map(|children| (name, children)))
        .collect()
}

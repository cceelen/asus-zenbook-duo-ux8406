//! Where a failure of the system can be injected: the calls whose failure no
//! arrangement of files brings about. In a build without the `failpoints`
//! feature these are the calls themselves and nothing else.

use std::io;

/// The result of a call to the system, or, if the fail point of that name
/// is switched on, a failure in its place.
///
/// # Errors
///
/// What the call gave, or the injected failure.
#[cfg(feature = "failpoints")]
pub fn point<T>(name: &str, real: io::Result<T>) -> io::Result<T> {
    fail::fail_point!(name, |_| Err(io::Error::other(format!(
        "injected at {name}"
    ))));
    real
}

/// The result of a call to the system: this build has no fail points.
///
/// # Errors
///
/// What the call gave.
#[cfg(not(feature = "failpoints"))]
pub const fn point<T>(_name: &str, real: io::Result<T>) -> io::Result<T> {
    real
}

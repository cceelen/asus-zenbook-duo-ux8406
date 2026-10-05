//! Files the guard believes: its configuration and what it has noted for
//! itself. As it runs as root, such a file has to be a regular file, not a
//! symbolic link, owned by root or by the user running the guard, and
//! writable by nobody else.

use std::fs::{self, File, Metadata};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rustix::fs::{Mode, OFlags};
use rustix::process::geteuid;

use crate::error::Error;
use crate::fault;

/// Permission bits that let someone other than the owner write.
const OTHERS_MAY_WRITE: u32 = 0o022;
const ROOT: u32 = 0;

fn owned(entry: &Metadata) -> bool {
    (entry.uid() == ROOT || entry.uid() == geteuid().as_raw())
        && entry.mode() & OTHERS_MAY_WRITE == 0
}

/// Read a file of at most `max` bytes.
///
/// # Errors
///
/// [`Error::Io`] if it cannot be opened or read, a symbolic link included,
/// and [`Error::NotTrusted`] if it is not a file of the kind described
/// above, or bigger.
pub fn read(path: &Path, max: usize) -> Result<Vec<u8>, Error> {
    let refused = || Error::NotTrusted(path.to_owned());
    // Without following a link, and without waiting if it is a pipe.
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = File::from(rustix::fs::open(path, flags, Mode::empty()).map_err(Error::io(path))?);
    let opened = fault::point("trusted-stat", file.metadata()).map_err(Error::io(path))?;
    let limit = u64::try_from(max).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();

    if !opened.is_file() || !owned(&opened) {
        return Err(refused());
    }
    // One byte more than allowed tells a file that is too big.
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(Error::io(path))?;
    if bytes.len() > max {
        return Err(refused());
    }
    Ok(bytes)
}

/// Check that a directory is one only the administrator can change.
///
/// # Errors
///
/// [`Error::Io`] if it cannot be looked at, and [`Error::NotTrusted`] if it
/// is something else.
pub fn directory(path: &Path) -> Result<(), Error> {
    let named = fs::symlink_metadata(path).map_err(Error::io(path))?;

    if named.is_dir() && owned(&named) {
        Ok(())
    } else {
        Err(Error::NotTrusted(path.to_owned()))
    }
}

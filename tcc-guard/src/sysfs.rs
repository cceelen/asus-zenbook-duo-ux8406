//! The sysfs attributes the guard reads and writes: small text files, each
//! holding one value. Nothing is ever created.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use rustix::fs::{Mode, OFlags};

use crate::error::Error;
use crate::fault;
use crate::input;

/// A number in sysfs is at most this long, with its newline.
pub const VALUE_MAX: usize = 30;

/// Open a file that exists, without following a symbolic link and without
/// waiting if it is a pipe.
fn open(path: &Path, access: OFlags) -> Result<File, Error> {
    let flags = access | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = File::from(rustix::fs::open(path, flags, Mode::empty()).map_err(Error::io(path))?);

    if fault::point("sysfs-stat", file.metadata())
        .map_err(Error::io(path))?
        .is_file()
    {
        Ok(file)
    } else {
        Err(Error::BadValue(path.to_owned()))
    }
}

/// Read a whole small file as text.
///
/// # Errors
///
/// [`Error::Io`] if it cannot be opened or read, a symbolic link included,
/// and [`Error::BadValue`] if it is not a regular file, is empty, longer
/// than `max` bytes, or not plain text.
pub fn read_text(path: &Path, max: usize) -> Result<String, Error> {
    let io = Error::io(path);
    let bad = || Error::BadValue(path.to_owned());
    let mut bytes = Vec::new();

    open(path, OFlags::RDONLY)?
        .take(u64::try_from(max).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(&io)?;
    if bytes.is_empty() || bytes.len() > max || bytes.contains(&0) {
        return Err(bad());
    }
    String::from_utf8(bytes).map_err(|_| bad())
}

fn read_value(path: &Path, parse: fn(&str) -> Option<i32>) -> Result<i32, Error> {
    parse(&read_text(path, VALUE_MAX)?).ok_or_else(|| Error::BadValue(path.to_owned()))
}

/// A temperature, in degrees.
///
/// # Errors
///
/// As [`read_text`], and [`Error::BadValue`] if it is not a plausible
/// temperature in millidegrees.
pub fn read_temp(path: &Path) -> Result<i32, Error> {
    read_value(path, input::parse_temp)
}

/// The limit of a package at offset 0, in degrees.
///
/// # Errors
///
/// As [`read_text`], and [`Error::BadValue`] if it is not a plausible limit.
pub fn read_tjmax(path: &Path) -> Result<i32, Error> {
    read_value(path, input::parse_tjmax)
}

/// A thermal offset.
///
/// # Errors
///
/// As [`read_text`], and [`Error::BadValue`] if it is not an offset.
pub fn read_offset(path: &Path) -> Result<i32, Error> {
    read_value(path, input::parse_offset)
}

/// Write an offset to a file that exists already.
///
/// # Errors
///
/// [`Error::BadValue`] if the offset is not one the register holds or the
/// file is not a regular file, and [`Error::Io`] if it cannot be opened or
/// written.
pub fn write_offset(path: &Path, offset: i32) -> Result<(), Error> {
    if !(0..=input::OFFSET_MAX).contains(&offset) {
        return Err(Error::BadValue(path.to_owned()));
    }
    open(path, OFlags::WRONLY)?
        .write_all(format!("{offset}\n").as_bytes())
        .map_err(Error::io(path))
}

/// Check that an offset could be written, without writing.
///
/// # Errors
///
/// [`Error::ReadOnly`] if not.
pub fn check_writable(path: &Path) -> Result<(), Error> {
    match open(path, OFlags::WRONLY) {
        Ok(_) => Ok(()),
        Err(Error::Io { path, source }) => Err(Error::ReadOnly { path, source }),
        Err(other) => Err(other),
    }
}

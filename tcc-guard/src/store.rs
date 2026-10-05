//! What the guard notes outside itself: for every package whose offset it
//! has changed, the offset it found. A guard that is stopped puts its
//! offsets back; one that is killed or crashes cannot. What is noted here
//! lets the next start, or `asus-ux8406-tcc-guard restore`, do it then.
//!
//! One small file per package, in a directory that only the administrator
//! can change. The directory does not survive a restart of the machine, and
//! neither does a thermal offset.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::fault;
use crate::input;
use crate::sysfs::VALUE_MAX;
use crate::trusted;

const PREFIX: &str = "package-";
/// The name a note has while it is written.
const FRESH: &str = "fresh";
const PACKAGE_MAX: i32 = 1023;
const DIR_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

/// The directory of the notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// The notes in `dir`. Nothing is touched yet.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
        }
    }

    fn file(&self, package: i32) -> PathBuf {
        self.dir.join(format!("{PREFIX}{package}"))
    }

    /// Make sure that the directory is there and is the administrator's.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it cannot be made, and [`Error::NotTrusted`] if it is
    /// there but someone else's.
    pub fn prepare(&self) -> Result<(), Error> {
        match DirBuilder::new().mode(DIR_MODE).create(&self.dir) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => {
                Err(Error::io(&self.dir)(error))
            }
            _ => trusted::directory(&self.dir),
        }
    }

    /// Note the offset found for a package, before it is changed. The note
    /// appears whole or not at all.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it cannot be written.
    pub fn save(&self, package: i32, base: i32) -> Result<(), Error> {
        let fresh = self.dir.join(FRESH);
        let _ = fs::remove_file(&fresh);

        Self::write(&fresh, base)
            .and_then(|()| fs::rename(&fresh, self.file(package)).map_err(Error::io(&fresh)))
            // Nothing half done stays behind.
            .inspect_err(|_| drop(fs::remove_file(&fresh)))
    }

    /// Write a note under the name it has while it is not whole.
    fn write(fresh: &Path, base: i32) -> Result<(), Error> {
        let io = Error::io(fresh);
        let mut note = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .open(fresh)
            .map_err(&io)?;

        fault::point(
            "store-write",
            note.write_all(format!("{base}\n").as_bytes()),
        )
        .map_err(&io)
    }

    /// Forget the note of a package. It is no failure if there is none.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if it is there and cannot be removed.
    pub fn clear(&self, package: i32) -> Result<(), Error> {
        let path = self.file(package);

        match fs::remove_file(&path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(Error::io(&path)(error)),
            _ => Ok(()),
        }
    }

    /// Every note: (package, offset found), lowest package first. None if
    /// there is no directory.
    ///
    /// # Errors
    ///
    /// [`Error::NotTrusted`] if the directory or a note is someone else's,
    /// [`Error::BadValue`] if a note is not an offset, and [`Error::Io`] if
    /// one cannot be read.
    pub fn saved(&self) -> Result<Vec<(i32, i32)>, Error> {
        let io = Error::io(&self.dir);
        let mut saved = Vec::new();

        match trusted::directory(&self.dir) {
            Err(error) if error.is_missing() => return Ok(saved),
            checked => checked?,
        }
        for entry in fs::read_dir(&self.dir).map_err(&io)? {
            let name = fault::point("store-list", entry).map_err(&io)?.file_name();
            let Some(package) = name
                .to_str()
                .and_then(|name| name.strip_prefix(PREFIX))
                .and_then(|number| input::parse_int(number, 0, PACKAGE_MAX))
            else {
                continue;
            };
            let path = self.file(package);
            // "package-007" is not a name this program gives.
            if path.file_name() != Some(&name) {
                continue;
            }
            let bytes = trusted::read(&path, VALUE_MAX)?;
            let base = std::str::from_utf8(&bytes)
                .ok()
                .and_then(input::parse_offset)
                .ok_or(Error::BadValue(path))?;

            saved.push((package, base));
        }
        saved.sort_unstable();
        Ok(saved)
    }
}

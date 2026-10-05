//! The kernel log as the source of the EC's event.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{Mode, OFlags};
use rustix::io::Errno;

use crate::error::Error;
use crate::fault;
use crate::input;

/// A record of the kernel log is at most this long.
const RECORD_SIZE: usize = 8192;
/// No more records than this are read before the packages are looked at.
const RECORDS_PER_WAIT: usize = 4096;
/// A regular file has an end, and so has a pipe whose writer has gone: both
/// always count as readable. How long to wait there for more.
const IDLE_PAUSE: Duration = Duration::from_millis(200);

/// The events of the kernel log, from the moment it was opened.
#[derive(Debug)]
pub struct Events {
    path: PathBuf,
    log: File,
    event_text: String,
}

/// Read the records that are waiting, every read giving one; whether one of
/// them was the event, and how many reads gave something.
///
/// # Errors
///
/// What the log said, unless it only said that there is nothing more for
/// now, or that records were overwritten before they were read.
pub fn scan<R: Read>(log: &mut R, event_text: &str) -> io::Result<(bool, usize)> {
    let mut record = [0_u8; RECORD_SIZE];
    let (mut seen, mut count) = (false, 0);

    while count < RECORDS_PER_WAIT {
        match log.read(&mut record) {
            Ok(0) => break,
            Ok(length) => {
                count += 1;
                seen |= input::is_event(event_text, &String::from_utf8_lossy(&record[..length]));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            // Records were overwritten before they were read.
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => count += 1,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok((seen, count))
}

impl Events {
    /// Open the log at its end: what is in it already is not news.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the log cannot be opened.
    pub fn open(path: &Path, event_text: &str) -> Result<Self, Error> {
        // Without waiting: a pipe that nobody writes to would not open.
        let flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC;
        let mut log =
            File::from(rustix::fs::open(path, flags, Mode::empty()).map_err(Error::io(path))?);

        // A pipe has no end to go to.
        let _ = log.seek(SeekFrom::End(0));
        Ok(Self {
            path: path.to_owned(),
            log,
            event_text: event_text.to_owned(),
        })
    }

    /// Wait for records for at most `patience`, or until a signal arrives;
    /// whether one of them was the event.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the log can no longer be read: the guard would not
    /// notice the event any more.
    pub fn wait(&mut self, patience: Duration) -> Result<bool, Error> {
        let patience = Timespec::try_from(patience).unwrap_or(Timespec {
            tv_sec: 1,
            tv_nsec: 0,
        });
        let waited = poll(
            &mut [PollFd::new(&self.log, PollFlags::IN)],
            Some(&patience),
        );

        match waited {
            Ok(0) | Err(Errno::INTR) => return Ok(false),
            _ => {}
        }
        fault::point("kmsg-poll", waited.map_err(io::Error::from))
            .map_err(Error::io(&self.path))?;
        let (seen, count) = scan(&mut self.log, &self.event_text).map_err(Error::io(&self.path))?;

        if count == 0 {
            thread::sleep(IDLE_PAUSE);
        }
        Ok(seen)
    }
}

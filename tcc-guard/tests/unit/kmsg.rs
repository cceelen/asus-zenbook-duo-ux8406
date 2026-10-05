//! The kernel log as the source of the event.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::kmsg::{Events, scan};
use rstest::rstest;

use crate::common::{KMSG, Machine, machine, ordinary_user};

const TEXT: &str = "test: thermal warning";
const EVENT: &str = "6,4,4,-;test: thermal warning\n";
const SOON: Duration = Duration::from_millis(50);

/// A log that gives what the test has written down for it, one record per
/// read, and then says that there is nothing more for now.
struct Script(VecDeque<io::Result<&'static str>>);

impl Script {
    fn new<const N: usize>(reads: [io::Result<&'static str>; N]) -> Self {
        Self(reads.into())
    }
}

impl Read for Script {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let record = self
            .0
            .pop_front()
            .unwrap_or_else(|| Err(io::ErrorKind::WouldBlock.into()))?;

        buffer[..record.len()].copy_from_slice(record.as_bytes());
        Ok(record.len())
    }
}

/// A log that never runs out of events.
struct Flood;

impl Read for Flood {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        buffer[..EVENT.len()].copy_from_slice(EVENT.as_bytes());
        Ok(EVENT.len())
    }
}

#[test]
fn every_read_is_a_record_and_one_event_is_enough() {
    let mut log = Script::new([
        Ok("6,1,1,-;something else\n"),
        Ok(EVENT),
        Ok("14,2,2,-;test: thermal warning\n"),
    ]);

    assert_eq!(scan(&mut log, TEXT).unwrap(), (true, 3));
    assert_eq!(scan(&mut log, TEXT).unwrap(), (false, 0), "nothing more");
    assert_eq!(scan(&mut Script::new([Ok("")]), TEXT).unwrap(), (false, 0));
    assert_eq!(
        scan(&mut Script::new([Ok("6,1,1,-;other\n")]), TEXT).unwrap(),
        (false, 1)
    );
}

/// The kernel says so when records were overwritten before they were read;
/// the next read gives the next record that is still there.
#[test]
fn records_that_were_lost_do_not_stop_the_reading() {
    let mut log = Script::new([
        Err(io::ErrorKind::BrokenPipe.into()),
        Err(io::ErrorKind::Interrupted.into()),
        Ok("6,1,1,-;something else\n"),
        Err(io::ErrorKind::BrokenPipe.into()),
        Ok(EVENT),
    ]);

    assert_eq!(scan(&mut log, TEXT).unwrap(), (true, 4));
}

#[test]
fn any_other_failure_of_the_log_is_one() {
    let mut log = Script::new([Ok(EVENT), Err(io::Error::other("gone"))]);

    assert_eq!(scan(&mut log, TEXT).unwrap_err().to_string(), "gone");
}

#[test]
fn a_flood_of_records_does_not_keep_the_guard_from_its_packages() {
    let (seen, count) = scan(&mut Flood, TEXT).unwrap();

    assert!(seen);
    assert_eq!(count, 4096);
}

fn append(machine: &Machine, record: &str) {
    machine.append(KMSG, record);
}

#[rstest]
fn only_the_event_is_told_and_only_what_comes_after_the_start(machine: Machine) {
    machine.put(KMSG, EVENT);
    let mut events = Events::open(&machine.join(KMSG), TEXT).unwrap();

    assert!(!events.wait(SOON).unwrap(), "old records");
    append(&machine, "14,2,2,-;test: thermal warning\n");
    assert!(!events.wait(SOON).unwrap(), "written by a program");
    append(&machine, "6,3,3,-;something else\n");
    assert!(!events.wait(SOON).unwrap(), "not the event");
    append(&machine, EVENT);
    assert!(events.wait(SOON).unwrap());
    assert!(!events.wait(SOON).unwrap(), "told once");
}

/// A regular file always counts as readable; the guard must not spin on it.
#[rstest]
fn a_log_at_its_end_is_waited_on(machine: Machine) {
    let mut events = Events::open(&machine.join(KMSG), TEXT).unwrap();
    let start = Instant::now();

    assert!(!events.wait(SOON).unwrap());
    assert!(start.elapsed() >= Duration::from_millis(150));
}

/// A pipe is what reading the kernel log is like: it waits.
#[rstest]
fn a_pipe_nobody_writes_to_opens_and_stays_quiet(machine: Machine) {
    machine.fifo(KMSG);
    let mut events = Events::open(&machine.join(KMSG), TEXT).unwrap();
    let start = Instant::now();

    assert!(!events.wait(Duration::from_millis(300)).unwrap());
    assert!(start.elapsed() >= Duration::from_millis(300), "waited");

    let mut writer = OpenOptions::new()
        .write(true)
        .open(machine.join(KMSG))
        .unwrap();
    writer.write_all(EVENT.as_bytes()).unwrap();
    assert!(events.wait(Duration::from_secs(20)).unwrap());
    assert!(!events.wait(SOON).unwrap(), "read, and quiet again");
}

#[rstest]
fn a_log_that_is_not_there_or_fails_is_an_error(machine: Machine) {
    ordinary_user();
    assert!(
        Events::open(&machine.join("dev/none"), TEXT)
            .unwrap_err()
            .is_missing()
    );
    // A directory can be opened, but not read.
    let mut events = Events::open(&machine.join("dev"), TEXT).unwrap();
    assert!(matches!(events.wait(SOON), Err(Error::Io { .. })));
    assert!(
        matches!(events.wait(SOON), Err(Error::Io { .. })),
        "and stays one"
    );

    machine.chmod(KMSG, 0o000);
    assert!(matches!(
        Events::open(&machine.join(KMSG), TEXT),
        Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::PermissionDenied
    ));
}

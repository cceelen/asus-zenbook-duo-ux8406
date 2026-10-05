//! The sysfs attributes the guard reads and writes.

use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::sysfs::{
    VALUE_MAX, check_writable, read_offset, read_temp, read_text, read_tjmax, write_offset,
};
use rstest::rstest;

use crate::common::{Machine, empty, ordinary_user};

fn bad<T>(result: &Result<T, Error>) -> bool {
    matches!(result, Err(Error::BadValue(_)))
}

fn denied<T>(result: &Result<T, Error>) -> bool {
    matches!(
        result,
        Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::PermissionDenied
    )
}

#[rstest]
fn values_are_read(empty: Machine) {
    assert_eq!(read_temp(&empty.put("temp", "95000\n")).unwrap(), 95);
    assert_eq!(read_tjmax(&empty.put("crit", "105000\n")).unwrap(), 105);
    assert_eq!(read_offset(&empty.put("offset", "10\n")).unwrap(), 10);
}

#[rstest]
#[case::empty(b"")]
#[case::not_a_number(b"rubbish\n")]
#[case::too_hot(b"999000\n")]
#[case::a_nul(b"95\x00000\n")]
#[case::not_text(b"95\xe5\n")]
#[case::too_long(b"9500000000000000000000000000000000000000\n")]
fn what_is_not_a_temperature_is_refused(empty: Machine, #[case] content: &[u8]) {
    assert!(bad(&read_temp(&empty.put("temp", content))));
}

#[rstest]
fn what_is_not_a_value_or_not_a_file_is_refused(empty: Machine) {
    let real = empty.put("real", "95000\n");
    let link = empty.join("link");

    assert!(bad(&read_offset(&empty.put("offset", "99\n"))));
    assert!(bad(&read_tjmax(&empty.put("crit", "999000\n"))));
    symlink(&real, &link).unwrap();
    assert!(
        matches!(read_temp(&link), Err(Error::Io { .. })),
        "a symbolic link is not followed"
    );
    assert!(bad(&read_temp(empty.root())), "a directory");
    empty.fifo("pipe");
    assert!(
        bad(&read_temp(&empty.join("pipe"))),
        "a pipe, without waiting"
    );
    assert!(read_temp(&empty.join("none")).unwrap_err().is_missing());
}

#[rstest]
fn a_file_that_may_not_be_read_is_an_error(empty: Machine) {
    let path = empty.put("closed", "95000\n");

    ordinary_user();
    empty.chmod("closed", 0o000);
    assert!(denied(&read_temp(&path)));
}

/// The kernel opens the memory of a process for it, and fails the read of
/// where nothing is mapped: a file that fails as a sensor can.
#[test]
fn a_file_that_opens_and_then_fails_is_an_error() {
    assert!(matches!(
        read_temp(Path::new("/proc/self/mem")),
        Err(Error::Io { .. })
    ));
}

#[rstest]
fn the_longest_text_fits_and_one_more_byte_does_not(empty: Machine) {
    let longest = "x".repeat(VALUE_MAX);
    let path = empty.put("text", &longest);

    assert_eq!(read_text(&path, VALUE_MAX).unwrap(), longest);
    assert!(bad(&read_text(&path, VALUE_MAX - 1)));
}

#[rstest]
fn offsets_are_written_to_files_that_exist(empty: Machine) {
    let path = empty.put("offset", "10\n");

    // Shorter and longer than what was there.
    for offset in [9, 12, 0, 63] {
        write_offset(&path, offset).unwrap();
        assert_eq!(read_offset(&path).unwrap(), offset);
    }
    assert!(bad(&write_offset(&path, 64)));
    assert!(bad(&write_offset(&path, -1)));
    assert_eq!(read_offset(&path).unwrap(), 63, "left alone");
    assert!(
        write_offset(&empty.join("none"), 5)
            .unwrap_err()
            .is_missing()
    );
    assert!(!empty.join("none").exists(), "nothing is created");
}

#[rstest]
fn an_offset_that_cannot_be_written_needs_root(empty: Machine) {
    let path = empty.put("offset", "10\n");

    ordinary_user();
    assert!(check_writable(&path).is_ok());
    assert_eq!(read_offset(&path).unwrap(), 10, "the check writes nothing");
    empty.chmod("offset", 0o400);
    assert!(matches!(
        check_writable(&path),
        Err(Error::ReadOnly { source, .. }) if source.kind() == io::ErrorKind::PermissionDenied
    ));
    assert!(denied(&write_offset(&path, 12)));
}

#[rstest]
fn an_offset_is_only_written_to_a_regular_file(empty: Machine) {
    let real = empty.put("real", "10\n");
    let link = empty.join("link");

    symlink(&real, &link).unwrap();
    for path in [link.as_path(), empty.root()] {
        assert!(write_offset(path, 12).is_err());
        assert!(check_writable(path).is_err());
    }
    empty.fifo("pipe");
    assert!(write_offset(&empty.join("pipe"), 12).is_err());
    assert_eq!(read_offset(&real).unwrap(), 10);
    assert!(check_writable(&empty.join("none")).is_err());
    // Something that can be written, and is no file.
    assert!(bad(&check_writable(Path::new("/dev/null"))));
    assert!(bad(&write_offset(Path::new("/dev/null"), 12)));
}

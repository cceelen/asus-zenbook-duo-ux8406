//! Files the guard believes.

use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::trusted::{believable, directory, read};
use rstest::rstest;

use crate::common::{Machine, empty, ordinary_user};

fn refused<T>(result: &Result<T, Error>) -> bool {
    matches!(result, Err(Error::NotTrusted(_)))
}

fn denied<T>(result: &Result<T, Error>) -> bool {
    matches!(
        result,
        Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::PermissionDenied
    )
}

#[rstest]
fn a_file_of_ones_own_is_read(empty: Machine) {
    let path = empty.put("conf", "hold_temp = 85\n");

    assert_eq!(read(&path, 15).unwrap(), b"hold_temp = 85\n");
    assert!(refused(&read(&path, 14)), "one byte too big");
}

#[rstest]
#[case(0o666)]
#[case(0o620)]
#[case(0o602)]
fn what_others_may_write_is_refused(empty: Machine, #[case] mode: u32) {
    let path = empty.put("conf", "x\n");

    empty.chmod("conf", mode);
    assert!(refused(&read(&path, 100)));
    empty.chmod("conf", 0o644);
    assert!(read(&path, 100).is_ok());
}

#[rstest]
fn what_is_not_a_file_is_refused(empty: Machine) {
    let real = empty.put("real", "x\n");
    let link = empty.join("link");

    symlink(&real, &link).unwrap();
    assert!(
        matches!(read(&link, 100), Err(Error::Io { .. })),
        "a symbolic link is not followed"
    );
    assert!(refused(&read(empty.root(), 100)), "a directory");
    empty.fifo("pipe");
    assert!(
        refused(&read(&empty.join("pipe"), 100)),
        "a pipe, without waiting"
    );
    assert!(read(&empty.join("none"), 100).unwrap_err().is_missing());
}

#[rstest]
fn a_file_that_may_not_be_read_is_an_error(empty: Machine) {
    let path = empty.put("closed", "x\n");

    ordinary_user();
    empty.chmod("closed", 0o000);
    assert!(denied(&read(&path, 100)));
}

/// The kernel opens the memory of a process for it, and fails the read of
/// where nothing is mapped.
#[test]
fn a_file_that_opens_and_then_fails_is_an_error() {
    assert!(matches!(
        read(Path::new("/proc/self/mem"), 100),
        Err(Error::Io { .. })
    ));
}

#[rstest]
fn directories(empty: Machine) {
    let file = empty.put("file", "x\n");
    let link = empty.join("link");

    assert!(directory(empty.root()).is_ok());
    assert!(refused(&directory(&file)));
    symlink(empty.root(), &link).unwrap();
    assert!(refused(&directory(&link)));
    assert!(directory(&empty.join("none")).unwrap_err().is_missing());
    empty.chmod("run", 0o777);
    assert!(refused(&directory(&empty.join("run"))));
}

#[rstest]
#[case::root_owns_it(0, 0o644, 1000, true)]
#[case::the_user_owns_it(1000, 0o600, 1000, true)]
#[case::root_runs_it_and_owns_it(0, 0o644, 0, true)]
#[case::another_user_owns_it(1001, 0o600, 1000, false)]
#[case::the_group_may_write(0, 0o664, 1000, false)]
#[case::others_may_write(1000, 0o606, 1000, false)]
#[case::another_user_and_others_may_write(1001, 0o666, 1000, false)]
fn who_owns_a_file_and_who_may_write_it(
    #[case] owner: u32,
    #[case] mode: u32,
    #[case] user: u32,
    #[case] believed: bool,
) {
    assert_eq!(believable(owner, mode, user), believed);
}

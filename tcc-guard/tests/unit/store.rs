//! What the guard notes outside itself.

use std::fs;
use std::io;
use std::os::unix::fs::{PermissionsExt, symlink};

use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::store::Store;
use rstest::rstest;

use crate::common::{Machine, STATE, empty, ordinary_user};

fn prepared(machine: &Machine) -> Store {
    let store = Store::new(&machine.join(STATE));

    store.prepare().unwrap();
    store
}

fn mode(machine: &Machine, relative: &str) -> u32 {
    fs::metadata(machine.join(relative))
        .unwrap()
        .permissions()
        .mode()
        & 0o777
}

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
fn the_directory_is_made_for_its_owner_only(empty: Machine) {
    let store = prepared(&empty);

    assert_eq!(mode(&empty, STATE), 0o700);
    assert!(store.prepare().is_ok(), "a second time");
    assert_eq!(store.saved().unwrap(), []);
}

#[rstest]
fn a_directory_that_is_not_the_administrators_is_refused(empty: Machine) {
    let store = prepared(&empty);

    empty.chmod(STATE, 0o777);
    assert!(refused(&store.prepare()));
    assert!(refused(&store.saved()));
    empty.put("file", "x\n");
    assert!(refused(&Store::new(&empty.join("file")).prepare()));
    symlink(empty.join("run"), empty.join("link")).unwrap();
    assert!(refused(&Store::new(&empty.join("link")).prepare()));
    assert!(
        Store::new(&empty.join("none/below"))
            .prepare()
            .unwrap_err()
            .is_missing(),
        "only the directory itself is made"
    );
}

#[rstest]
fn notes_are_kept_until_cleared(empty: Machine) {
    let store = prepared(&empty);

    store.save(1, 12).unwrap();
    store.save(0, 10).unwrap();
    store.save(1, 13).unwrap();
    assert_eq!(store.saved().unwrap(), [(0, 10), (1, 13)]);
    assert_eq!(Store::new(&empty.join(STATE)).saved().unwrap().len(), 2);
    assert_eq!(fs::read_dir(empty.join(STATE)).unwrap().count(), 2);
    assert_eq!(mode(&empty, "run/asus-ux8406-tcc-guard/package-1"), 0o600);
    store.clear(0).unwrap();
    store.clear(0).unwrap();
    assert_eq!(store.saved().unwrap(), [(1, 13)]);
}

#[rstest]
fn a_note_that_was_begun_and_left_is_written_over(empty: Machine) {
    let store = prepared(&empty);

    empty.put("run/asus-ux8406-tcc-guard/fresh", "1");
    store.save(0, 10).unwrap();
    assert_eq!(store.saved().unwrap(), [(0, 10)]);
}

#[rstest]
fn a_directory_that_is_closed_fails_what_needs_it(empty: Machine) {
    let store = prepared(&empty);

    ordinary_user();
    store.save(0, 10).unwrap();
    // Nothing may be added or removed.
    empty.chmod(STATE, 0o500);
    assert!(denied(&store.save(1, 11)));
    assert!(denied(&store.clear(0)));
    assert!(store.clear(1).is_ok(), "nothing to remove");
    assert_eq!(store.saved().unwrap(), [(0, 10)]);
    // Nothing may be listed.
    empty.chmod(STATE, 0o300);
    assert!(denied(&store.saved()));
}

#[rstest]
fn no_directory_means_no_notes(empty: Machine) {
    assert_eq!(Store::new(&empty.join("none")).saved().unwrap(), []);
}

#[rstest]
fn other_files_are_passed_over(empty: Machine) {
    let store = prepared(&empty);

    for name in [
        "fresh",
        "package-",
        "package-x",
        "package-007",
        "package-1024",
        "other",
    ] {
        empty.put(&format!("run/asus-ux8406-tcc-guard/{name}"), "10\n");
    }
    assert_eq!(store.saved().unwrap(), []);
}

#[rstest]
#[case("64\n")]
#[case("rubbish\n")]
#[case("")]
#[case("10 11\n")]
#[case("10\n11\n")]
fn a_note_that_is_not_an_offset_is_refused(empty: Machine, #[case] text: &str) {
    let store = prepared(&empty);

    empty.put("run/asus-ux8406-tcc-guard/package-3", text);
    assert!(matches!(store.saved(), Err(Error::BadValue(_))));
}

#[rstest]
fn a_note_that_others_may_write_is_refused(empty: Machine) {
    let store = prepared(&empty);

    empty.put("run/asus-ux8406-tcc-guard/package-3", "12\n");
    empty.chmod("run/asus-ux8406-tcc-guard/package-3", 0o666);
    assert!(refused(&store.saved()));
}

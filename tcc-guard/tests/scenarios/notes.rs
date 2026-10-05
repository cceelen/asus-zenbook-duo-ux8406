//! Notes of changed offsets that the program must not believe.

use std::os::unix::fs::symlink;

use rstest::rstest;

use crate::common::STATE;
use crate::world::World;

const NOTE: &str = "run/asus-ux8406-tcc-guard/package-0";

/// A machine on which the guard has run once, so that its directory of
/// notes is there.
fn world() -> World {
    let mut world = World::new();

    world.expect_start("without notes");
    world
}

#[rstest]
#[case("rubbish\n")]
#[case("64\n")]
#[case("")]
#[case("10\n11\n")]
fn a_note_that_is_not_an_offset(#[case] text: &str) {
    let mut world = world();

    world.machine.put(NOTE, text);
    world.expect_refusal("a note that is not an offset", &[]);
    assert_eq!(world.run(&["restore"]), Some(1));
    assert_eq!(world.offset(0), Some(10));
    world.machine.remove(NOTE);
    world.expect_start("with the note gone");
}

#[test]
fn a_note_that_is_not_the_administrators() {
    let mut world = world();

    world.machine.put(NOTE, "12\n");
    world.machine.chmod(NOTE, 0o666);
    world.expect_refusal("a note that others may write", &[]);
    world.machine.remove(NOTE);
    world.machine.put("elsewhere", "12\n");
    symlink(world.machine.join("elsewhere"), world.machine.join(NOTE)).unwrap();
    world.expect_refusal("a note that is a symbolic link", &[]);
    assert_eq!(world.run(&["restore"]), Some(1));
    assert_eq!(world.offset(0), Some(10));
}

#[test]
fn a_directory_of_notes_that_is_not_the_administrators() {
    let mut world = world();

    world.machine.chmod(STATE, 0o777);
    world.expect_refusal("a directory of notes that others may write", &[]);
    assert_eq!(world.run(&["restore"]), Some(1));
    world.machine.chmod(STATE, 0o700);

    std::fs::remove_dir_all(world.machine.join(STATE)).unwrap();
    symlink(world.machine.join("etc"), world.machine.join(STATE)).unwrap();
    world.expect_refusal("a symbolic link for the directory of notes", &[]);
}

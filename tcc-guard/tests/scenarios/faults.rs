//! Failures of the system that no arrangement of files brings about,
//! injected at the fail points of the program (`--features failpoints`).

use rstest::rstest;

use crate::world::{EVENT_RECORD, Kind, RAISED, World, step};

/// What must fail before the guard has touched anything.
#[rstest]
#[case::the_signals_cannot_be_set_up("signals=return", "signals: ")]
#[case::the_configuration_cannot_be_looked_at(
    "trusted-stat=return",
    "etc/asus-ux8406-tcc-guard.toml: "
)]
#[case::a_sysfs_file_cannot_be_looked_at("sysfs-stat=return", "product_name: ")]
fn refused(#[case] faults: &'static str, #[case] said: &str) {
    let mut world = World::new();

    world.faults = Some(faults);
    world.expect_refusal(faults, &[]);
    world.expect_log(said);
    world.faults = None;
    world.expect_start("without the fault");
}

/// The notes cannot be listed: the guard does not start as if there were
/// none, and `restore` does not claim that there is nothing to put back.
#[test]
fn the_notes_cannot_be_listed() {
    let mut world = World::new();

    world.expect_start("without notes");
    world
        .machine
        .put("run/asus-ux8406-tcc-guard/package-0", "10\n");
    world.faults = Some("store-list=return");
    world.expect_refusal("store-list", &[]);
    assert_eq!(world.run(&["restore"]), Some(1));
    world.expect_no_log("no offset to put back");
}

/// The note of the offset found cannot be written: the offset is not
/// changed.
#[test]
fn the_note_cannot_be_written() {
    let mut world = World::new();
    let steps = [step(None, 1000, Kind::Record(EVENT_RECORD))];

    world.faults = Some("store-write=return");
    assert_eq!(world.play(&steps, &[]), Some(1));
    world.expect_log("package 0: EC event at 95 C, offset 10");
    world.expect_log("package 0: stopping: cannot read or write");
    world.expect_no_log(RAISED);
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

/// Waiting for the kernel log fails: the guard would be blind, and stops.
#[test]
fn waiting_for_the_log_fails() {
    let mut world = World::new();

    world.faults = Some("kmsg-poll=return");
    assert_eq!(world.play(&[], &[]), Some(1));
    world.expect_log("package 0: armed");
    world.expect_log("stopping: ");
    world.expect_log("package 0: stopping: offset untouched");
}

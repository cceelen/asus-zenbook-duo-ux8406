//! The machine fails the guard while it runs.

use crate::world::{EVENT_RECORD, HOLDING, Kind, RAISED, World, step};

/// A temperature that cannot be trusted any more: stop, put the offset back,
/// and report a failure.
#[test]
fn failing_sensor() {
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(HOLDING), 300, Kind::BreakSensor(0)),
    ];

    assert_eq!(world.play(&steps, &[]), Some(1));
    world.expect_log("package 0: stopping: cannot read or write");
    world.expect_log("package 0: stopping: offset back to 10");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

/// The sensor disappears while nothing is going on: stop and report a
/// failure; there is no offset to put back.
#[test]
fn sensor_gone() {
    let mut world = World::new();
    let steps = [step(None, 1500, Kind::RemoveSensor(0))];

    assert_eq!(world.play(&steps, &[]), Some(1));
    world.expect_log("package 0: stopping: cannot read or write");
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

/// The offset can be read but no longer written when the event comes: stop,
/// say that it could not be put back either, and report a failure. The note
/// of the offset stays until it has been acted on.
#[test]
fn write_fails() {
    let mut world = World::new();
    let steps = [
        step(None, 300, Kind::LockOffset(0)),
        step(None, 1200, Kind::Record(EVENT_RECORD)),
    ];

    assert_eq!(world.play(&steps, &[]), Some(1));
    world.expect_log("package 0: EC event at 95 C, offset 10");
    world.expect_log("package 0: stopping: cannot read or write");
    world.expect_log("package 0: stopping: could not put the offset back to 10");
    world.expect_no_log(RAISED);
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), [("package-0".to_owned(), Some(10))]);

    assert_eq!(world.run(&["restore"]), Some(1));
    assert_eq!(world.notes().len(), 1);
    world.machine.chmod(crate::common::OFFSET, 0o600);
    assert_eq!(world.run(&["restore"]), Some(0));
    assert_eq!(world.notes(), []);
}

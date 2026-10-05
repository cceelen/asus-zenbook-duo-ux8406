//! What the guard does with the offset between an event and the release.

use rustix::process::Signal;

use crate::world::{EVENT_RECORD, HOLDING, Kind, LOWERED, RAISED, World, step};

/// The load ends: the offset comes down by itself, and at the end of the
/// running time there is nothing left to put back.
#[test]
fn release() {
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(RAISED), 1500, Kind::Unload(0)),
        step(Some("released, back to 10"), 100, Kind::ExpectOffset(0, 10)),
    ];

    assert_eq!(world.play(&steps, &["-t", "20"]), Some(0));
    world.expect_log(HOLDING);
    world.expect_log("60 C, offset not in use: released, back to 10");
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

/// A second event while an offset is held, and something else lowering the
/// offset behind the program's back.
#[test]
fn trip_and_interference() {
    const INTERFERED: &str = "offset was lowered by something else";
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(HOLDING), 200, Kind::SetOffset(0, 10)),
        step(Some(INTERFERED), 200, Kind::ExpectRaised(0, 10)),
        step(Some(INTERFERED), 400, Kind::Record(EVENT_RECORD)),
        step(
            Some("the next step down waits 6 s"),
            300,
            Kind::Signal(Signal::INT),
        ),
    ];

    assert_eq!(world.play(&steps, &[]), Some(0));
    world.expect_log("event while holding: going back to offset");
    world.expect_log("package 0: stopping: offset back to 10");
    assert_eq!(world.offset(0), Some(10));
}

/// Two packages, one of them idle: the event reaches both, but only the hot
/// one has its offset raised, and each keeps to its own file.
#[test]
fn two_packages() {
    let mut world = World::two_packages();
    let steps = [
        step(None, 0, Kind::Unload(1)),
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(RAISED), 300, Kind::ExpectRaised(0, 10)),
        step(Some(RAISED), 300, Kind::ExpectOffset(1, 10)),
        step(
            Some("package 1: 60 C, offset not in use: released"),
            0,
            Kind::ExpectOffset(1, 10),
        ),
        step(Some(LOWERED), 300, Kind::Signal(Signal::HUP)),
    ];

    assert_eq!(world.play(&steps, &[]), Some(0));
    world.expect_log("package 1: armed: offset 10");
    world.expect_log("package 1: EC event at 60 C, offset 10");
    world.expect_log("package 1: 60 C, alert cleared: holding offset 10");
    world.expect_no_log("package 1: 60 C, above");
    world.expect_log("package 0: stopping: offset back to 10");
    world.expect_log("package 1: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.offset(1), Some(10));
}

/// The steps down reach the offset found at the event, and that one
/// survives its wait as well: released, with nothing to put back.
#[test]
fn walk_down_to_the_release() {
    const RELEASED: &str = "no event at the original offset: released at 10";
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(RELEASED), 300, Kind::Signal(Signal::TERM)),
    ];

    // 95 C is above 94: one step up to 12, which gives 93 C.
    assert_eq!(world.play(&steps, &["-H", "94"]), Some(0));
    world.expect_log("package 0: 93 C, alert cleared: holding offset 12");
    world.expect_log("offset lowered to 11");
    world.expect_log("offset lowered to 10");
    world.expect_log(RELEASED);
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.notes(), []);
}

/// A machine that stays too hot whatever the offset: the guard stops raising
/// at the maximum.
#[test]
fn the_maximum_offset() {
    const AT_MAX: &str = "above the hold temperature at the maximum offset 13";
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(AT_MAX), 300, Kind::Signal(Signal::TERM)),
    ];

    world
        .machine
        .append(crate::common::CONFIG, "max_offset = 13\n");
    assert_eq!(world.play(&steps, &[]), Some(0));
    world.expect_log("offset raised to 12");
    world.expect_log("offset raised to 13");
    world.expect_no_log("offset raised to 14");
    world.expect_log("package 0: stopping: offset back to 10");
    assert_eq!(world.offset(0), Some(10));
}

//! The guard is stopped, or killed, while it holds an offset.

use rstest::rstest;
use rustix::process::Signal;

use crate::world::{EVENT_RECORD, HOLDING, Kind, LOWERED, PUT_BACK, RAISED, World, step};

/// Event under load: raise, hold, one step down; then a signal ends it and
/// the offset goes back. The signals are the ones that stop a service, a
/// program on a terminal, and one whose terminal has gone.
#[rstest]
#[case(Signal::TERM)]
#[case(Signal::INT)]
#[case(Signal::HUP)]
fn a_signal_puts_the_offset_back(#[case] signal: Signal) {
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(HOLDING), 100, Kind::ExpectRaised(0, 10)),
        step(Some(LOWERED), 300, Kind::Signal(signal)),
    ];

    assert_eq!(world.play(&steps, &[]), Some(0));
    world.expect_log("package 0: armed: offset 10, CPU 95 C, tjmax 105");
    world.expect_log("package 0: EC event at 95 C, offset 10");
    world.expect_log("offset raised to 12");
    world.expect_log("package 0: stopping: offset back to 10");
    world.expect_no_log("package 1");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

/// A signal while nothing is held ends the guard at once.
#[test]
fn a_signal_without_an_offset_held_leaves_it_untouched() {
    let mut world = World::new();
    let steps = [step(Some("armed"), 300, Kind::Signal(Signal::TERM))];

    assert_eq!(world.play(&steps, &[]), Some(0));
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
}

/// The program killed outright, as on running out of memory: the offset
/// stays where it was, its note tells what it was, and `restore` puts it
/// back.
#[test]
fn restore_puts_back_what_a_killed_guard_left() {
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(RAISED), 100, Kind::Signal(Signal::KILL)),
    ];

    assert_eq!(world.play(&steps, &[]), None);
    world.expect_no_log("stopping");
    assert!(world.offset(0).is_some_and(|offset| offset > 10));
    assert_eq!(world.notes(), [("package-0".to_owned(), Some(10))]);

    assert_eq!(world.run(&["restore"]), Some(0));
    world.expect_log(PUT_BACK);
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);

    assert_eq!(world.run(&["restore"]), Some(0));
    world.expect_log("no offset to put back");
    assert_eq!(world.offset(0), Some(10));
}

/// The same, put back by the next start before anything else.
#[test]
fn the_next_start_puts_back_what_a_killed_guard_left() {
    let mut world = World::new();
    let steps = [
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(Some(HOLDING), 100, Kind::Signal(Signal::KILL)),
    ];

    assert_eq!(world.play(&steps, &[]), None);
    assert!(world.offset(0).is_some_and(|offset| offset > 10));

    assert_eq!(world.run(&["-t", "1"]), Some(0));
    world.expect_log(PUT_BACK);
    // The temperature is still the one of the raised offset.
    world.expect_log("package 0: armed: offset 10, CPU ");
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

/// Two packages, only one of them hot: only that one is put back.
#[test]
fn restore_keeps_to_the_package_that_was_held() {
    let mut world = World::two_packages();
    let steps = [
        step(None, 0, Kind::Unload(1)),
        step(None, 1000, Kind::Record(EVENT_RECORD)),
        step(
            Some("package 1: 60 C, offset not in use: released"),
            100,
            Kind::Signal(Signal::KILL),
        ),
    ];

    assert_eq!(world.play(&steps, &[]), None);
    assert_eq!(world.notes(), [("package-0".to_owned(), Some(10))]);

    assert_eq!(world.run(&["restore"]), Some(0));
    world.expect_log(PUT_BACK);
    world.expect_no_log("package 1");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.offset(1), Some(10));
}

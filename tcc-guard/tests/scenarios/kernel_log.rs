//! The kernel log: what counts as the event, and logs that say nothing.

use std::fs::OpenOptions;

use crate::common::{CONFIG, KMSG};
use crate::world::{Kind, World, hwmon, step};

/// A kernel log that delivers nothing, as a pipe nobody writes to: the
/// program keeps running, and ends when its time is up.
#[test]
fn quiet_log() {
    let mut world = World::new();

    world.machine.fifo(KMSG);
    assert_eq!(world.play(&[], &["-t", "3"]), Some(0));
    world.expect_log("package 0: armed: offset 10");
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
}

/// The same with a writer that never writes: reading waits, as on the
/// kernel log while nothing happens.
#[test]
fn quiet_log_with_a_writer() {
    let mut world = World::new();

    world.machine.fifo(KMSG);
    let _writer = OpenOptions::new()
        .read(true)
        .write(true)
        .open(world.machine.join(KMSG))
        .unwrap();

    assert_eq!(world.play(&[], &["-t", "3"]), Some(0));
    world.expect_log("package 0: stopping: offset untouched");
}

/// A log that cannot be read while the guard runs: it would never see the
/// event, so it stops and reports a failure.
#[test]
fn a_log_that_fails() {
    let mut world = World::new();

    world.machine.remove(KMSG);
    std::fs::create_dir(world.machine.join(KMSG)).unwrap();
    assert_eq!(world.play(&[], &[]), Some(1));
    world.expect_log("package 0: armed");
    world.expect_log("stopping: ");
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
}

/// Kernel log records that must not count as the event, and a configured
/// tjmax in place of one the kernel does not give.
#[test]
fn not_an_event() {
    let mut world = World::new();
    let steps = [
        step(None, 600, Kind::Record("14,1,1,-;test: thermal warning\n")),
        step(
            None,
            1000,
            Kind::Record("6,2,2,-;other\n DEVICE=test: thermal warning\n"),
        ),
        step(
            None,
            1400,
            Kind::Record("6,3,3,-;a test: thermal warning\n"),
        ),
        step(None, 1800, Kind::Record("test: thermal warning\n")),
    ];

    world.machine.put(&hwmon("temp1_crit"), "rubbish\n");
    world.machine.append(CONFIG, "tjmax = 100\n");
    assert_eq!(world.play(&steps, &["-t", "4"]), Some(0));
    world.expect_log("package 0: armed: offset 10, CPU 95 C, tjmax 100");
    world.expect_no_log("EC event");
    world.expect_log("package 0: stopping: offset untouched");
    assert_eq!(world.offset(0), Some(10));
    assert_eq!(world.notes(), []);
}

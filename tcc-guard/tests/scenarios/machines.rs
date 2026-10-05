//! Machines that the program must not start on.

use std::os::unix::fs::symlink;

use rstest::rstest;

use crate::common::{CONFIG, KMSG, OFFSET, PRODUCT_NAME, TEMP};
use crate::world::{World, hwmon, usual_config};

/// Temperature files that are not what they should be.
#[rstest]
#[case::not_a_number(b"rubbish\n")]
#[case::not_plausible(b"999000\n")]
#[case::empty(b"")]
#[case::too_long(b"9500000000000000000000000000000000000000\n")]
#[case::a_nul_in_it(b"95\x00000\n")]
fn refused_temperature(#[case] content: &[u8]) {
    let mut world = World::new();

    world.machine.put(TEMP, content);
    world.expect_refusal("a temperature file that is not one", &[]);
    world.machine.put(TEMP, "95000\n");
    world.expect_start("with the temperature restored");
}

#[test]
fn no_temperature_file_or_a_pipe() {
    let mut world = World::new();

    world.machine.remove(TEMP);
    world.expect_refusal("no temperature file", &[]);
    world.machine.fifo(TEMP);
    world.expect_refusal("a pipe for a temperature file", &[]);
}

#[test]
fn an_offset_that_is_not_plausible_is_left_as_it_is() {
    let mut world = World::new();

    world.machine.put(OFFSET, "99\n");
    world.frozen = true;
    assert_eq!(world.run(&[]), Some(1));
    world.expect_no_log("armed");
    assert_eq!(world.offset(0), Some(99));
}

#[test]
fn an_offset_file_that_cannot_be_written_or_is_not_there() {
    let mut world = World::new();
    let away = world.machine.join("away");

    world.machine.chmod(OFFSET, 0o400);
    world.expect_refusal("an offset file that cannot be written", &[]);
    world.expect_log("(needs root)");
    world.machine.chmod(OFFSET, 0o600);

    std::fs::rename(world.machine.join(OFFSET), &away).unwrap();
    world.frozen = true;
    assert_eq!(world.run(&[]), Some(1), "no offset file");
    world.expect_no_log("armed");
    std::fs::rename(&away, world.machine.join(OFFSET)).unwrap();
    world.expect_start("with the files restored");
}

#[test]
fn limits_labels_and_logs() {
    let mut world = World::new();
    let (crit, label) = (hwmon("temp1_crit"), hwmon("temp1_label"));

    world.machine.put(&crit, "999000\n");
    world.expect_refusal("a limit that is not plausible", &[]);
    world.expect_log("set tjmax");
    world.machine.put(&crit, "105000\n");
    world.machine.put(&label, "Core 0\n");
    world.expect_refusal("no package sensor", &[]);
    world.expect_log("no coretemp package temperature sensor");
    world.machine.put(&label, "Package id 0\n");
    world.machine.remove(KMSG);
    world.expect_refusal("no kernel log", &[]);
    world.machine.put(KMSG, "");
    world.expect_start("with the machine restored");
}

#[test]
fn packages_that_are_not_accounted_for() {
    let mut world = World::new();

    world.machine.sensor(5, 1, 95_000);
    world.expect_refusal("two packages and no file for their offsets", &[]);
    world.expect_log("package 0: name the file of its offset in a [[package]] table");
    // With a file named for package 0, so that this is the only cause.
    world
        .machine
        .put("sys/class/hwmon/hwmon5/temp1_label", "Package id 0\n");
    world.machine.append(
        CONFIG,
        "[[package]]
id = 0
\
         offset = \"/sys/bus/pci/devices/0000:00:04.0/tcc_offset_degree_celsius\"
",
    );
    world.expect_refusal("two sensors for one package", &[]);
    world.expect_log("package 0 has two temperature sensors");
}

#[test]
fn nine_packages() {
    let mut world = World::new();

    for package in 1..=8 {
        world
            .machine
            .sensor(10 + u32::try_from(package).unwrap(), package, 60_000);
    }
    world.expect_refusal("nine packages", &[]);
    world.expect_log("more than 8 packages");
}

/// A machine of another model: the program ends at once and touches
/// nothing, without calling that a failure, unless the configuration says
/// that the guard is wanted there.
#[test]
fn another_model() {
    let mut world = World::new();

    world
        .machine
        .put(PRODUCT_NAME, "ASUS Zenbook 14 UX3405MA_UX3405MA\n");
    world.frozen = true;
    assert_eq!(world.run(&[]), Some(0), "{}", world.text);
    world.expect_log("this machine is \"ASUS Zenbook 14 UX3405MA_UX3405MA\", not a UX8406");
    world.expect_log("nothing to do");
    world.expect_no_log("armed");
    assert!(
        !world.machine.join(crate::common::STATE).exists(),
        "nothing was made on it"
    );

    world.machine.append(CONFIG, "model = \"UX3405MA\"\n");
    world.expect_start("with that model named");
    world.machine.remove(PRODUCT_NAME);
    world.expect_refusal("a machine without a name", &[]);
    world.machine.put(CONFIG, usual_config());
    world.machine.append(CONFIG, "model = \"any\"\n");
    world.expect_start("with any model allowed");
}

/// The root of the system behind a symbolic link with a long name: the
/// program starts there.
#[test]
fn a_long_name_for_the_root() {
    let mut world = World::new();
    let deep = world.machine.join(&"d".repeat(120));
    let deep = deep.to_str().unwrap();

    symlink(world.machine.root(), deep).unwrap();
    world.frozen = true;
    assert_eq!(world.run(&["--root", deep, "-t", "1"]), Some(0));
    world.expect_log("package 0: armed");
}

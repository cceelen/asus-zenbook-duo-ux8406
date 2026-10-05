//! Configuration files that the program must not accept.

use std::os::unix::fs::symlink;

use rstest::rstest;

use crate::common::{CONFIG, CONFIG_TWO};
use crate::world::{World, usual_config};

/// The usual configuration with one more line, which must make it refused.
#[rstest]
#[case::a_value_that_is_not_a_number("hold_temp = \"eighty\"\n")]
#[case::a_value_out_of_its_range("hold_temp = 200\n")]
#[case::an_unknown_name("release_temp = 80\n")]
#[case::a_name_set_twice("settle = 3\n")]
#[case::settings_that_contradict_each_other("probe_max = 1\n")]
#[case::an_offset_file_outside_sys("[[package]]\nid = 0\noffset = \"/etc/passwd\"\n")]
#[case::not_toml("hold_temp 85\n")]
#[case::a_package_that_is_not_there(CONFIG_TWO)]
fn refused_content(#[case] line: &str) {
    let mut world = World::new();

    world.machine.append(CONFIG, line);
    world.expect_refusal(line, &[]);
    world.machine.put(CONFIG, usual_config());
    world.expect_start("with the configuration restored");
}

#[test]
fn a_file_that_is_too_big() {
    let mut world = World::new();
    let filler = format!(
        "# a file that is too big, line by line {}\n",
        ".".repeat(117)
    );

    world.machine.append(CONFIG, &filler.repeat(200));
    world.expect_refusal("a file that is too big", &[]);
}

/// An offset file whose configured name is longer than a name below `/sys`
/// may be; the longest one is taken, and then not found.
#[test]
fn an_offset_file_whose_name_is_too_long() {
    let mut world = World::new();
    let table = |length: usize| {
        format!(
            "[[package]]\nid = 0\noffset = \"/sys/{}\"\n",
            "a".repeat(length)
        )
    };

    world.machine.append(CONFIG, &table(158));
    world.expect_refusal("an offset file that is not there", &[]);
    world.expect_no_log("its offset has to be");
    world.machine.put(CONFIG, usual_config());
    world.machine.append(CONFIG, &table(159));
    world.expect_refusal("an offset file whose name is too long", &[]);
    world.expect_log("package 0: its offset has to be a file below /sys");
}

/// What is wrong with the file is said, with the place where that can be
/// told.
#[test]
fn a_refusal_names_the_file_and_the_fault() {
    let mut world = World::new();

    world.machine.append(CONFIG, "release_temp = 80\n");
    world.expect_refusal("an unknown name", &[]);
    world.expect_log("etc/asus-ux8406-tcc-guard.toml: ");
    world.expect_log("line 7");
    world.expect_log("release_temp");
}

/// Files that must not even be read, as the usual file and as a named one.
#[rstest]
#[case::the_usual_file(false)]
#[case::a_named_file(true)]
fn refused_files(#[case] named: bool) {
    let mut world = World::new();
    let conf = world.path(CONFIG);
    let arguments: &[&str] = if named { &["-c", &conf] } else { &[] };
    let real = world.machine.join("etc/real");

    world.machine.chmod(CONFIG, 0o666);
    world.expect_refusal("a file that others may write", arguments);
    world.machine.chmod(CONFIG, 0o600);
    world.expect_start("with the permissions restored");
    std::fs::rename(&conf, &real).unwrap();
    symlink(&real, &conf).unwrap();
    world.expect_refusal("a symbolic link", arguments);
    world.machine.remove(CONFIG);
    std::fs::create_dir(&conf).unwrap();
    world.expect_refusal("a directory", arguments);
    std::fs::remove_dir(&conf).unwrap();
    world.machine.fifo(CONFIG);
    world.expect_refusal("a pipe", arguments);
}

/// A file that is named has to be there; the usual one may be missing, and
/// the defaults then apply.
#[test]
fn a_missing_file() {
    let mut world = World::new();
    let conf = world.path(CONFIG);

    world.machine.remove(CONFIG);
    world.expect_refusal("a named file that is missing", &["-c", &conf]);
    world.expect_start("without the usual file");
    world.expect_log("interval 4, settle 20");
    world.expect_log("event text: \"asus_wmi: Unknown key code 0x6d\"");
}

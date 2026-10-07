//! How the command line is read.

use std::path::Path;

use asus_ux8406_keyboard_state::{Action, Bpffs, Cli};
use clap::Parser;
use rstest::rstest;

use crate::common::{BLUETOOTH, USB, device};

fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from([&["asus-ux8406-keyboard-state"], arguments].concat())
}

#[rstest]
#[case::save("save", Action::Save)]
#[case::load("load", Action::Load)]
fn the_action_and_the_device_are_taken(#[case] word: &str, #[case] action: Action) {
    let cli = parse(&[word, USB]).unwrap();

    assert_eq!(cli.action, action);
    assert_eq!(cli.device, device(USB));
}

#[test]
fn the_system_places_are_the_defaults() {
    let cli = parse(&["save", USB]).unwrap();

    assert_eq!(cli.bpffs(), Bpffs::new(Path::new("/sys/fs/bpf")));
    assert_eq!(cli.state, Path::new("/run/asus-ux8406-keyboard-state"));
}

#[rstest]
#[case::short(&["-b", "/tmp/bpf", "-s", "/tmp/state", "load", BLUETOOTH])]
#[case::long(&["--bpffs", "/tmp/bpf", "--state", "/tmp/state", "load", BLUETOOTH])]
#[case::after_the_action(&["load", BLUETOOTH, "-b", "/tmp/bpf", "-s", "/tmp/state"])]
#[case::given_twice(&["-b", "/x", "-b", "/tmp/bpf", "-s", "/y", "-s", "/tmp/state", "load", BLUETOOTH])]
fn other_places_can_be_given(#[case] arguments: &[&str]) {
    let cli = parse(arguments).unwrap();

    assert_eq!(cli.bpffs(), Bpffs::new(Path::new("/tmp/bpf")));
    assert_eq!(cli.state, Path::new("/tmp/state"));
}

#[rstest]
#[case::nothing(&[])]
#[case::no_device(&["save"])]
#[case::no_action(&[USB])]
#[case::unknown_action(&["restore", USB])]
#[case::another_device(&["save", "0003:056A:0357.0007"])]
#[case::a_path_for_a_device(&["save", "../../etc"])]
#[case::two_devices(&["save", USB, BLUETOOTH])]
#[case::unknown_option(&["-x", "save", USB])]
#[case::option_without_value(&["save", USB, "-s"])]
fn anything_else_is_refused(#[case] arguments: &[&str]) {
    assert!(parse(arguments).is_err());
}

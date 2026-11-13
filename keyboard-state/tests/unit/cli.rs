//! How the command line is read.

use std::path::Path;

use asus_ux8406_keyboard_state::cli::Kind;
use asus_ux8406_keyboard_state::{Bpffs, Cli, Command, Phase};
use clap::Parser;
use rstest::rstest;

use crate::common::{BLUETOOTH, USB, device};

fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from([&["asus-ux8406-keyboard-state"], arguments].concat())
}

#[rstest]
#[case::save("save", Command::Save { device: device(USB) })]
#[case::load("load", Command::Load { device: device(USB) })]
fn the_action_and_the_device_are_taken(#[case] word: &str, #[case] command: Command) {
    assert_eq!(parse(&[word, USB]).unwrap().command, command);
}

#[rstest]
#[case::pre(&["sleep", "pre"], Phase::Pre, None)]
#[case::post(&["sleep", "post"], Phase::Post, None)]
#[case::suspend(&["sleep", "pre", "suspend"], Phase::Pre, Some(Kind::Suspend))]
#[case::hibernate(&["sleep", "post", "hibernate"], Phase::Post, Some(Kind::Hibernate))]
#[case::hybrid(&["sleep", "pre", "hybrid-sleep"], Phase::Pre, Some(Kind::HybridSleep))]
#[case::then(
    &["sleep", "post", "suspend-then-hibernate"],
    Phase::Post,
    Some(Kind::SuspendThenHibernate)
)]
fn sleep_takes_the_words_of_a_systemd_hook(
    #[case] arguments: &[&str],
    #[case] phase: Phase,
    #[case] kind: Option<Kind>,
) {
    assert_eq!(
        parse(arguments).unwrap().command,
        Command::Sleep { phase, kind }
    );
}

#[test]
fn the_system_places_are_the_defaults() {
    let cli = parse(&["save", USB]).unwrap();

    assert_eq!(cli.bpffs(), Bpffs::new(Path::new("/sys/fs/bpf")));
    assert_eq!(cli.state, Path::new("/run/asus-ux8406-keyboard-state"));
    assert_eq!(cli.sysfs, Path::new("/sys"));
    assert_eq!(cli.dev, Path::new("/dev"));
}

#[rstest]
#[case::short(&["-b", "/tmp/bpf", "-s", "/tmp/state", "-y", "/tmp/sys", "-d", "/tmp/dev", "load", BLUETOOTH])]
#[case::long(&["--bpffs", "/tmp/bpf", "--state", "/tmp/state", "--sysfs", "/tmp/sys", "--dev", "/tmp/dev", "load", BLUETOOTH])]
#[case::after_the_action(&["load", BLUETOOTH, "-b", "/tmp/bpf", "-s", "/tmp/state", "-y", "/tmp/sys", "-d", "/tmp/dev"])]
#[case::after_sleep(&["sleep", "pre", "-b", "/tmp/bpf", "-s", "/tmp/state", "-y", "/tmp/sys", "-d", "/tmp/dev"])]
#[case::given_twice(&["-b", "/x", "-b", "/tmp/bpf", "-s", "/y", "-s", "/tmp/state", "-y", "/z", "-y", "/tmp/sys", "-d", "/w", "-d", "/tmp/dev", "load", BLUETOOTH])]
fn other_places_can_be_given(#[case] arguments: &[&str]) {
    let cli = parse(arguments).unwrap();

    assert_eq!(cli.bpffs(), Bpffs::new(Path::new("/tmp/bpf")));
    assert_eq!(cli.state, Path::new("/tmp/state"));
    assert_eq!(cli.sysfs, Path::new("/tmp/sys"));
    assert_eq!(cli.dev, Path::new("/tmp/dev"));
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
#[case::no_phase(&["sleep"])]
#[case::unknown_phase(&["sleep", "now"])]
#[case::unknown_kind(&["sleep", "pre", "reboot"])]
#[case::two_kinds(&["sleep", "pre", "suspend", "hibernate"])]
#[case::device_for_sleep(&["sleep", USB])]
fn anything_else_is_refused(#[case] arguments: &[&str]) {
    assert!(parse(arguments).is_err());
}

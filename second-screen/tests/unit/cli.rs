//! What the command line asks for.

use std::path::Path;

use asus_ux8406_second_screen::{Action, Cli, Mode, Sysfs};
use clap::Parser;
use clap::error::ErrorKind;
use rstest::rstest;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("asus-ux8406-second-screen").chain(args.iter().copied()))
}

/// What the arguments ask for, as the action, the tree and the mode.
fn asked(args: &[&str]) -> (Action, Sysfs, Mode) {
    let cli = parse(args).unwrap();

    (cli.action, cli.sysfs(), cli.mode())
}

#[rstest]
#[case("dock", Action::Dock)]
#[case("brightness", Action::Brightness)]
#[case("release", Action::Release)]
fn an_action_alone_means_the_real_sysfs_and_writing(#[case] word: &str, #[case] action: Action) {
    assert_eq!(asked(&[word]), (action, Sysfs::new("/sys"), Mode::Apply));
}

#[test]
fn the_options_are_taken_in_any_order_and_either_spelling() {
    let wanted = (Action::Dock, Sysfs::new("/tmp/x"), Mode::DryRun);

    assert_eq!(asked(&["-n", "-r", "/tmp/x", "dock"]), wanted);
    assert_eq!(asked(&["dock", "-r", "/tmp/x", "-n"]), wanted);
    assert_eq!(asked(&["--root", "/tmp/x", "dock", "--dry-run"]), wanted);
    assert_eq!(
        Path::new("/tmp/x"),
        parse(&["-r", "/tmp/x", "dock"]).unwrap().root
    );
}

#[rstest]
#[case(&["-h"], ErrorKind::DisplayHelp)]
#[case(&["--help"], ErrorKind::DisplayHelp)]
#[case(&["-V"], ErrorKind::DisplayVersion)]
#[case(&["-n", "dock", "--version"], ErrorKind::DisplayVersion)]
fn help_and_version_win_over_the_other_arguments(#[case] args: &[&str], #[case] kind: ErrorKind) {
    assert_eq!(parse(args).unwrap_err().kind(), kind);
}

#[rstest]
#[case(&[])]
#[case(&["-n"])]
#[case(&["--bogus", "dock"])]
#[case(&["dock", "-r"])]
#[case(&["dock", "brightness"])]
#[case(&["undock"])]
fn anything_else_is_refused(#[case] args: &[&str]) {
    let kind = parse(args).unwrap_err().kind();

    assert_ne!(kind, ErrorKind::DisplayHelp);
    assert_ne!(kind, ErrorKind::DisplayVersion);
}

#[test]
fn the_root_may_look_like_an_option_and_the_last_one_wins() {
    assert_eq!(asked(&["-r", "-n", "dock"]).1, Sysfs::new("-n"));
    assert_eq!(
        asked(&["-r", "/a", "-r", "/b", "-n", "-n", "dock"]),
        (Action::Dock, Sysfs::new("/b"), Mode::DryRun)
    );
}

//! The command line.

use std::path::PathBuf;

use asus_ux8406_tcc_guard::cli::{Action, Cli};
use clap::Parser;
use clap::error::ErrorKind;
use rstest::rstest;

fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("asus-ux8406-tcc-guard").chain(arguments.iter().copied()))
}

#[test]
fn the_command_line_is_well_formed() {
    use clap::CommandFactory;

    Cli::command().debug_assert();
}

#[test]
fn no_arguments_run_the_guard_on_the_machine() {
    let cli = parse(&[]).unwrap();

    assert_eq!(cli.action, None);
    assert_eq!(cli.config, None);
    assert_eq!(cli.hold_temp, None);
    assert_eq!(cli.probe_interval, None);
    assert_eq!(cli.duration, 0);
    assert_eq!(cli.root, PathBuf::from("/"));
}

#[test]
fn options_are_taken_short_or_long_apart_or_attached() {
    let short = parse(&[
        "-c", "/x/conf", "-H", "85", "-P", "60", "-t", "5", "restore",
    ])
    .unwrap();
    let attached = parse(&["-c/x/conf", "-H85", "-P60", "-t5", "restore"]).unwrap();
    let long = parse(&[
        "--config",
        "/x/conf",
        "--hold-temp=85",
        "--probe-interval",
        "60",
        "--time",
        "5",
        "restore",
    ])
    .unwrap();

    assert_eq!(short, attached);
    assert_eq!(short, long);
    assert_eq!(short.action, Some(Action::Restore));
    assert_eq!(short.config, Some(PathBuf::from("/x/conf")));
    assert_eq!(short.hold_temp, Some(85));
    assert_eq!(short.probe_interval, Some(60));
    assert_eq!(short.duration, 5);
    assert_eq!(parse(&["run"]).unwrap().action, Some(Action::Run));
    assert_eq!(
        parse(&["--root", "/mnt"]).unwrap().root,
        PathBuf::from("/mnt")
    );
}

#[rstest]
#[case(&["-h"], ErrorKind::DisplayHelp)]
#[case(&["--help"], ErrorKind::DisplayHelp)]
#[case(&["-V"], ErrorKind::DisplayVersion)]
#[case(&["-t", "5", "--version"], ErrorKind::DisplayVersion)]
fn help_and_version(#[case] arguments: &[&str], #[case] kind: ErrorKind) {
    assert_eq!(parse(arguments).unwrap_err().kind(), kind);
}

/// Numbers are taken as strictly as the configuration file takes them, and
/// within the same ranges.
#[rstest]
#[case(&["-H", "8x"])]
#[case(&["-H", "200"])]
#[case(&["-H", "49"])]
#[case(&["-H", "+85"])]
#[case(&["-H", " 85"])]
#[case(&["-H"])]
#[case(&["-P", "0"])]
#[case(&["-P", "86401"])]
#[case(&["-t", "-1"])]
#[case(&["-t", "1h"])]
#[case(&["-t", "604801"])]
#[case(&["-t", "1", "x"])]
#[case(&["extra"])]
#[case(&["-x"])]
#[case(&["-x", "1"])]
#[case(&["--bogus"])]
#[case(&["run", "restore"])]
#[case(&["-c", "/a", "-c", "/b"])]
#[case(&["--", "-t", "5"])]
#[case(&["--", "restore"])]
#[case(&["restore", "-t", "5"])]
fn anything_else_is_refused(#[case] arguments: &[&str]) {
    let error = parse(arguments).unwrap_err();

    assert!(
        !matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        ),
        "{arguments:?}"
    );
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn the_limits_themselves_are_taken() {
    assert_eq!(parse(&["-H", "50"]).unwrap().hold_temp, Some(50));
    assert_eq!(parse(&["-H", "105"]).unwrap().hold_temp, Some(105));
    assert_eq!(
        parse(&["-P", "86400"]).unwrap().probe_interval,
        Some(86_400)
    );
    assert_eq!(parse(&["-t", "604800"]).unwrap().duration, 604_800);
}

#[test]
fn the_help_names_every_setting_of_the_configuration_file() {
    use clap::CommandFactory;

    let help = Cli::command().render_long_help().to_string();

    for number in asus_ux8406_tcc_guard::input::Number::ALL {
        assert!(help.contains(number.name()), "{}", number.name());
    }
    for word in [
        "event_text",
        "model",
        "package",
        "restore",
        "/etc/asus-ux8406-tcc-guard.toml",
        "--root",
    ] {
        assert!(help.contains(word), "{word}");
    }
}

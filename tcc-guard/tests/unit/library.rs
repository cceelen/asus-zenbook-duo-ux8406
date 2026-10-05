//! What a command line leads to.

use std::sync::atomic::{AtomicBool, Ordering};

use asus_ux8406_tcc_guard::cli::Cli;
use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::input::Settings;
use asus_ux8406_tcc_guard::paths::Paths;
use asus_ux8406_tcc_guard::store::Store;
use asus_ux8406_tcc_guard::{execute, settings, stop_on_signals};
use clap::Parser;
use rstest::rstest;

use crate::common::{CONFIG, KMSG, Machine, OFFSET, PRODUCT_NAME, STATE, empty, machine};

/// The command line with these arguments, on the machine.
fn cli(machine: &Machine, arguments: &[&str]) -> Cli {
    let root = machine.root().to_str().unwrap();

    Cli::try_parse_from(
        ["asus-ux8406-tcc-guard", "--root", root]
            .into_iter()
            .chain(arguments.iter().copied()),
    )
    .unwrap()
}

/// Told to stop before it starts: the guard arms, and stops.
fn run(machine: &Machine, arguments: &[&str]) -> Result<bool, Error> {
    execute(&cli(machine, arguments), &AtomicBool::new(true))
}

#[test]
fn the_system_is_below_the_root() {
    let cli = Cli::try_parse_from(["asus-ux8406-tcc-guard"]).unwrap();
    let paths = Paths::below(&cli.root);

    assert_eq!(paths.sysfs.to_str(), Some("/sys"));
    assert_eq!(paths.kmsg.to_str(), Some("/dev/kmsg"));
    assert_eq!(paths.state.to_str(), Some("/run/asus-ux8406-tcc-guard"));
    assert_eq!(
        paths.config.to_str(),
        Some("/etc/asus-ux8406-tcc-guard.toml")
    );
}

#[rstest]
fn the_command_line_comes_after_the_file(machine: Machine) {
    let cli = cli(&machine, &["-H", "85", "-P", "60"]);
    let settings = settings(&cli, &machine.paths()).unwrap();

    assert_eq!(settings.guard.interval, 1, "from the file");
    assert_eq!(settings.guard.hold_temp, 85);
    assert_eq!(settings.guard.probe_interval, 60);
}

#[rstest]
fn the_usual_file_may_be_missing_but_a_named_one_may_not(machine: Machine, empty: Machine) {
    let named = machine.put("named", "settle = 7\n");
    let named = named.to_str().unwrap();

    assert_eq!(
        settings(&cli(&empty, &[]), &empty.paths()).unwrap(),
        Settings::default()
    );
    // A file that is named stands alone.
    let alone = settings(&cli(&machine, &["-c", named]), &machine.paths()).unwrap();
    assert_eq!(alone.guard.settle, 7);
    assert_eq!(alone.guard.interval, Settings::default().guard.interval);

    machine.remove("named");
    assert!(
        settings(&cli(&machine, &["-c", named]), &machine.paths())
            .unwrap_err()
            .is_missing()
    );
    machine.put(CONFIG, "bogus\n");
    assert!(matches!(
        settings(&cli(&machine, &[]), &machine.paths()),
        Err(Error::BadConfig { .. })
    ));
}

#[rstest]
fn settings_that_do_not_go_together_are_refused(machine: Machine) {
    machine.put(CONFIG, "interval = 10\nprobe_interval = 5\n");
    assert!(matches!(
        settings(&cli(&machine, &[]), &machine.paths()),
        Err(Error::Contradiction)
    ));
    machine.put(CONFIG, "interval = 10\n");
    assert!(matches!(
        settings(&cli(&machine, &["-P", "5"]), &machine.paths()),
        Err(Error::Contradiction)
    ));
}

#[rstest]
fn the_guard_arms_and_stops_untouched(machine: Machine) {
    assert!(run(&machine, &[]).unwrap());
    assert_eq!(machine.number(OFFSET), Some(10));
    assert!(machine.join(STATE).is_dir());
    assert!(run(&machine, &["-t", "1"]).unwrap());
}

#[rstest]
fn a_failure_while_running_is_told_apart_from_one_at_the_start(machine: Machine) {
    // A directory can be opened as the log, but not read.
    machine.remove(KMSG);
    std::fs::create_dir(machine.join(KMSG)).unwrap();
    assert!(!execute(&cli(&machine, &[]), &AtomicBool::new(false)).unwrap());
    machine.remove(OFFSET);
    assert!(run(&machine, &[]).unwrap_err().is_missing());
}

#[rstest]
fn on_another_model_there_is_nothing_to_do_but_restore_works(machine: Machine) {
    machine.put(PRODUCT_NAME, "Some Other Machine\n");
    assert!(run(&machine, &[]).unwrap());
    assert!(!machine.join(STATE).exists(), "nothing was touched");

    assert!(run(&machine, &["restore"]).unwrap());
    let store = Store::new(&machine.join(STATE));
    store.prepare().unwrap();
    store.save(0, 7).unwrap();
    assert!(run(&machine, &["restore"]).unwrap());
    assert_eq!(machine.number(OFFSET), Some(7));
    assert_eq!(store.saved().unwrap(), []);
}

#[rstest]
fn without_a_name_or_a_log_the_guard_does_not_start(machine: Machine) {
    machine.remove(KMSG);
    assert!(run(&machine, &[]).unwrap_err().is_missing());
    assert!(!machine.join(STATE).exists(), "nothing was touched");
    machine.put(KMSG, "");
    machine.remove(PRODUCT_NAME);
    assert!(run(&machine, &[]).unwrap_err().is_missing());
    assert!(!machine.join(STATE).exists(), "nothing was touched");
}

#[test]
fn the_signals_that_stop_a_service_are_taken() {
    let stop = stop_on_signals().unwrap();

    assert!(!stop.load(Ordering::Relaxed));
}

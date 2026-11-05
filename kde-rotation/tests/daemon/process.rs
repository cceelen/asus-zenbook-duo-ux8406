//! Waiting for a process to end, as `restore` does after the program let go
//! of its name.

use std::process::Command;
use std::time::{Duration, Instant};

use asus_ux8406_kde_rotation::wait_ended;

#[test]
fn a_process_that_ends_is_waited_for() {
    let mut child = Command::new("sleep").arg("0.4").spawn().expect("sleep");
    let start = Instant::now();

    assert!(wait_ended(child.id(), Duration::from_secs(10)));
    assert!(start.elapsed() >= Duration::from_millis(300));
    child.wait().expect("end");
}

#[test]
fn a_process_that_runs_on_is_waited_for_no_longer_than_the_patience() {
    let mut child = Command::new("sleep").arg("30").spawn().expect("sleep");
    let start = Instant::now();

    assert!(!wait_ended(child.id(), Duration::from_millis(200)));
    assert!(start.elapsed() < Duration::from_secs(5));
    child.kill().expect("kill");
    child.wait().expect("end");
}

#[test]
fn a_process_that_is_gone_is_not_waited_for() {
    let mut child = Command::new("true").spawn().expect("true");
    let id = child.id();

    child.wait().expect("end");
    assert!(wait_ended(id, Duration::from_secs(10)));
}

#[test]
fn a_number_that_is_no_process_is_not_waited_for() {
    assert!(wait_ended(u32::MAX, Duration::from_secs(10)));
    assert!(wait_ended(0, Duration::from_secs(10)));
}

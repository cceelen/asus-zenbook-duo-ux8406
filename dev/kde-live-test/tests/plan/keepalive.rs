//! The renewal of the rights of sudo.

use std::time::{Duration, Instant};

use asus_ux8406_kde_live_test::keepalive::keep;

use crate::support::FakeRunner;

fn renewals(runner: &FakeRunner) -> usize {
    runner
        .calls()
        .iter()
        .filter(|call| *call == "sudo -n -v")
        .count()
}

#[test]
fn sudo_is_renewed_while_the_body_runs() {
    let runner = FakeRunner::new();
    let end = Instant::now() + Duration::from_secs(10);
    let answer = keep(&runner, Duration::from_millis(5), || {
        while renewals(&runner) < 2 {
            assert!(Instant::now() < end, "no renewal");
            std::thread::sleep(Duration::from_millis(5));
        }
        42
    });

    assert_eq!(answer, 42);
}

#[test]
fn the_renewal_stops_with_the_body() {
    let runner = FakeRunner::new();

    keep(&runner, Duration::from_millis(5), || {
        std::thread::sleep(Duration::from_millis(30));
    });

    let done = renewals(&runner);

    std::thread::sleep(Duration::from_millis(50));
    assert!(done > 0);
    assert_eq!(renewals(&runner), done);
}

#[test]
fn a_short_body_ends_before_the_first_renewal() {
    let runner = FakeRunner::new();

    keep(&runner, Duration::from_secs(60), || ());
    assert_eq!(renewals(&runner), 0);
}

//! A wait for the kernel log that runs out reads nothing: the fail point after
//! the wait is not reached. A program of its own, so that the fail point that
//! it switches on does not touch the tests that run at the same time
//! (`--features failpoints`).

#[cfg(feature = "failpoints")]
#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "failpoints")]
#[test]
fn a_wait_that_runs_out_reads_nothing() {
    use std::time::Duration;

    use asus_ux8406_tcc_guard::kmsg::Events;

    use crate::common::{KMSG, machine};

    let machine = machine();

    machine.fifo(KMSG);

    let mut events = Events::open(&machine.join(KMSG), "never in the log").unwrap();
    let scenario = fail::FailScenario::setup();

    // A read after the wait would meet the injected failure.
    fail::cfg("kmsg-poll", "return").unwrap();
    assert!(!events.wait(Duration::from_millis(50)).unwrap());
    scenario.teardown();
}

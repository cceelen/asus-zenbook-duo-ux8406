//! The rights of sudo: sudo forgets the password after 5 minutes, and the
//! run takes longer.

use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

use crate::runner::Runner;

/// Run `body` while a thread renews the rights of sudo (`sudo -n -v`) at
/// each `interval`. The thread ends when `body` ends, and no program stays
/// behind: each renewal runs to its end.
pub fn keep<R>(runner: &dyn Runner, interval: Duration, body: impl FnOnce() -> R) -> R {
    let (stop, stopped) = channel::<()>();

    std::thread::scope(|scope| {
        scope.spawn(move || {
            while stopped.recv_timeout(interval) == Err(RecvTimeoutError::Timeout) {
                let _ = runner.run("sudo", &["-n", "-v"]);
            }
        });

        let result = body();

        drop(stop);
        result
    })
}

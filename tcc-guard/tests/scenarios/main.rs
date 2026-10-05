//! The program on a machine that the test plays.
//!
//! Each test copies a machine from `tests/fixtures/` (a `/sys` with hwmon
//! devices and offset attributes, a kernel log, a configuration), starts the
//! program on it with `--root`, and plays the machine: the temperature of a
//! loaded package sits at the limit its offset allows, EC events arrive,
//! sensors fail, the program is stopped or killed. The program's log, its
//! exit status, the offset files and its notes are then checked.
//!
//! Steps of a test are tied to what the program has logged, not to the
//! clock, so that slow machines pass as well. One file per topic; the tests
//! of all of them run side by side.

#[path = "../common/mod.rs"]
mod common;
mod world;

mod configuration;
mod failures;
#[cfg(feature = "failpoints")]
mod faults;
mod holding;
mod kernel_log;
mod machines;
mod notes;
mod options;
mod stopping;

//! The parts of the guard, each on its own: one file per module of the
//! library. The program as a whole is in `tests/scenarios/`.

#[path = "../common/mod.rs"]
mod common;

mod cli;
mod config;
mod error;
mod guard;
mod input;
mod kmsg;
mod library;
mod machine;
mod run;
mod store;
mod sysfs;
mod trusted;

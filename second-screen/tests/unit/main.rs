//! The library of asus-ux8406-second-screen through its public interface, one module per
//! topic. All of them run in this one test program, in parallel threads.

#[path = "../common/mod.rs"]
mod common;

mod brightness;
mod cli;
mod dock;
mod error;
mod panel;
mod sysfs;

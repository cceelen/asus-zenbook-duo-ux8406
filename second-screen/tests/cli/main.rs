//! The program as udev runs it: its exit status and what it leaves in a
//! directory that stands in for `/sys`. The decisions themselves are covered
//! by the tests of the library. All topics run in this one test program, in
//! parallel threads.

#[path = "../common/mod.rs"]
mod common;

mod brightness;
mod dock;
mod dry_run;
mod refusal;
mod release;
mod usage;

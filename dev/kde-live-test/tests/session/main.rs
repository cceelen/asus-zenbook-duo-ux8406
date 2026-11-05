//! The tool against a fake KWin, a fake kglobalacceld and the other fakes on
//! a private bus: one module per topic. A thread of the test plays the hands
//! of the tester where a check waits for an action.

#[path = "../support/mod.rs"]
mod support;

mod above;
mod activate;
mod applet;
mod backlight;
mod booted;
mod collect;
mod dbus;
mod dead;
mod hand;
mod keyboard;
mod keys;
mod lower_key;
mod panels;
mod policy;
mod ready;
mod rotation;
mod run;
mod runtime;
mod shortcuts;
mod swap;
mod touch;

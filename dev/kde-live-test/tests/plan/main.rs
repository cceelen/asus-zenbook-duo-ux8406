//! The parts of the tool that need no compositor: one module per topic.

#[path = "../support/mod.rs"]
mod support;

mod booted;
mod cli;
mod console;
mod deps;
mod install;
mod keepalive;
mod live;
mod preflight;
mod report;
mod runner;
mod text;

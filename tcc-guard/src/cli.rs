//! The command line.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::input::{self, Number};

/// The longest time `--time` takes: a week.
const DURATION_MAX: i32 = 7 * 24 * 3600;

const AFTER_HELP: &str = "\
Configuration file (TOML), every setting optional:
  hold_temp, step_up, max_offset, interval, settle, probe_interval,
  probe_backoff, probe_max, tjmax, use_margin, idle_release (numbers),
  event_text (start of the kernel log message of the event),
  model (part of the product name of the machine; \"any\" for all),
  [[package]] tables with id and offset (the file below /sys where the
  offset of that processor package is written)";

/// CPU thermal-offset guard for the thermal warning of the ASUS embedded
/// controller.
///
/// The offset is left alone until the EC raises its thermal event; it is
/// then raised until the EC's alert has cleared, and lowered again step by
/// step for as long as no further event arrives.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
#[command(version, after_help = AFTER_HELP)]
pub struct Cli {
    /// Configuration file, in place of /etc/asus-ux8406-tcc-guard.toml
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// `hold_temp`: clear the alert by getting the CPU to this temperature
    #[arg(short = 'H', long, value_name = "DEGREES", value_parser = hold_temp)]
    pub hold_temp: Option<i32>,

    /// `probe_interval`: seconds between steps down
    #[arg(short = 'P', long, value_name = "SECONDS", value_parser = probe_interval)]
    pub probe_interval: Option<i32>,

    /// Stop after this many seconds; 0 runs until stopped
    #[arg(short = 't', long = "time", value_name = "SECONDS", default_value_t = 0,
          value_parser = duration)]
    pub duration: i32,

    /// Work on the system below this directory
    #[arg(long, value_name = "DIR", default_value = "/")]
    pub root: PathBuf,

    /// What to do; `run` if nothing is named
    #[command(subcommand)]
    pub action: Option<Action>,
}

/// What the guard is asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Subcommand)]
pub enum Action {
    /// Watch for the EC's thermal event and look after the offset
    Run,
    /// Put back the offset that a guard which was killed left raised
    Restore,
}

/// A whole number within a range, as strictly as the configuration file
/// takes it.
fn number(text: &str, (min, max): (i32, i32)) -> Result<i32, String> {
    input::parse_int(text, min, max)
        .ok_or_else(|| format!("not a whole number from {min} to {max}"))
}

fn hold_temp(text: &str) -> Result<i32, String> {
    number(text, Number::HoldTemp.range())
}

fn probe_interval(text: &str) -> Result<i32, String> {
    number(text, Number::ProbeInterval.range())
}

fn duration(text: &str) -> Result<i32, String> {
    number(text, (0, DURATION_MAX))
}

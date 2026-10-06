//! CPU thermal-offset guard for the thermal warning of the ASUS embedded
//! controller.
//!
//! The CPU thermal offset is left alone until the EC raises its thermal
//! event. The control logic in [`guard`] then raises the offset until the
//! EC's alert has cleared, and lowers it again step by step for as long as
//! no further event arrives, until the original offset is back. Nothing is
//! learned from one event for the next.
//!
//! Inputs are the EC's event, which the kernel log shows, and the package
//! temperatures as Linux reads them; nothing else. The guard needs root, and
//! so takes nothing from outside that [`input`] has not checked.
//!
//! A guard that is stopped (SIGTERM, SIGINT, SIGHUP) puts the offset back.
//! One that is killed cannot: the offset it found is therefore noted before
//! the first change ([`store`]), and the next start, or `asus-ux8406-tcc-guard restore`,
//! puts it back.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use log::info;
use sd_notify::NotifyState;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

pub mod cli;
pub mod config;
pub mod error;
pub mod fault;
pub mod guard;
pub mod input;
pub mod kmsg;
pub mod machine;
pub mod paths;
pub mod run;
pub mod store;
pub mod sysfs;
pub mod trusted;

use cli::{Action, Cli};
use error::Error;
use input::Settings;
use kmsg::Events;
use paths::Paths;
use run::Guard;
use store::Store;

/// The settings a command line asks for: the defaults, then the
/// configuration file, then the command line. The usual configuration file
/// may be missing; one that is named may not.
///
/// # Errors
///
/// What makes the file or the combination of the settings unacceptable.
pub fn settings(cli: &Cli, paths: &Paths) -> Result<Settings, Error> {
    let mut settings = Settings::default();

    match &cli.config {
        Some(path) => config::load(&mut settings, path)?,
        None => match config::load(&mut settings, &paths.config) {
            Err(error) if error.is_missing() => {}
            loaded => loaded?,
        },
    }
    if let Some(hold_temp) = cli.hold_temp {
        settings.guard.hold_temp = hold_temp;
    }
    if let Some(probe_interval) = cli.probe_interval {
        settings.guard.probe_interval = probe_interval;
    }
    if settings.valid() {
        Ok(settings)
    } else {
        Err(Error::Contradiction)
    }
}

/// A flag that the signals which stop a service set.
///
/// # Errors
///
/// [`Error::Signals`] if the system does not take the handlers.
pub fn stop_on_signals() -> Result<Arc<AtomicBool>, Error> {
    let stop = Arc::new(AtomicBool::new(false));

    for signal in [SIGTERM, SIGINT, SIGHUP] {
        fault::point(
            "signals",
            signal_hook::flag::register(signal, Arc::clone(&stop)),
        )
        .map_err(Error::Signals)?;
    }
    Ok(stop)
}

/// Tell the service manager, if there is one, how the guard is doing.
fn tell_manager(state: &[NotifyState]) {
    // Without a manager there is nobody to tell, and nothing depends on it.
    let _ = sd_notify::notify(state);
}

/// Do what a command line asks for, until it is done or `stop` is set;
/// false if the guard ran and stopped because of a failure.
///
/// On a machine of another model there is nothing to do, and that is no
/// failure: a service that is installed there ends, and is not started
/// again and again.
///
/// # Errors
///
/// What kept the guard from starting, or the offsets from being put back.
pub fn execute(cli: &Cli, stop: &AtomicBool) -> Result<bool, Error> {
    let paths = Paths::below(&cli.root);
    let settings = settings(cli, &paths)?;
    let store = Store::new(&paths.state);

    if cli.action == Some(Action::Restore) {
        if run::restore(&paths.sysfs, &settings, &store)? == 0 {
            info!("no offset to put back");
        }
        return Ok(true);
    }
    if let Err(error) = machine::check_model(&paths.sysfs, &settings) {
        if !matches!(error, Error::WrongModel { .. }) {
            return Err(error);
        }
        info!("{error}; nothing to do");
        tell_manager(&[NotifyState::Ready, NotifyState::Status("not this model")]);
        return Ok(true);
    }
    // Before anything is touched: without the log there is no guard.
    let mut events = Events::open(&paths.kmsg, &settings.event_text)?;
    let mut guard = Guard::arm(&paths.sysfs, &settings, store)?;

    guard.announce(&settings);
    tell_manager(&[NotifyState::Ready]);
    let ran = guard.watch(&mut events, cli.duration, stop);

    tell_manager(&[NotifyState::Stopping]);
    let restored = guard.finish();

    Ok(ran && restored)
}

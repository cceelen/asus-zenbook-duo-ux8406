//! The guard at work: the I/O around the control logic. Per package it reads
//! a temperature and the offset in force, and writes the offset the logic
//! asks for; the event comes from the kernel log and goes to all packages.
//!
//! Before an offset is changed for the first time, the offset found is noted
//! in the [`Store`]; the note goes when that offset is back. A guard that is
//! killed leaves the note behind, and [`restore`] acts on it.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use log::{error, info};

use crate::error::Error;
use crate::guard::{self, Phase, What};
use crate::input::Settings;
use crate::kmsg::Events;
use crate::machine::{self, Package};
use crate::store::Store;
use crate::sysfs;

/// How long one wait for the event lasts; the packages are looked at after
/// each.
const PATIENCE: Duration = Duration::from_secs(1);

/// Put back every offset that is noted as changed, and forget the notes; how
/// many there were.
///
/// # Errors
///
/// What stood in the way: notes that cannot be believed, a package without a
/// place for its offset, or a file that cannot be written. The note of an
/// offset that was not put back stays.
pub fn restore(sysfs: &Path, settings: &Settings, store: &Store) -> Result<usize, Error> {
    let saved = store.saved()?;
    let alone = saved.len() == 1 && settings.controls.is_empty();

    for &(package, base) in &saved {
        let path =
            machine::control(sysfs, settings, package, alone).ok_or(Error::NoControl(package))?;

        sysfs::write_offset(&path, base)?;
        store.clear(package)?;
        info!("package {package}: a guard was killed while it held an offset: back to {base}");
    }
    Ok(saved.len())
}

/// What a control step did, for the log.
const fn text(what: What) -> &'static str {
    match what {
        What::Raised => "above the hold temperature: offset raised to",
        What::AtMax => "above the hold temperature at the maximum offset",
        What::Cleared => "alert cleared: holding offset",
        What::Lowered => "no event: offset lowered to",
        What::Tripped => "event while holding: going back to offset",
        What::Reasserted => "offset was lowered by something else: back to",
        What::Released => "no event at the original offset: released at",
        What::IdleRelease => "offset not in use: released, back to",
    }
}

/// One package and what the guard remembers about it.
#[derive(Debug)]
struct Watched {
    package: Package,
    state: guard::State,
}

impl Watched {
    /// Read the inputs, run one control step and write what it asks for.
    fn step(
        &mut self,
        config: &guard::Config,
        store: &Store,
        event: bool,
        now: i32,
    ) -> Result<(), Error> {
        let id = self.package.id;
        let offset = sysfs::read_offset(&self.package.offset)?;
        let temp = sysfs::read_temp(&self.package.temp)?;
        let input = guard::Input {
            now,
            event,
            temp,
            offset,
            tjmax: self.package.tjmax,
        };

        if event {
            info!("package {id}: EC event at {temp} C, offset {offset}");
        }
        let before = self.state.phase;
        let output = self.state.step(config, &input);
        let after = self.state.phase;

        if before == Phase::Idle && after != Phase::Idle {
            store.save(id, self.state.base)?;
        }
        if output.write {
            sysfs::write_offset(&self.package.offset, output.offset)?;
        }
        if before != Phase::Idle && after == Phase::Idle {
            store.clear(id)?;
        }
        if let Some(what) = output.what {
            info!("package {id}: {temp} C, {} {}", text(what), output.offset);
            if what == What::Tripped {
                info!(
                    "package {id}: the next step down waits {} s",
                    self.state.wait
                );
            }
        }
        Ok(())
    }
}

/// The guard, armed: its packages are known and their files usable.
#[derive(Debug)]
pub struct Guard {
    config: guard::Config,
    store: Store,
    watched: Vec<Watched>,
}

impl Guard {
    /// Find the packages, check their files, and put back what a guard that
    /// was killed has left.
    ///
    /// # Errors
    ///
    /// What makes the machine unusable for the guard.
    pub fn arm(sysfs: &Path, settings: &Settings, store: Store) -> Result<Self, Error> {
        let packages = machine::packages(sysfs, settings)?;

        machine::check(&packages)?;
        store.prepare()?;
        restore(sysfs, settings, &store)?;
        Ok(Self {
            config: settings.guard,
            store,
            watched: packages
                .into_iter()
                .map(|package| Watched {
                    package,
                    state: guard::State::default(),
                })
                .collect(),
        })
    }

    /// Say what the guard looks after, and with which settings.
    pub fn announce(&self, settings: &Settings) {
        let config = &self.config;

        for Watched { package, .. } in &self.watched {
            if let (Ok(temp), Ok(offset)) = (
                sysfs::read_temp(&package.temp),
                sysfs::read_offset(&package.offset),
            ) {
                info!(
                    "package {}: armed: offset {offset}, CPU {temp} C, tjmax {}",
                    package.id, package.tjmax
                );
            }
        }
        info!(
            "hold_temp {}, step_up {}, max_offset {}, interval {}, settle {}",
            config.hold_temp, config.step_up, config.max_offset, config.interval, config.settle
        );
        info!(
            "probe_interval {}, probe_backoff {}, probe_max {}, use_margin {}, idle_release {}",
            config.probe_interval,
            config.probe_backoff,
            config.probe_max,
            config.use_margin,
            config.idle_release
        );
        info!(
            "event text: {:?}, model: {}",
            settings.event_text, settings.model
        );
    }

    /// The phase the guard is in with each package, and the offset it found
    /// there at the first event: (package, phase, offset found).
    #[must_use]
    pub fn phases(&self) -> Vec<(i32, Phase, i32)> {
        self.watched
            .iter()
            .map(|watched| (watched.package.id, watched.state.phase, watched.state.base))
            .collect()
    }

    /// One control step for every package; false if the guard has to stop
    /// because one of them failed.
    pub fn step(&mut self, event: bool, now: i32) -> bool {
        for watched in &mut self.watched {
            if let Err(failure) = watched.step(&self.config, &self.store, event, now) {
                error!(
                    "package {}: stopping: cannot read or write: {failure}",
                    watched.package.id
                );
                return false;
            }
        }
        true
    }

    /// Watch the event and the packages until `stop` is set, or for
    /// `duration` seconds if that is not 0; false if the guard stops because
    /// of a failure.
    pub fn watch(&mut self, events: &mut Events, duration: i32, stop: &AtomicBool) -> bool {
        let start = Instant::now();

        while !stop.load(Ordering::Relaxed) {
            let now = i32::try_from(start.elapsed().as_secs()).unwrap_or(i32::MAX);

            if duration > 0 && now >= duration {
                break;
            }
            match events.wait(PATIENCE) {
                Ok(event) => {
                    if !self.step(event, now) {
                        return false;
                    }
                }
                Err(failure) => {
                    error!("stopping: {failure}");
                    return false;
                }
            }
        }
        true
    }

    /// Put the offset back wherever the guard is still holding one; false if
    /// that failed for a package. Its note then stays.
    pub fn finish(&mut self) -> bool {
        let mut restored = true;

        for watched in &mut self.watched {
            let id = watched.package.id;
            let base = watched.state.base;

            if watched.state.phase == Phase::Idle {
                info!("package {id}: stopping: offset untouched");
                continue;
            }
            match sysfs::write_offset(&watched.package.offset, base)
                .and_then(|()| self.store.clear(id))
            {
                Ok(()) => {
                    watched.state = guard::State::default();
                    info!("package {id}: stopping: offset back to {base}");
                }
                Err(failure) => {
                    restored = false;
                    error!(
                        "package {id}: stopping: could not put the offset back to {base}: {failure}"
                    );
                }
            }
        }
        restored
    }
}

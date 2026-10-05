//! The control logic of asus-ux8406-tcc-guard, free of any I/O so that it can be tested
//! with plain input sequences.
//!
//! Inputs per step: the time, whether the EC's thermal event arrived, the CPU
//! temperature and the thermal offset currently in force. Output: the offset
//! to write, if any. Nothing else is an input; in particular not the lid.
//!
//! One [`State`] belongs to one processor package. The event does not say
//! which package caused it, so it is given to every state.
//!
//! - **Idle**: nothing is written until an event arrives.
//! - **Clearing**: the offset is raised until the CPU is at or below
//!   `hold_temp` and has stayed there, so that the EC's alert switches off.
//! - **Holding**: the offset is held and challenged. After a wait without an
//!   event it is lowered by one; a step that survives `probe_interval`, which
//!   must be longer than the EC's delay before it raises the event, is
//!   followed by the next one. An event means the value in force is too low:
//!   one more is needed, and the wait before the next challenge grows.
//!
//! All waits count only the time the offset is in use, that is while the CPU
//! is within `use_margin` of the limit the offset allows. A pause in the load
//! proves nothing about the value in force, so it neither counts towards a
//! step down nor makes the guard give the offset up.
//!
//! Two ways lead back to Idle, and both forget everything learned:
//! - the steps down reach the offset found at the first event, and that value
//!   survives its `probe_interval` like any other step;
//! - the offset has been out of use for `idle_release` seconds on end.

/// Where the guard is with one package.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Nothing is written until an event arrives.
    #[default]
    Idle,
    /// The offset is raised until the EC's alert has switched off.
    Clearing,
    /// The offset is held, and lowered step by step.
    Holding,
}

/// What a step did, for the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    /// Clearing: one step up.
    Raised,
    /// Clearing: still too hot at the maximum offset.
    AtMax,
    /// Clearing done; the offset is now held.
    Cleared,
    /// Holding: no event for the wait, one step down.
    Lowered,
    /// Event while holding: one more is needed.
    Tripped,
    /// Something else lowered the offset; written again.
    Reasserted,
    /// The original offset survived; everything forgotten.
    Released,
    /// Out of use for long; original offset, forgotten.
    IdleRelease,
}

/// The settings of the control logic. Times are in seconds, temperatures and
/// offsets in degrees C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Clearing ends at or below this temperature.
    pub hold_temp: i32,
    /// Degrees added per clearing step.
    pub step_up: i32,
    /// The offset is never raised beyond this.
    pub max_offset: i32,
    /// Seconds between control steps.
    pub interval: i32,
    /// Seconds at or below `hold_temp` before clearing ends.
    pub settle: i32,
    /// Seconds without an event before one step down.
    pub probe_interval: i32,
    /// The wait is multiplied by this after each trip.
    pub probe_backoff: i32,
    /// The wait never grows beyond this.
    pub probe_max: i32,
    /// Within this of its limit the offset is in use.
    pub use_margin: i32,
    /// Seconds out of use before the offset is given up.
    pub idle_release: i32,
}

/// What the guard remembers about one package.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// Where the guard is.
    pub phase: Phase,
    /// Offset found at the first event; the release point.
    pub base: i32,
    /// Offset the guard last asked for.
    pub want: i32,
    /// Offset to hold after clearing; 0 = where clearing ends.
    pub floor: i32,
    /// Seconds at or below `hold_temp` while clearing.
    below: i32,
    /// Seconds on end that the offset has been out of use.
    unused: i32,
    /// Seconds in use since the last step down or clearing.
    used: i32,
    /// Seconds in use after which the next step down comes.
    due: i32,
    /// Seconds in use before a held offset is challenged.
    pub wait: i32,
    /// Time of the next control step.
    next_step: i32,
    /// The last step down has not yet survived.
    trial: bool,
}

/// What the guard is given at one moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    /// Seconds, monotonic.
    pub now: i32,
    /// The EC's event arrived since the last step.
    pub event: bool,
    /// Temperature of this package.
    pub temp: i32,
    /// Thermal offset currently in force for this package.
    pub offset: i32,
    /// Temperature this package is limited to at offset 0.
    pub tjmax: i32,
}

/// What one step asks for and reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// The offset is to be written.
    pub write: bool,
    /// The offset that the step is about.
    pub offset: i32,
    /// What happened, if anything.
    pub what: Option<What>,
}

impl Output {
    /// Something to report about an offset that is not written.
    const fn report(offset: i32, what: What) -> Self {
        Self {
            write: false,
            offset,
            what: Some(what),
        }
    }
}

impl State {
    /// One moment of the guard's life: take the inputs, say what to write.
    ///
    /// An event is acted on at once; otherwise a control step happens every
    /// `interval` seconds.
    #[must_use]
    pub fn step(&mut self, config: &Config, input: &Input) -> Output {
        if input.event && self.on_event(config, input) {
            return Output::report(self.floor, What::Tripped);
        }
        if self.phase == Phase::Idle || input.now < self.next_step {
            return Output::default();
        }
        self.next_step = input.now + config.interval;
        if self.phase == Phase::Clearing {
            self.clearing(config, input)
        } else {
            self.holding(config, input)
        }
    }

    fn write(&mut self, offset: i32, what: What) -> Output {
        self.want = offset;
        Output {
            write: true,
            offset,
            what: Some(what),
        }
    }

    /// The wait before the next challenge, after a trip.
    const fn longer_wait(&self, config: &Config) -> i32 {
        if config.probe_backoff < 1 || self.wait > config.probe_max / config.probe_backoff {
            return config.probe_max;
        }
        self.wait * config.probe_backoff
    }

    /// An event arrived. True if it tripped the offset that was being held.
    fn on_event(&mut self, config: &Config, input: &Input) -> bool {
        let before = self.phase;

        if before == Phase::Idle {
            self.base = input.offset;
            self.want = input.offset;
            self.wait = config.probe_interval;
        }
        if before == Phase::Holding {
            self.floor = (self.want + 1).min(config.max_offset);
            self.wait = self.longer_wait(config);
        }
        self.phase = Phase::Clearing;
        self.below = 0;
        self.unused = 0;
        self.used = 0;
        self.trial = false;
        self.next_step = input.now;
        before == Phase::Holding
    }

    fn clearing(&mut self, config: &Config, input: &Input) -> Output {
        if input.temp > config.hold_temp {
            self.below = 0;
            if input.offset >= config.max_offset {
                return Output::report(input.offset, What::AtMax);
            }
            let raised = (input.offset + config.step_up).min(config.max_offset);
            return self.write(raised, What::Raised);
        }
        self.below += config.interval;
        if self.below < config.settle {
            return Output::default();
        }
        self.phase = Phase::Holding;
        self.due = self.wait;
        if self.floor > 0 && self.floor != input.offset {
            return self.write(self.floor, What::Cleared);
        }
        self.want = input.offset;
        Output::report(input.offset, What::Cleared)
    }

    /// True while the CPU is close enough to the limit that the offset allows
    /// for the offset to be what holds its temperature down.
    const fn in_use(&self, config: &Config, input: &Input) -> bool {
        input.temp > input.tjmax - self.want - config.use_margin
    }

    /// Back to the original offset, with nothing kept.
    fn release(&mut self, what: What) -> Output {
        let base = self.base;

        *self = Self::default();
        Output {
            write: true,
            offset: base,
            what: Some(what),
        }
    }

    /// The offset in force has been in use for long enough without an event:
    /// one step down, or the release if it is the original offset already.
    fn challenge(&mut self, config: &Config) -> Output {
        if self.trial {
            // The last step survived: carry on.
            self.wait = config.probe_interval;
        }
        if self.want <= self.base {
            return self.release(What::Released);
        }
        self.trial = true;
        self.used = 0;
        self.due = config.probe_interval;
        self.write(self.want - 1, What::Lowered)
    }

    /// Keep the wanted offset in force, and lower it when that is due.
    fn holding(&mut self, config: &Config, input: &Input) -> Output {
        if input.offset < self.want {
            return self.write(self.want, What::Reasserted);
        }
        self.want = input.offset;
        if !self.in_use(config, input) {
            self.unused += config.interval;
            if self.unused < config.idle_release {
                return Output::default();
            }
            return self.release(What::IdleRelease);
        }
        self.unused = 0;
        self.used += config.interval;
        if self.used < self.due {
            return Output::default();
        }
        self.challenge(config)
    }
}

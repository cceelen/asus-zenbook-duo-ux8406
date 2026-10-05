//! Input traces for the control logic. Each trace covers one family of
//! transitions and hits every threshold from both sides: `hold_temp`, the
//! margin within which the offset counts as in use, the idle time after
//! which it is given up, the wait before a step down and its growth, and
//! the maximum offset.

use asus_ux8406_tcc_guard::guard::*;

/// Short times so that a trace stays readable; one step per second.
const CONFIG: Config = Config {
    hold_temp: 87,
    step_up: 2,
    max_offset: 30,
    interval: 1,
    settle: 3,
    probe_interval: 10,
    probe_backoff: 2,
    probe_max: 35,
    use_margin: 5,
    idle_release: 6,
};

/// The pace of the defaults: a control step every four seconds, and the
/// other times in multiples of that.
const PACED: Config = Config {
    interval: 4,
    settle: 8,
    probe_interval: 12,
    idle_release: 8,
    ..CONFIG
};

/// The machine as the guard sees it: its state, the clock and the offset.
struct Rig {
    config: Config,
    state: State,
    now: i32,
    offset: i32,
    writes: u32,
}

impl Rig {
    fn new(config: Config) -> Self {
        Self {
            config,
            state: State::default(),
            now: 0,
            offset: 10,
            writes: 0,
        }
    }

    /// Advance one second with the given inputs and apply what the guard
    /// writes.
    fn tick(&mut self, event: bool, temp: i32) -> Option<What> {
        let input = Input {
            now: self.now,
            event,
            temp,
            offset: self.offset,
            tjmax: 105,
        };
        let out = self.state.step(&self.config, &input);

        if out.write {
            self.offset = out.offset;
            self.writes += 1;
        }
        self.now += 1;
        out.what
    }

    /// Let the given number of seconds pass at a temperature, without an
    /// event.
    fn run(&mut self, seconds: u32, temp: i32) {
        for _ in 0..seconds {
            let _ = self.tick(false, temp);
        }
    }

    /// Clear to offset 20 and hold it, with the CPU at its limit
    /// throughout.
    fn clear_to_20(&mut self) {
        let _ = self.tick(true, 95);
        self.run(4, 95); // 10 -> 20
        assert_eq!(self.offset, 20);
        self.run(3, 85);
        assert_eq!(self.state.phase, Phase::Holding);
    }

    /// An event at 90 C while holding, then clearing again.
    fn trip_and_clear(&mut self) {
        assert_eq!(self.tick(true, 90), Some(What::Tripped));
        self.run(1, 90);
        self.run(3, 85);
        assert_eq!(self.state.phase, Phase::Holding);
    }
}

/// Event and clearing; then the load ends. The offset is held through
/// short pauses and given up only after it has been out of use for
/// `idle_release` seconds on end. With offset 14 the CPU is limited to
/// 91 C, so the offset is in use above 86 C.
#[test]
fn clear_and_idle_release() {
    let mut rig = Rig::new(CONFIG);

    rig.run(5, 95);
    assert_eq!(rig.writes, 0, "writes while idle");
    let _ = rig.tick(true, 95); // event: first step in the same second
    assert_eq!(rig.offset, 12);
    rig.run(1, 88); // one above the hold temperature: still a step
    assert_eq!(rig.offset, 14);
    rig.run(2, 87); // at the hold temperature: no step, not yet settled
    assert_eq!(rig.offset, 14);
    assert_eq!(rig.state.phase, Phase::Clearing);
    rig.run(1, 87);
    assert_eq!(rig.state.phase, Phase::Holding);
    rig.run(5, 86); // out of use, one second short of idle_release
    rig.run(1, 87); // one degree warmer is in use: the count starts again
    rig.run(5, 86);
    assert_eq!(rig.offset, 14);
    assert_eq!(rig.tick(false, 86), Some(What::IdleRelease));
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Idle);
    let writes = rig.writes;
    rig.run(5, 95);
    assert_eq!(rig.writes, writes, "writes after release");
}

/// Steps down while the CPU is at its limit: each one has to survive the
/// probe interval, a trip puts the last good value back, and every trip
/// lengthens the wait before that value is challenged again, up to the
/// longest wait.
#[test]
fn probe_and_trip() {
    let mut rig = Rig::new(CONFIG);

    rig.clear_to_20();
    rig.run(9, 85); // one second short of the probe interval
    assert_eq!(rig.offset, 20);
    rig.run(1, 85);
    assert_eq!(rig.offset, 19);
    rig.run(10, 86); // 19 survived: on to 18
    assert_eq!(rig.offset, 18);
    rig.run(5, 86); // five seconds into the trial of 18
    rig.trip_and_clear(); // 18 is too low; those five seconds count for nothing
    assert_eq!(rig.offset, 19);
    assert_eq!(rig.state.wait, 20);
    rig.run(19, 85); // one second short of the doubled wait
    assert_eq!(rig.offset, 19);
    rig.run(1, 85);
    assert_eq!(rig.offset, 18);
    rig.trip_and_clear(); // still too low: 40 s would exceed the longest wait
    assert_eq!(rig.offset, 19);
    assert_eq!(rig.state.wait, 35);
    rig.run(34, 85);
    assert_eq!(rig.offset, 19);
    rig.run(1, 85);
    assert_eq!(rig.offset, 18);
}

/// A pause in the load: the offset stays, and the pause does not count
/// towards the next step down. This is the run of 2026-10-02, where the
/// offset was given up during a pause and the returning load brought the
/// event back.
#[test]
fn pause() {
    let mut rig = Rig::new(CONFIG);

    rig.clear_to_20();
    rig.run(5, 85); // five seconds in use
    rig.run(5, 70); // a pause, one second short of idle_release
    assert_eq!(rig.offset, 20);
    rig.run(4, 85); // nine seconds in use in all
    assert_eq!(rig.offset, 20);
    rig.run(1, 85); // ten: the step down, as if there had been no pause
    assert_eq!(rig.offset, 19);
    rig.run(5, 70); // the trial of 19 is not judged while out of use
    rig.run(9, 86);
    assert_eq!(rig.offset, 19);
    rig.run(1, 86);
    assert_eq!(rig.offset, 18);
    rig.run(5, 70); // a long pause gives the offset up in one go
    assert_eq!(rig.offset, 18);
    rig.run(1, 70);
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Idle);
}

/// Conditions change (the lid opens): the CPU stays at its limit, but no
/// event comes. The steps down follow each other at the probe interval,
/// and the original offset has to survive like any other step before it
/// counts as the release.
#[test]
fn walk_to_release() {
    let mut rig = Rig::new(CONFIG);

    rig.clear_to_20();
    rig.run(20, 85); // 20 -> 18
    rig.trip_and_clear();
    rig.run(20, 85); // held at 19 for the doubled wait, then 18 again
    assert_eq!(rig.offset, 18);
    rig.run(9, 95);
    assert_eq!(rig.offset, 18);
    rig.run(1, 95); // 18 survived this time: the wait is short again
    assert_eq!(rig.offset, 17);
    assert_eq!(rig.state.wait, 10);
    rig.run(70, 95); // 16, 15, ... down to the original 10
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Holding); // written, but not yet trusted
    rig.run(9, 95);
    assert_eq!(rig.state.phase, Phase::Holding);
    assert_eq!(rig.tick(false, 95), Some(What::Released));
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Idle);
    assert_eq!(rig.state.floor, 0);
}

/// The original offset does not survive: the event comes while it is on
/// trial. That is a trip like any other, not a new start: one more is put
/// back and the wait grows.
#[test]
fn trip_at_the_original_offset() {
    let mut rig = Rig::new(CONFIG);

    rig.clear_to_20();
    rig.run(100, 95); // ten steps down, to 10
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Holding);
    assert_eq!(rig.tick(true, 95), Some(What::Tripped));
    assert_eq!(rig.state.floor, 11);
    assert_eq!(rig.state.wait, 20);
    assert_eq!(rig.state.base, 10);
    rig.run(1, 95); // clearing: 10 -> 12
    rig.run(3, 87);
    assert_eq!(rig.state.phase, Phase::Holding);
    assert_eq!(rig.offset, 11);
    rig.run(19, 95); // held for the doubled wait before the next try
    assert_eq!(rig.offset, 11);
    rig.run(1, 95);
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Holding);
}

/// An event while an offset is simply held asks for one more. Nothing of
/// that is left after the release: the next event starts from scratch.
#[test]
fn forgotten() {
    let mut rig = Rig::new(CONFIG);

    rig.clear_to_20();
    rig.trip_and_clear(); // no step down had been made: 20 itself was too low
    assert_eq!(rig.offset, 21);
    assert_eq!(rig.state.floor, 21);
    assert_eq!(rig.state.wait, 20);
    rig.run(6, 70); // out of use for idle_release: given up
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Idle);
    assert_eq!(rig.state.floor, 0);
    assert_eq!(rig.state.wait, 0);
    rig.clear_to_20(); // held where clearing ended, not at 21
    rig.run(9, 85);
    assert_eq!(rig.offset, 20);
    rig.run(1, 85); // and challenged after the short wait
    assert_eq!(rig.offset, 19);
}

/// The CPU never cools: the offset stops at the maximum.
#[test]
fn saturation() {
    let mut rig = Rig::new(CONFIG);

    let _ = rig.tick(true, 95);
    rig.run(8, 95); // 12 after the event, then 8 more steps of 2: 28
    assert_eq!(rig.offset, 28);
    rig.run(1, 95);
    assert_eq!(rig.offset, 30);
    let writes = rig.writes;
    rig.run(5, 95);
    assert_eq!(rig.offset, 30);
    assert_eq!(rig.writes, writes, "writes at the maximum");
    // A repeated event must not move the offset to go back to.
    let _ = rig.tick(true, 95);
    assert_eq!(rig.state.base, 10);
    // An event while the maximum is held cannot ask for more than that.
    rig.run(3, 70);
    assert_eq!(rig.state.phase, Phase::Holding);
    let _ = rig.tick(true, 70);
    assert_eq!(rig.state.floor, 30);
}

/// Something else changes the offset while the guard holds one.
#[test]
fn interference() {
    let mut rig = Rig::new(CONFIG);

    rig.clear_to_20();
    rig.offset = 10; // lowered behind the guard's back: written again
    rig.run(1, 86);
    assert_eq!(rig.offset, 20);
    rig.offset = 25; // raised behind its back: left alone
    rig.run(9, 86);
    assert_eq!(rig.offset, 25);
    rig.run(1, 86); // ten seconds in use: the step down, from 25
    assert_eq!(rig.offset, 24);
    rig.run(5, 70); // the release still ends at the original offset
    assert_eq!(rig.offset, 24);
    rig.run(1, 70);
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Idle);
}

/// A control interval longer than a second: steps wait for it, an event
/// does not, and the timers count in its units.
#[test]
fn pacing() {
    let mut rig = Rig::new(PACED);

    let _ = rig.tick(true, 95); // t=0: the event is acted on at once
    assert_eq!(rig.offset, 12);
    rig.run(3, 95); // t=1..3: not yet time for the next step
    assert_eq!(rig.offset, 12);
    rig.run(1, 95); // t=4
    assert_eq!(rig.offset, 14);
    rig.run(4, 95); // t=8
    assert_eq!(rig.offset, 16);
    rig.run(7, 85); // t=9..15: one step at or below hold_temp, need two
    assert_eq!(rig.state.phase, Phase::Clearing);
    rig.run(1, 85); // t=16
    assert_eq!(rig.state.phase, Phase::Holding);

    rig.run(11, 85); // t=17..27: two control steps in use, need three
    assert_eq!(rig.offset, 16);
    rig.run(1, 85); // t=28
    assert_eq!(rig.offset, 15);
    rig.run(1, 85); // t=29
    assert_eq!(rig.tick(true, 90), Some(What::Tripped)); // t=30, between steps
    rig.run(1, 90); // t=31: clearing starts at once, not at t=32
    assert_eq!(rig.offset, 17);
    rig.run(3, 90); // t=32..34: and then keeps the interval
    assert_eq!(rig.offset, 17);
    rig.run(1, 90); // t=35
    assert_eq!(rig.offset, 19);
    rig.run(4, 70); // t=36..39: step at t=39, settled after two
    rig.run(4, 70); // t=43: holding at 16, the value before the trip
    assert_eq!(rig.offset, 16);
    rig.run(4, 70); // t=47: out of use for one control step, need two
    assert_eq!(rig.offset, 16);
    rig.run(3, 70); // t=48..50: no control step yet
    assert_eq!(rig.offset, 16);
    rig.run(1, 70); // t=51: out of use for 8 s, given up
    assert_eq!(rig.offset, 10);
    assert_eq!(rig.state.phase, Phase::Idle);
}

/// Clearing ends with the offset already where it is to be held: nothing
/// is written. And a backoff of 0, which the configuration check refuses,
/// must still not divide by zero.
#[test]
fn corners() {
    let mut rig = Rig::new(Config {
        probe_backoff: 0,
        ..CONFIG
    });

    rig.clear_to_20();
    let _ = rig.tick(true, 90); // 20 was too low: 21 is wanted
    assert_eq!(rig.state.floor, 21);
    assert_eq!(rig.state.wait, 35);
    rig.offset = 21; // and something else has set it already
    let writes = rig.writes;
    rig.run(3, 85);
    assert_eq!(rig.state.phase, Phase::Holding);
    assert_eq!(rig.offset, 21);
    assert_eq!(rig.writes, writes);
    assert_eq!(rig.state.want, 21);
}

//! The guard at work on a machine, second by second. What it says about it
//! is checked where the program runs as a whole (`tests/scenarios/`).

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use asus_ux8406_tcc_guard::config;
use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::guard::Phase;
use asus_ux8406_tcc_guard::input::Settings;
use asus_ux8406_tcc_guard::kmsg::Events;
use asus_ux8406_tcc_guard::run::{Guard, restore};
use asus_ux8406_tcc_guard::store::Store;
use rstest::{fixture, rstest};

use crate::common::{
    CONFIG, KMSG, Machine, OFFSET, OFFSET_TWO, STATE, TEMP, machine, ordinary_user,
};

/// The settings of the machine's configuration file: short times.
fn settings(machine: &Machine) -> Settings {
    let mut settings = Settings::default();

    config::load(&mut settings, &machine.join(CONFIG)).unwrap();
    settings
}

fn store(machine: &Machine) -> Store {
    Store::new(&machine.join(STATE))
}

fn arm(machine: &Machine, settings: &Settings) -> Guard {
    Guard::arm(&machine.sys(), settings, store(machine)).unwrap()
}

/// A machine with a guard on it.
struct Bench {
    machine: Machine,
    guard: Guard,
    loaded: bool,
    now: i32,
}

#[fixture]
fn bench(machine: Machine) -> Bench {
    Bench::with(machine, None)
}

impl Bench {
    fn with(machine: Machine, changed: Option<Settings>) -> Self {
        let guard = arm(&machine, &changed.unwrap_or_else(|| settings(&machine)));

        Self {
            machine,
            guard,
            loaded: true,
            now: 0,
        }
    }

    fn offset(&self) -> i32 {
        self.machine.number(OFFSET).unwrap()
    }

    /// One second: the processor runs at the limit its offset allows, or at
    /// 60 C without load; then the guard looks.
    fn second(&mut self, event: bool) -> bool {
        let temp = if self.loaded { 105 - self.offset() } else { 60 };

        self.machine.put(TEMP, format!("{}\n", temp * 1000));
        self.now += 1;
        self.guard.step(event, self.now)
    }

    fn seconds(&mut self, count: usize) {
        for _ in 0..count {
            assert!(self.second(false));
        }
    }

    /// The offsets the guard has noted as changed.
    fn noted(&self) -> Vec<(i32, i32)> {
        store(&self.machine).saved().unwrap()
    }

    fn phase(&self) -> Phase {
        self.guard.phases()[0].1
    }
}

#[rstest]
fn nothing_is_written_or_noted_without_an_event(mut bench: Bench) {
    bench.seconds(5);
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
    assert_eq!(bench.guard.phases(), [(0, Phase::Idle, 0)]);
    assert!(bench.guard.finish());
    assert_eq!(bench.offset(), 10);
}

#[rstest]
fn an_event_under_load_raises_holds_and_steps_down(mut bench: Bench) {
    assert!(bench.second(true));
    assert_eq!(bench.offset(), 12);
    assert_eq!(bench.noted(), [(0, 10)], "noted with the first change");
    assert_eq!(bench.guard.phases(), [(0, Phase::Clearing, 10)]);
    // 93, 91, 89 C: three more steps up; 87 C for two seconds: held.
    bench.seconds(5);
    assert_eq!(bench.offset(), 18);
    assert_eq!(bench.phase(), Phase::Holding);
    // In use for three seconds without an event: one step down.
    bench.seconds(3);
    assert_eq!(bench.offset(), 17);
    assert_eq!(bench.noted(), [(0, 10)]);

    assert!(bench.guard.finish());
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
    assert_eq!(bench.phase(), Phase::Idle);
    assert!(bench.guard.finish(), "a second time");
    assert_eq!(bench.offset(), 10);
}

#[rstest]
fn an_event_while_holding_asks_for_one_more(mut bench: Bench) {
    assert!(bench.second(true));
    bench.seconds(5);
    assert!(bench.second(true));
    assert_eq!(bench.phase(), Phase::Clearing);
    bench.seconds(3);
    assert_eq!(bench.offset(), 19);
    assert_eq!(
        bench.noted(),
        [(0, 10)],
        "the offset found at the first event"
    );
}

#[rstest]
fn an_offset_out_of_use_is_given_up_and_its_note_with_it(mut bench: Bench) {
    assert!(bench.second(true));
    bench.loaded = false;
    bench.seconds(4);
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
    assert_eq!(bench.phase(), Phase::Idle);
}

#[rstest]
fn steps_down_end_at_the_offset_found_and_release_it(mut bench: Bench) {
    assert!(bench.second(true));
    bench.seconds(5);
    assert_eq!(bench.offset(), 18);
    // Eight steps down, three seconds each, under load all the time.
    bench.seconds(24);
    assert_eq!(bench.offset(), 10);
    assert_eq!(
        bench.noted(),
        [(0, 10)],
        "not released before it has survived"
    );
    assert_eq!(bench.phase(), Phase::Holding);
    bench.seconds(3);
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
    assert_eq!(bench.phase(), Phase::Idle);
}

#[rstest]
fn the_offset_is_not_raised_beyond_the_maximum(machine: Machine) {
    let mut limited = settings(&machine);

    limited.guard.max_offset = 13;
    let mut bench = Bench::with(machine, Some(limited));

    assert!(bench.second(true));
    bench.seconds(5);
    assert_eq!(bench.offset(), 13);
    assert_eq!(bench.phase(), Phase::Clearing, "still too hot");
}

#[rstest]
fn an_offset_lowered_by_something_else_is_written_again(mut bench: Bench) {
    assert!(bench.second(true));
    bench.seconds(5);
    bench.machine.put(OFFSET, "10\n");
    bench.seconds(1);
    assert_eq!(bench.offset(), 18);
}

#[rstest]
fn a_sensor_that_fails_stops_the_guard_and_the_offset_goes_back(mut bench: Bench) {
    assert!(bench.second(true));
    bench.machine.put(TEMP, "rubbish\n");
    assert!(!bench.guard.step(false, 2));
    assert!(bench.guard.finish());
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
}

#[rstest]
fn an_offset_that_is_gone_stops_the_guard(mut bench: Bench) {
    bench.machine.remove(OFFSET);
    assert!(!bench.guard.step(false, 1));
    assert_eq!(bench.phase(), Phase::Idle);
    assert!(bench.guard.finish(), "nothing to put back");
}

#[rstest]
fn an_offset_that_cannot_be_put_back_keeps_its_note(mut bench: Bench) {
    ordinary_user();
    bench.machine.chmod(OFFSET, 0o400);
    assert!(!bench.second(true));
    assert_eq!(bench.offset(), 10, "not raised");
    assert!(!bench.guard.finish());
    assert_eq!(bench.noted(), [(0, 10)]);

    // Once it can be written, the next start puts it back.
    bench.machine.chmod(OFFSET, 0o600);
    bench.machine.put(OFFSET, "14\n");
    arm(&bench.machine, &settings(&bench.machine));
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
}

#[rstest]
fn an_offset_is_not_changed_if_it_cannot_be_noted_first(mut bench: Bench) {
    ordinary_user();
    bench.machine.chmod(STATE, 0o500);
    assert!(!bench.second(true));
    assert_eq!(bench.offset(), 10);
    bench.machine.chmod(STATE, 0o700);
    assert!(bench.guard.finish());
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
}

#[rstest]
fn a_note_that_cannot_be_removed_stops_the_guard_with_the_offset_back(mut bench: Bench) {
    let (sys, store) = (bench.machine.sys(), store(&bench.machine));
    let settings = settings(&bench.machine);

    ordinary_user();
    assert!(bench.second(true));
    assert_eq!(bench.offset(), 12);
    bench.loaded = false;
    bench.machine.chmod(STATE, 0o500);
    // Cleared after two seconds, given up after two more.
    bench.seconds(3);
    assert!(!bench.second(false));
    assert_eq!(
        bench.offset(),
        10,
        "the offset goes back before the note goes"
    );
    assert_eq!(bench.noted(), [(0, 10)]);

    // The same for restore: the offset is written, the note stays.
    bench.machine.put(OFFSET, "15\n");
    assert!(matches!(
        restore(&sys, &settings, &store),
        Err(Error::Io { .. })
    ));
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), [(0, 10)]);
    bench.machine.chmod(STATE, 0o700);
    assert_eq!(restore(&sys, &settings, &store).unwrap(), 1);
    assert_eq!(bench.noted(), []);
}

#[rstest]
fn restore_puts_back_what_a_killed_guard_left(mut bench: Bench) {
    let (sys, store) = (bench.machine.sys(), store(&bench.machine));
    let settings = settings(&bench.machine);

    assert!(bench.second(true));
    bench.seconds(5);
    assert_eq!(bench.offset(), 18);
    // The guard is gone without a word: only its note is left.
    assert_eq!(restore(&sys, &settings, &store).unwrap(), 1);
    assert_eq!(bench.offset(), 10);
    assert_eq!(bench.noted(), []);
    assert_eq!(restore(&sys, &settings, &store).unwrap(), 0);
    assert_eq!(bench.offset(), 10);
}

#[rstest]
fn restore_does_not_guess_between_packages(machine: Machine) {
    let store = store(&machine);
    let mut configured = settings(&machine);

    store.prepare().unwrap();
    store.save(0, 5).unwrap();
    store.save(1, 6).unwrap();
    assert!(matches!(
        restore(&machine.sys(), &settings(&machine), &store),
        Err(Error::NoControl(0))
    ));
    assert_eq!(machine.number(OFFSET), Some(10));
    assert_eq!(store.saved().unwrap().len(), 2, "the notes stay");

    machine.put(OFFSET_TWO, "20\n");
    configured.add_control(0, &format!("/{OFFSET}")).unwrap();
    configured
        .add_control(1, &format!("/{OFFSET_TWO}"))
        .unwrap();
    assert_eq!(restore(&machine.sys(), &configured, &store).unwrap(), 2);
    assert_eq!(machine.number(OFFSET), Some(5));
    assert_eq!(machine.number(OFFSET_TWO), Some(6));
    assert_eq!(store.saved().unwrap(), []);
}

#[rstest]
fn a_package_that_cannot_be_read_does_not_keep_the_guard_from_saying_its_settings(bench: Bench) {
    bench.guard.announce(&settings(&bench.machine));
    bench.machine.remove(TEMP);
    bench.guard.announce(&settings(&bench.machine));
}

#[rstest]
fn watching_ends_when_the_time_is_up(mut bench: Bench) {
    let mut events = Events::open(&bench.machine.join(KMSG), "test: thermal warning").unwrap();
    let start = Instant::now();

    assert!(bench.guard.watch(&mut events, 1, &AtomicBool::new(false)));
    assert!(start.elapsed() >= Duration::from_secs(1));
    assert_eq!(bench.phase(), Phase::Idle);
}

#[rstest]
fn watching_ends_at_once_when_the_guard_is_told_to_stop(mut bench: Bench) {
    let mut events = Events::open(&bench.machine.join(KMSG), "test: thermal warning").unwrap();
    let start = Instant::now();

    assert!(bench.guard.watch(&mut events, 0, &AtomicBool::new(true)));
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[rstest]
fn watching_ends_with_a_failure_when_the_log_fails_or_a_package_does(mut bench: Bench) {
    // A directory can be opened, but not read.
    let mut broken = Events::open(&bench.machine.join("dev"), "x").unwrap();
    let mut events = Events::open(&bench.machine.join(KMSG), "x").unwrap();
    let stop = AtomicBool::new(false);

    assert!(!bench.guard.watch(&mut broken, 0, &stop));
    bench.machine.remove(TEMP);
    assert!(!bench.guard.watch(&mut events, 0, &stop));
}

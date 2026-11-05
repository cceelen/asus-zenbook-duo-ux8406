//! What the program does with the orientation, the outputs and the
//! commands: the logic of the GNOME extension (`gnome-rotation/extension/
//! extension.js`), on KWin.
//!
//! - The laptop holds an orientation for [`SETTLE`]: the layout for it,
//!   from [`asus_ux8406_kde_common::rotation::rotate`], goes to KWin.
//! - KWin's own auto-rotate turns only the first built-in screen. The
//!   program sets its policy to `never` on each built-in screen, at the
//!   start and each time it changes, and keeps the policy from before in its
//!   configuration file, for `restore`.
//! - Where no sensor gives the orientation, the applet shows two buttons
//!   that turn all screens by 90 degrees.
//! - The applet turns each built-in screen by hand, to any of the four
//!   orientations. That puts the screens on hold ([`Hold`]) for the session:
//!   the sensor is not followed, so that it cannot undo the turn, and the
//!   rotation shows off. The saved setting does not change. `SetEnabled` and
//!   `Toggle` hand all screens back to the sensor. When the monitors change,
//!   each held screen that is there is turned again.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use asus_ux8406_kde_common::daemon::Handler;
use asus_ux8406_kde_common::geometry::{Monitor, State};
use asus_ux8406_kde_common::hold::{Action, Event as HoldEvent, Hold};
use asus_ux8406_kde_common::output::{Event, Policy, Session};
use asus_ux8406_kde_common::rotation::{
    Control, Direction, Orientation, Screen, buttons_from, control, orientation_of, rotate,
    screens, turn_one, turned, upright_after,
};
use log::{info, warn};
use zbus::blocking::Connection;

use crate::config::{self, Config};
use crate::sensor::{Change, Report, Worker};
use crate::service::{self, Command, Status};

/// The laptop must hold an orientation this long before the screens follow.
pub const SETTLE: Duration = Duration::from_millis(700);

/// What the program knows of the accelerometer, as the worker reported it.
#[derive(Clone, Copy, Debug)]
struct Accelerometer {
    /// Whether the sensor service gives the orientation.
    present: bool,
    /// Whether this program has a claim on it.
    claimed: bool,
    /// Whether the worker has the job to hold the claim.
    asked: bool,
    /// The orientation that the service gave last.
    orientation: Orientation,
}

/// What a layout that KWin has not answered yet is for.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Purpose {
    /// The screens follow the orientation of the laptop.
    Follow,
    /// A screen turned by hand, as a caller asked.
    Manual,
    /// A held screen turned again after the monitors changed.
    Again(String),
}

/// A layout that KWin has not answered yet.
struct Pending {
    number: u64,
    purpose: Purpose,
    /// The layout to remember as upright once KWin took it.
    upright: Option<Vec<Monitor>>,
    /// The orientation of the screens then, for the buttons.
    now: Option<Orientation>,
}

/// The state of the program.
pub struct Rotation {
    bus: Connection,
    sensor: Option<Worker>,
    path: PathBuf,
    config: Config,
    accelerometer: Accelerometer,
    /// The orientation that the buttons turned the screens to.
    manual: Option<Orientation>,
    /// Whether KWin has a built-in screen.
    builtin: bool,
    /// The layout from before the screens were turned.
    upright: Option<Vec<Monitor>>,
    /// When to follow the orientation.
    settle: Option<Instant>,
    /// The layouts that KWin has not answered yet.
    pending: Vec<Pending>,
    /// The numbers of the configurations that set policies, which KWin has
    /// not answered yet.
    policies: Vec<u64>,
    /// The screens that can be turned one by one.
    screens: Vec<Screen>,
    /// The screens that were turned by hand.
    hold: Hold,
    /// Whether a caller asked the program to quit: the program changes no
    /// policy and no layout then.
    stopping: Arc<AtomicBool>,
}

impl Rotation {
    /// The program, with its configuration, its service and the sensor (none
    /// without a system bus), and the flag that the service sets at `Quit`.
    /// It sets KWin's policies, claims the
    /// accelerometer if it can, and follows the orientation soon.
    pub fn new(
        session: &mut Session,
        bus: Connection,
        sensor: Option<Worker>,
        path: PathBuf,
        config: Config,
        stopping: Arc<AtomicBool>,
    ) -> Self {
        let mut rotation = Self {
            bus,
            sensor,
            path,
            config,
            accelerometer: Accelerometer {
                present: false,
                claimed: false,
                asked: false,
                orientation: Orientation::Undefined,
            },
            manual: None,
            builtin: false,
            upright: None,
            settle: None,
            pending: Vec::new(),
            policies: Vec::new(),
            screens: Vec::new(),
            hold: Hold::new(),
            stopping,
        };

        rotation.changed(session);
        if let Some(sensor) = &rotation.sensor {
            sensor.refresh(Change::Owner);
        }
        rotation
    }

    /// The control that the applet shows.
    fn control(&self) -> Control {
        control(false, self.accelerometer.present, self.builtin)
    }

    /// Whether the screens follow the orientation of the laptop: the saved
    /// setting is on and no screen was turned by hand.
    fn enabled(&self) -> bool {
        self.config.enabled && !self.hold.active()
    }

    /// The status for the applet.
    #[must_use]
    pub fn status(&self) -> Status {
        let from = buttons_from(self.manual);

        Status {
            enabled: self.enabled(),
            screens: self.screens.clone(),
            control: self.control(),
            clockwise: turned(from, Direction::Clockwise).is_some(),
            counterclockwise: turned(from, Direction::Counterclockwise).is_some(),
        }
    }

    fn stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    /// Do what the hold asks.
    fn act(&mut self, session: &mut Session, actions: Vec<Action>) {
        let mut again = None;

        for action in actions {
            match action {
                Action::Lock(locked) => {
                    self.config.enabled = !locked;
                    self.save();
                }
                Action::Turn {
                    connector,
                    orientation,
                } => {
                    again.get_or_insert((connector, orientation));
                }
                Action::Follow => self.settle_soon(),
            }
        }
        // One turn at a time: the next one starts from the layout that this
        // one makes, which comes with the next change of the monitors.
        if let Some((connector, orientation)) = again.filter(|_| !self.turning()) {
            self.turn_again(session, &connector, orientation);
        }
    }

    /// Whether a turn of one screen waits for KWin.
    fn turning(&self) -> bool {
        self.pending
            .iter()
            .any(|pending| pending.purpose != Purpose::Follow)
    }

    /// What follows a change of whether the rotation is on: the journal, the
    /// claim, the applet.
    fn shown(&mut self, before: bool, caller: &str) {
        if self.enabled() != before {
            info!("rotation {} ({caller})", if before { "off" } else { "on" });
        }
        self.claim();
        self.publish();
        self.settle_soon();
    }

    /// The switch: off when the screens follow the sensor, else on again, and
    /// all screens go back under the sensor.
    fn switch(&mut self, session: &mut Session, caller: &str) {
        let before = self.enabled();
        let held = self.hold.active();
        let actions = self.hold.handle(HoldEvent::Switch {
            locked: !self.config.enabled,
        });

        self.act(session, actions);
        if held {
            info!("{caller}: the screens go back under the sensor");
        }
        self.shown(before, caller);
    }

    /// Follow the orientation of the laptop, or not, and say who asked.
    fn set_enabled(&mut self, session: &mut Session, enabled: bool, caller: &str) {
        if enabled != self.enabled() {
            self.switch(session, caller);
        }
    }

    /// Turn one screen by hand. The screens are on hold from then on.
    fn turn_screen(&mut self, session: &mut Session, connector: &str, orientation: Orientation) {
        let before = self.enabled();
        let state = session.outputs.state();
        let stopping = self.stopping();
        let busy = stopping | self.turning();
        let layout = turn_one(&state, connector, orientation).filter(|_| !busy);

        if let Some(layout) = layout {
            self.hold.handle(HoldEvent::Turn {
                screens: screens(&state),
                connector: connector.to_owned(),
                orientation,
            });
            info!(
                "{connector}: turned to {} by hand (TurnScreen)",
                orientation.as_str()
            );
            self.apply_turn(session, &state, &layout, Purpose::Manual);
            self.shown(before, "TurnScreen");
        } else {
            warn!(
                "{connector}: cannot be turned to {} now (TurnScreen)",
                orientation.as_str()
            );
        }
    }

    /// A held screen is turned again after the monitors changed.
    fn turn_again(&mut self, session: &mut Session, connector: &str, orientation: Orientation) {
        let state = session.outputs.state();
        let stopping = self.stopping();

        turn_one(&state, connector, orientation)
            .filter(|_| !stopping)
            .into_iter()
            .for_each(|layout| {
                info!(
                    "{connector}: turned to {} again (the monitors changed)",
                    orientation.as_str()
                );
                self.apply_turn(
                    session,
                    &state,
                    &layout,
                    Purpose::Again(connector.to_owned()),
                );
            });
    }

    fn apply_turn(
        &mut self,
        session: &mut Session,
        state: &State,
        layout: &[Monitor],
        purpose: Purpose,
    ) {
        let after = State {
            logical: layout.to_vec(),
            ..state.clone()
        };
        let now = orientation_of(&after);

        self.pending.push(Pending {
            number: session.apply(layout),
            purpose,
            upright: upright_after(
                state,
                now.unwrap_or(Orientation::Undefined),
                self.upright.as_deref(),
            ),
            now,
        });
    }

    fn publish(&self) {
        service::publish(&self.bus, self.status());
    }

    fn save(&self) {
        if let Err(error) = config::save(&self.path, &self.config) {
            warn!("{error}");
        }
    }

    /// The outputs changed: KWin's policies, the control, the screens, and
    /// the layout soon, or the held screens turned again.
    fn changed(&mut self, session: &mut Session) {
        let state = session.outputs.state();

        self.never(session);
        self.builtin = !state.builtin.is_empty();
        self.manual = self.manual.or_else(|| orientation_of(&state));
        self.screens = screens(&state);

        let actions = self.hold.handle(HoldEvent::Monitors {
            screens: self.screens.clone(),
        });

        self.act(session, actions);
        self.publish();
    }

    /// Set KWin's auto-rotate policy to `never` where it is another one,
    /// and keep the one from before.
    fn never(&mut self, session: &mut Session) {
        let stopping = self.stopping();
        let others: Vec<(String, Policy)> = session
            .outputs
            .policies()
            .into_iter()
            .filter(|(_, policy)| *policy != Policy::Never && !stopping)
            .collect();

        if others.is_empty() {
            return;
        }
        for (connector, policy) in &others {
            info!(
                "{connector}: KWin's auto-rotate {} is now never",
                policy.as_str()
            );
            self.config
                .policies
                .insert(connector.clone(), policy.as_str().to_owned());
        }
        self.save();

        let never: Vec<(String, Policy)> = others
            .into_iter()
            .map(|(connector, _)| (connector, Policy::Never))
            .collect();

        // None: a KWin without the setting, where there is nothing to set.
        self.policies.extend(session.set_policies(&never));
    }

    fn settle_soon(&mut self) {
        self.settle = Some(Instant::now() + SETTLE);
    }

    /// The worker reports on the sensor service. After a new start of the
    /// service, the claim is gone.
    fn sensor_report(&mut self, session: &mut Session, report: Report) {
        match report {
            Report::State { change, snapshot } => {
                if change == Change::Owner {
                    self.accelerometer.claimed = false;
                    self.accelerometer.asked = false;
                }
                self.accelerometer.present = snapshot.present;
                self.accelerometer.orientation = snapshot.orientation;
                self.claim();
                self.publish();
            }
            Report::Claimed(claimed) => self.accelerometer.claimed = claimed,
        }

        let actions = self.hold.handle(HoldEvent::Sensor);

        self.act(session, actions);
    }

    /// Ask the worker for a claim while the screens follow the sensor, and
    /// only then: the service reads the accelerometer as long as a program
    /// has one.
    fn claim(&mut self) {
        let wanted = self.enabled() && self.accelerometer.present;

        if wanted != self.accelerometer.asked {
            self.accelerometer.asked = wanted;
            self.sensor.iter().for_each(|sensor| sensor.claim(wanted));
        }
    }

    /// The orientation that the screens follow: the one of the buttons
    /// where they are shown, else the one of the sensor while there is a
    /// claim and the rotation is on.
    fn orientation(&self) -> Option<Orientation> {
        if self.control() == Control::Buttons {
            return self.manual;
        }
        (self.enabled() && self.accelerometer.claimed).then_some(self.accelerometer.orientation)
    }

    fn follow(&mut self, session: &mut Session) {
        let state: State = session.outputs.state();
        let stopping = self.stopping();
        let held = self.hold.active();
        let away = stopping | held;
        let orientation = self.orientation().filter(|_| !away);
        let layout = orientation.and_then(|orientation| {
            rotate(
                &state,
                orientation,
                self.upright.as_deref(),
                self.hold.top(),
            )
        });

        match (orientation, layout) {
            (Some(orientation), Some(layout)) => {
                info!("turned to {}", orientation.as_str());
                self.pending.push(Pending {
                    number: session.apply(&layout),
                    purpose: Purpose::Follow,
                    upright: upright_after(&state, orientation, self.upright.as_deref()),
                    now: None,
                });
            }
            (Some(_), None) => {
                self.hold.handle(HoldEvent::Followed);
            }
            (None, _) => {}
        }
    }

    fn answered(&mut self, session: &mut Session, number: u64, refusal: Option<&str>) {
        if let Some(at) = self.policies.iter().position(|policy| *policy == number) {
            self.policies.remove(at);
            if let Some(reason) = refusal {
                warn!("KWin did not take the policy never: {reason}");
            }
        }
        let (answered, pending) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition::<Vec<_>, _>(|pending| pending.number == number);

        self.pending = pending;
        for pending in answered {
            match (refusal, pending.purpose) {
                (None, purpose) => {
                    self.upright = pending.upright;
                    if purpose == Purpose::Follow {
                        self.hold.handle(HoldEvent::Followed);
                    } else {
                        // The buttons go on from the orientation of the screens,
                        // if they have one.
                        self.manual = pending.now;
                    }
                }
                (Some(reason), Purpose::Follow) => {
                    warn!("KWin did not take the layout: {reason}");
                }
                (Some(reason), Purpose::Manual) => {
                    warn!("KWin did not take the turn: {reason}");
                    self.hold.handle(HoldEvent::Refused);
                    self.shown_after_refusal();
                }
                (Some(reason), Purpose::Again(connector)) => {
                    warn!("{connector} is not held any more: KWin did not take the turn: {reason}");

                    let actions = self.hold.handle(HoldEvent::Dropped { connector });

                    self.act(session, actions);
                    self.shown_after_refusal();
                }
            }
        }
    }

    /// The hold changed because KWin refused: the claim and the applet follow.
    fn shown_after_refusal(&mut self) {
        self.claim();
        self.publish();
    }
}

impl Handler for Rotation {
    type Command = Command;

    fn outputs(&mut self, session: &mut Session, events: Vec<Event>) {
        for event in events {
            match event {
                Event::Changed => self.changed(session),
                Event::Applied(number) => self.answered(session, number, None),
                Event::Failed(number, reason) => self.answered(session, number, Some(&reason)),
            }
        }
    }

    fn command(&mut self, session: &mut Session, command: Command) {
        match command {
            Command::SetEnabled(enabled) => self.set_enabled(session, enabled, "SetEnabled"),
            Command::Toggle => self.switch(session, "Toggle"),
            Command::Shortcut => self.switch(session, "shortcut"),
            Command::TurnScreen(connector, orientation) => {
                self.turn_screen(session, &connector, orientation);
            }
            Command::Turn(direction) => {
                let from = buttons_from(self.manual);
                let to = turned(from, direction).filter(|_| self.control() == Control::Buttons);

                if to.is_some() {
                    let before = self.enabled();

                    self.hold.handle(HoldEvent::All);
                    self.shown(before, "Turn");
                }
                self.manual = to.or(self.manual);
                self.publish();
                if to.is_some() {
                    self.follow(session);
                }
            }
            Command::Quit => self.stopping.store(true, Ordering::SeqCst),
            Command::Sensor(report) => self.sensor_report(session, report),
        }
    }

    fn tick(&mut self, session: &mut Session) -> Option<Instant> {
        if self.settle.is_some_and(|at| at <= Instant::now()) {
            self.settle = None;
            self.follow(session);
        }
        self.settle
    }

    fn done(&self) -> bool {
        self.stopping()
    }
}

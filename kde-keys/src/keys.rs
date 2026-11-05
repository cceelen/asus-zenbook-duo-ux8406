//! What the program does with the key and with the outputs.

use std::time::Instant;

use asus_ux8406_kde_common::daemon::Handler;
use asus_ux8406_kde_common::geometry::{Monitor, State};
use asus_ux8406_kde_common::layout::{Toggle, toggle};
use asus_ux8406_kde_common::output::{Event, Session};
use asus_ux8406_kde_common::touch::{Controller, assignments};
use log::{info, warn};
use zbus::blocking::Connection;

use crate::input;
use crate::service::Command;

/// The connector of the upper panel.
pub const UPPER: &str = "eDP-1";
/// The connector of the lower panel.
pub const LOWER: &str = "eDP-2";

/// The state of the program.
pub struct Keys {
    bus: Connection,
    controllers: Vec<Controller>,
    /// The layout from before the key switched the lower screen off.
    saved: Option<Vec<Monitor>>,
    /// The configurations that KWin has not answered yet, each with the
    /// layout to remember once KWin took it.
    pending: Vec<(u64, Option<Vec<Monitor>>)>,
}

impl Keys {
    /// The program with its connection to the session bus and the touch
    /// controllers of the machine.
    #[must_use]
    pub const fn new(bus: Connection, controllers: Vec<Controller>) -> Self {
        Self {
            bus,
            controllers,
            saved: None,
            pending: Vec::new(),
        }
    }

    /// Give each touchscreen and pen its panel, where it acts on no output.
    pub fn assign(&self, state: &State) {
        let assignments = assignments(&self.controllers, state, UPPER, LOWER);

        match input::devices(&self.bus) {
            Ok(devices) => {
                for (sys_name, connector) in input::plan(&devices, &assignments) {
                    match input::set_output(&self.bus, &sys_name, &connector) {
                        Ok(()) => info!("input device {sys_name}: on {connector}"),
                        Err(error) => warn!("input device {sys_name}: {error}"),
                    }
                }
            }
            Err(error) => warn!("KWin's input devices: {error}"),
        }
    }

    fn toggle(&mut self, session: &mut Session) {
        let Toggle { apply, save } = toggle(
            &session.outputs.state(),
            self.saved.as_deref(),
            LOWER,
            UPPER,
        );

        if let Some(layout) = apply {
            let number = session.apply(&layout);

            self.pending.push((number, save));
        } else {
            info!("the lower screen is not connected or not by itself: nothing to do");
            self.saved = save;
        }
    }

    fn answered(&mut self, number: u64, refusal: Option<&str>) {
        let (answered, pending) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition::<Vec<_>, _>(|(pending, _)| *pending == number);

        self.pending = pending;
        for (_, save) in answered {
            match refusal {
                None => self.saved = save,
                Some(reason) => warn!("KWin did not take the layout: {reason}"),
            }
        }
    }
}

impl Handler for Keys {
    type Command = Command;

    fn outputs(&mut self, session: &mut Session, events: Vec<Event>) {
        for event in events {
            match event {
                Event::Changed => self.assign(&session.outputs.state()),
                Event::Applied(number) => self.answered(number, None),
                Event::Failed(number, reason) => self.answered(number, Some(&reason)),
            }
        }
    }

    fn command(&mut self, session: &mut Session, command: Command) {
        match command {
            Command::Toggle => self.toggle(session),
            Command::Devices => self.assign(&session.outputs.state()),
        }
    }

    fn tick(&mut self, _: &mut Session) -> Option<Instant> {
        None
    }

    fn done(&self) -> bool {
        false
    }
}

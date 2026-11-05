//! The hold of the manual turns: what the program remembers while panels are
//! turned by hand, and what it does when something happens.
//!
//! This is `gnome-rotation/extension/hold.js` on plain data.
//!
//! A turn by hand puts the screens on hold: the sensor is not followed, so
//! that it cannot undo the turn, and each panel has the orientation that it
//! was turned to. The hold is for the session. The saved setting of the
//! rotation is not part of it: [`Action::Lock`] comes only from the switch of
//! the user.

use std::collections::BTreeMap;

use crate::rotation::{Orientation, Screen};

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A panel is to be turned by hand. `screens` is the list of
    /// [`crate::rotation::screens`].
    Turn {
        /// The screens now.
        screens: Vec<Screen>,
        /// The panel to turn.
        connector: String,
        /// Its new orientation.
        orientation: Orientation,
    },
    /// The compositor did not take that turn.
    Refused,
    /// All panels were turned together (the buttons): there is no hold on
    /// one by one any more.
    All,
    /// The user used the switch; `locked` is the saved setting.
    Switch {
        /// Whether the rotation is off in the saved setting.
        locked: bool,
    },
    /// The monitors changed.
    Monitors {
        /// The screens now.
        screens: Vec<Screen>,
    },
    /// The compositor did not take the held orientation of that panel again.
    Dropped {
        /// The panel.
        connector: String,
    },
    /// The sensor gave an orientation.
    Sensor,
    /// The panels were brought to the orientation of the sensor, or were
    /// there already.
    Followed,
}

/// What the program does about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Set the saved setting of the rotation to off (`true`) or on.
    Lock(bool),
    /// Turn a panel to its held orientation.
    Turn {
        /// The panel.
        connector: String,
        /// The orientation.
        orientation: Orientation,
    },
    /// Bring the panels to the orientation that is followed.
    Follow,
}

/// The panels that were turned by hand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    /// The orientation of each panel that was turned by hand, by connector. A
    /// panel that is gone keeps its place until it comes back.
    held: BTreeMap<String, Orientation>,
    /// The same before the last turn, to put back if it is refused.
    before: BTreeMap<String, Orientation>,
    /// The connectors from the top of the laptop to its bottom: the order of
    /// the compositor's list of built-in panels. It stays until the panels
    /// follow the sensor again, which needs it for the panels that are turned
    /// differently.
    top: Option<Vec<String>>,
}

impl Hold {
    /// Nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether panels are on hold: the sensor is not followed.
    #[must_use]
    pub fn active(&self) -> bool {
        !self.held.is_empty()
    }

    /// The order of the panels, or none when it is not needed.
    #[must_use]
    pub fn top(&self) -> Option<&[String]> {
        self.top.as_deref()
    }

    /// The held orientations, for [`crate::rotation::turn_many`].
    #[must_use]
    pub const fn held(&self) -> &BTreeMap<String, Orientation> {
        &self.held
    }

    /// Whether the switch shows on: not locked and not on hold.
    #[must_use]
    pub fn is_auto(&self, locked: bool) -> bool {
        !locked && !self.active()
    }

    /// The actions to take for an event.
    pub fn handle(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Turn {
                screens,
                connector,
                orientation,
            } => {
                self.turn(&screens, connector, orientation);
                Vec::new()
            }
            Event::Refused => {
                self.held.clone_from(&self.before);
                if !self.active() {
                    self.top = None;
                }
                Vec::new()
            }
            Event::All => {
                self.held.clear();
                Vec::new()
            }
            Event::Switch { locked } => self.switch(locked),
            Event::Monitors { screens } => self.monitors(&screens),
            Event::Dropped { connector } => {
                self.held.remove(&connector);
                self.follow_unless_held()
            }
            Event::Sensor => self.follow_unless_held(),
            Event::Followed => {
                if !self.active() {
                    self.top = None;
                }
                Vec::new()
            }
        }
    }

    fn follow_unless_held(&self) -> Vec<Action> {
        if self.active() {
            Vec::new()
        } else {
            vec![Action::Follow]
        }
    }

    fn turn(&mut self, screens: &[Screen], connector: String, orientation: Orientation) {
        self.before.clone_from(&self.held);
        if !self.active() {
            // The other panels keep the orientation that they have now.
            self.held = screens
                .iter()
                .filter_map(|screen| {
                    screen
                        .orientation
                        .map(|orientation| (screen.connector.clone(), orientation))
                })
                .collect();
            self.top = Some(
                screens
                    .iter()
                    .map(|screen| screen.connector.clone())
                    .collect(),
            );
        }
        self.held.insert(connector, orientation);
    }

    fn switch(&mut self, locked: bool) -> Vec<Action> {
        if self.is_auto(locked) {
            return vec![Action::Lock(true)];
        }
        // On again: the panels go back under the sensor.
        self.held.clear();
        vec![Action::Lock(false)]
    }

    fn monitors(&self, screens: &[Screen]) -> Vec<Action> {
        if !self.active() {
            return vec![Action::Follow];
        }
        // The compositor changed the layout: each panel that is there and has
        // another orientation than the held one is turned again. A mirrored
        // panel (no orientation) is the user's own setting.
        screens
            .iter()
            .filter_map(|screen| {
                let held = self.held.get(&screen.connector)?;

                screen
                    .orientation
                    .filter(|now| now != held)
                    .map(|_| Action::Turn {
                        connector: screen.connector.clone(),
                        orientation: *held,
                    })
            })
            .collect()
    }
}

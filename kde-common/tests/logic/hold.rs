//! The hold of the manual turns: the cases of `gnome-rotation/tests/hold.test.js`.

use std::collections::BTreeMap;

use asus_ux8406_kde_common::hold::{Action, Event, Hold};
use asus_ux8406_kde_common::rotation::{Orientation, Screen};

use Orientation::{BottomUp, LeftUp, Normal};

const UPPER: &str = "eDP-1";
const LOWER: &str = "eDP-2";

fn screen(connector: &str, name: &str, orientation: Option<Orientation>) -> Screen {
    Screen {
        connector: connector.to_owned(),
        name: name.to_owned(),
        orientation,
    }
}

fn screens(upper: Option<Orientation>, lower: Option<Orientation>) -> Vec<Screen> {
    vec![
        screen(UPPER, "Upper screen", upper),
        screen(LOWER, "Lower screen", lower),
    ]
}

fn turn(
    hold: &mut Hold,
    list: Vec<Screen>,
    connector: &str,
    orientation: Orientation,
) -> Vec<Action> {
    hold.handle(Event::Turn {
        screens: list,
        connector: connector.to_owned(),
        orientation,
    })
}

fn held(pairs: &[(&str, Orientation)]) -> BTreeMap<String, Orientation> {
    pairs
        .iter()
        .map(|(connector, orientation)| ((*connector).to_owned(), *orientation))
        .collect()
}

fn order() -> Vec<String> {
    vec![UPPER.to_owned(), LOWER.to_owned()]
}

fn turn_again(connector: &str, orientation: Orientation) -> Action {
    Action::Turn {
        connector: connector.to_owned(),
        orientation,
    }
}

#[test]
fn nothing_is_held_at_the_start_and_the_switch_shows_the_setting() {
    let hold = Hold::new();

    assert!(!hold.active());
    assert_eq!(hold.top(), None);
    assert!(hold.held().is_empty());
    assert!(hold.is_auto(false));
    assert!(!hold.is_auto(true));
}

#[test]
fn a_turn_by_hand_holds_all_panels_and_the_others_as_they_are() {
    let mut hold = Hold::new();

    assert_eq!(
        turn(
            &mut hold,
            screens(Some(Normal), Some(Normal)),
            LOWER,
            BottomUp
        ),
        []
    );
    assert!(hold.active());
    assert_eq!(hold.held(), &held(&[(UPPER, Normal), (LOWER, BottomUp)]));
    assert_eq!(hold.top(), Some(order().as_slice()));
    assert!(!hold.is_auto(false));
}

#[test]
fn a_mirrored_panel_has_no_orientation_to_hold() {
    let mut hold = Hold::new();

    turn(&mut hold, screens(None, Some(Normal)), LOWER, LeftUp);
    assert_eq!(hold.held(), &held(&[(LOWER, LeftUp)]));
}

#[test]
fn a_second_turn_changes_one_panel_and_keeps_the_order() {
    let mut hold = Hold::new();
    let mut reversed = screens(Some(Normal), Some(BottomUp));

    reversed.reverse();
    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    turn(&mut hold, reversed, UPPER, LeftUp);
    assert_eq!(hold.held(), &held(&[(UPPER, LeftUp), (LOWER, BottomUp)]));
    assert_eq!(hold.top(), Some(order().as_slice()));
}

#[test]
fn a_turn_that_is_refused_goes_back_to_what_was_held() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    turn(
        &mut hold,
        screens(Some(Normal), Some(BottomUp)),
        UPPER,
        LeftUp,
    );
    assert_eq!(hold.handle(Event::Refused), []);
    assert_eq!(hold.held(), &held(&[(UPPER, Normal), (LOWER, BottomUp)]));
    assert!(hold.active());
    assert_eq!(hold.top(), Some(order().as_slice()));
}

#[test]
fn the_first_turn_is_refused_so_there_is_no_hold_and_no_order() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    hold.handle(Event::Refused);
    assert!(!hold.active());
    assert_eq!(hold.top(), None);
}

#[test]
fn the_switch_is_off_when_on_and_on_again_after_a_hold_or_a_lock() {
    let mut hold = Hold::new();

    assert_eq!(
        hold.handle(Event::Switch { locked: false }),
        [Action::Lock(true)]
    );
    // Locked in the setting: on again.
    assert_eq!(
        hold.handle(Event::Switch { locked: true }),
        [Action::Lock(false)]
    );
    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    // On hold the switch shows off, whatever the setting says.
    assert_eq!(
        hold.handle(Event::Switch { locked: false }),
        [Action::Lock(false)]
    );
    assert!(!hold.active());
    // The order stays for the panels that are turned differently, until they
    // follow.
    assert_eq!(hold.top(), Some(order().as_slice()));
    hold.handle(Event::Followed);
    assert_eq!(hold.top(), None);
}

#[test]
fn the_panels_follow_later_while_a_hold_is_on_and_the_order_stays() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    hold.handle(Event::Followed);
    assert_eq!(hold.top(), Some(order().as_slice()));
}

#[test]
fn the_sensor_is_followed_only_without_a_hold() {
    let mut hold = Hold::new();

    assert_eq!(hold.handle(Event::Sensor), [Action::Follow]);
    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    assert_eq!(hold.handle(Event::Sensor), []);
}

#[test]
fn monitors_that_change_without_a_hold_follow_the_sensor() {
    assert_eq!(
        Hold::new().handle(Event::Monitors {
            screens: screens(Some(Normal), Some(Normal))
        }),
        [Action::Follow]
    );
}

#[test]
fn monitors_that_change_on_hold_turn_each_panel_that_is_there_again() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    // The compositor made both upright.
    assert_eq!(
        hold.handle(Event::Monitors {
            screens: screens(Some(Normal), Some(Normal))
        }),
        [turn_again(LOWER, BottomUp)]
    );
    // Already as held: nothing to do.
    assert_eq!(
        hold.handle(Event::Monitors {
            screens: screens(Some(Normal), Some(BottomUp))
        }),
        []
    );
}

#[test]
fn a_panel_that_is_gone_keeps_its_orientation_until_it_is_back() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    // The lower panel is off: only the upper one is listed.
    assert_eq!(
        hold.handle(Event::Monitors {
            screens: vec![screen(UPPER, "Upper screen", Some(Normal))]
        }),
        []
    );
    assert_eq!(hold.held(), &held(&[(UPPER, Normal), (LOWER, BottomUp)]));
    assert_eq!(
        hold.handle(Event::Monitors {
            screens: screens(Some(Normal), Some(Normal))
        }),
        [turn_again(LOWER, BottomUp)]
    );
}

#[test]
fn a_mirrored_panel_is_not_turned_again_and_one_that_is_not_held_is_left_alone() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    assert_eq!(
        hold.handle(Event::Monitors {
            screens: screens(Some(Normal), None)
        }),
        []
    );

    let mut with_other = screens(Some(Normal), Some(BottomUp));

    with_other.push(screen("eDP-3", "Built-in screen 3", Some(LeftUp)));
    assert_eq!(
        hold.handle(Event::Monitors {
            screens: with_other
        }),
        []
    );
}

#[test]
fn a_turn_again_that_is_refused_drops_the_hold_of_that_panel() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    assert_eq!(
        hold.handle(Event::Dropped {
            connector: LOWER.to_owned()
        }),
        []
    );
    assert_eq!(hold.held(), &held(&[(UPPER, Normal)]));
    assert!(hold.active());
    // The last one: the sensor is followed again.
    assert_eq!(
        hold.handle(Event::Dropped {
            connector: UPPER.to_owned()
        }),
        [Action::Follow]
    );
    assert!(!hold.active());
}

#[test]
fn the_buttons_turn_all_panels_together_so_there_is_no_hold_one_by_one() {
    let mut hold = Hold::new();

    turn(
        &mut hold,
        screens(Some(Normal), Some(Normal)),
        LOWER,
        BottomUp,
    );
    assert_eq!(hold.handle(Event::All), []);
    assert!(!hold.active());
}

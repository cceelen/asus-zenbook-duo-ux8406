//! The layout for an orientation of the laptop.

use asus_ux8406_kde_common::geometry::{Monitor, State, find, is_valid};
use asus_ux8406_kde_common::rotation::{
    Control, Direction, Orientation, control, orientation_of, rotate, screens, turn_one, turned,
    upright_after,
};
use rstest::rstest;

use crate::fixtures::{
    LOWER, PANEL, UPPER, at, logical, lower_off, panel, place, size, state, two_panels,
};

use Orientation::{BottomUp, LeftUp, Normal, RightUp, Undefined};

/// The state with the layout that a turn gave.
fn turned_to(state: &State, orientation: Orientation) -> State {
    State {
        logical: rotate(state, orientation, None, None).expect("a layout"),
        ..state.clone()
    }
}

fn beside(x: i32, y: i32) -> State {
    state(
        &[
            (UPPER, Some(PANEL)),
            (LOWER, Some(PANEL)),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(UPPER, 0, 0, false),
            panel(LOWER, 0, 1440, true),
            logical("DP-7", x, y, false, 1.0),
        ],
    )
}

#[test]
fn left_side_up_turns_both_panels_and_the_lower_one_goes_left() {
    let layout = rotate(&two_panels(), LeftUp, None, None).expect("a layout");

    assert_eq!(place(&layout, LOWER), (0, 0, 1));
    assert_eq!(place(&layout, UPPER), (1440, 0, 1));
    assert!(find(&layout, LOWER).expect("lower").primary);
}

#[test]
fn right_side_up_turns_both_panels_and_the_lower_one_goes_right() {
    let layout = rotate(&two_panels(), RightUp, None, None).expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 3));
    assert_eq!(place(&layout, LOWER), (1440, 0, 3));
}

#[test]
fn upright_again_puts_the_lower_panel_below_the_upper_one() {
    let layout = rotate(&turned_to(&two_panels(), LeftUp), Normal, None, None).expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 0));
    assert_eq!(place(&layout, LOWER), (0, 1440, 0));
}

#[test]
fn a_turn_goes_from_one_side_to_the_other() {
    let layout = rotate(&turned_to(&two_panels(), LeftUp), RightUp, None, None).expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 3));
    assert_eq!(place(&layout, LOWER), (1440, 0, 3));
}

#[test]
fn upside_down_turns_both_panels_and_the_lower_one_goes_on_top() {
    let layout = rotate(&two_panels(), BottomUp, None, None).expect("a layout");

    assert_eq!(place(&layout, LOWER), (0, 0, 2));
    assert_eq!(place(&layout, UPPER), (0, 1440, 2));
    assert!(find(&layout, LOWER).expect("lower").primary);
}

#[test]
fn upright_again_after_upside_down_puts_the_layout_from_before_back() {
    let upright = two_panels();
    let turned = turned_to(&upright, BottomUp);
    let layout = rotate(
        &turned,
        Normal,
        upright_after(&upright, BottomUp, None).as_deref(),
        None,
    )
    .expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 0));
    assert_eq!(place(&layout, LOWER), (0, 1440, 0));
}

#[test]
fn a_turn_goes_from_a_side_to_upside_down() {
    let layout = rotate(&turned_to(&two_panels(), LeftUp), BottomUp, None, None).expect("a layout");

    assert_eq!(place(&layout, LOWER), (0, 0, 2));
    assert_eq!(place(&layout, UPPER), (0, 1440, 2));
}

#[test]
fn upside_down_with_the_lower_panel_off_turns_only_the_upper_one() {
    let layout = rotate(&lower_off(), BottomUp, None, None).expect("a layout");

    assert_eq!(layout.len(), 1);
    assert_eq!(place(&layout, UPPER), (0, 0, 2));
}

#[test]
fn with_the_lower_panel_off_only_the_upper_one_is_turned() {
    let layout = rotate(&lower_off(), LeftUp, None, None).expect("a layout");

    assert_eq!(layout.len(), 1);
    assert_eq!(place(&layout, UPPER), (0, 0, 1));
}

#[test]
fn nothing_is_done_when_the_layout_is_that_one_already() {
    assert_eq!(rotate(&two_panels(), Normal, None, None), None);
    assert_eq!(
        rotate(&turned_to(&two_panels(), RightUp), RightUp, None, None),
        None
    );
}

#[rstest]
fn an_orientation_that_is_not_known_changes_nothing(#[values(Undefined)] orientation: Orientation) {
    assert_eq!(rotate(&two_panels(), orientation, None, None), None);
}

#[test]
fn with_the_built_in_panels_off_nothing_is_turned() {
    let mut state = state(
        &[(UPPER, Some(PANEL)), ("DP-7", Some(PANEL))],
        vec![logical("DP-7", 0, 0, true, 1.0)],
    );

    assert_eq!(rotate(&state, LeftUp, None, None), None);
    state.builtin.clear();
    assert_eq!(rotate(&state, LeftUp, None, None), None);
}

#[rstest]
fn a_mirrored_panel_is_left_as_it_is(
    #[values(4, 5, 6, 7)] transform: u32,
    #[values(Normal, LeftUp, RightUp, BottomUp)] orientation: Orientation,
) {
    let mut state = two_panels();

    for monitor in &mut state.logical {
        monitor.transform = transform;
    }
    assert_eq!(rotate(&state, orientation, None, None), None);
}

#[test]
fn a_monitor_without_a_mode_stops_the_turn() {
    let mut state = beside(2304, 0);

    state.connected.insert("DP-7".to_owned(), None);
    assert_eq!(rotate(&state, LeftUp, None, None), None);
}

#[test]
fn the_other_panel_tells_the_order_when_the_compositor_turned_the_first_back() {
    let mut state = two_panels();

    state.logical = vec![
        Monitor {
            x: 1440,
            ..panel(UPPER, 0, 0, false)
        },
        Monitor {
            transform: 3,
            ..panel(LOWER, 0, 0, true)
        },
    ];

    let layout = rotate(&state, RightUp, None, None).expect("a layout");

    assert_eq!(place(&layout, LOWER), (0, 0, 3));
    assert_eq!(place(&layout, UPPER), (1440, 0, 3));
}

#[test]
fn a_monitor_right_of_the_panels_stays_right_of_them() {
    let layout = rotate(&beside(2304, 200), LeftUp, None, None).expect("a layout");

    assert_eq!(place(&layout, "DP-7"), (2880, 200, 0));
    assert_eq!(place(&layout, UPPER), (1440, 0, 1));
}

#[test]
fn a_monitor_below_the_panels_stays_below_them() {
    let layout = rotate(&beside(300, 2880), LeftUp, None, None).expect("a layout");

    assert_eq!(place(&layout, "DP-7"), (300, 2304, 0));
    assert_eq!(place(&layout, LOWER), (0, 0, 1));
}

#[test]
fn a_monitor_left_of_the_panels_stays_where_it_is() {
    let state = state(
        &[
            (UPPER, Some(PANEL)),
            (LOWER, Some(PANEL)),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            logical("DP-7", 0, 0, false, 1.0),
            panel(UPPER, 2880, 0, false),
            panel(LOWER, 2880, 1440, true),
        ],
    );
    let layout = rotate(&state, RightUp, None, None).expect("a layout");

    assert_eq!(place(&layout, "DP-7"), (0, 0, 0));
    assert_eq!(place(&layout, UPPER), (2880, 0, 3));
    assert_eq!(place(&layout, LOWER), (4320, 0, 3));
}

#[test]
fn a_monitor_that_is_not_beside_the_panels_stays_where_it_is() {
    let state = state(
        &[
            (UPPER, Some(PANEL)),
            ("DP-8", Some(size(2880, 1800))),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(UPPER, 760, 1800, false),
            logical("DP-8", 0, 0, true, 1.0),
            logical("DP-7", 2880, 0, false, 1.0),
        ],
    );
    let turned = turned_to(&state, LeftUp);

    assert_eq!(place(&turned.logical, "DP-7"), (2880, 0, 0));

    let layout = rotate(&turned, Normal, None, None).expect("a layout");

    assert_eq!(place(&layout, "DP-7"), (2880, 0, 0));
    assert_eq!(place(&layout, UPPER), (760, 1800, 0));
}

#[test]
fn upright_again_the_layout_from_before_the_turn_is_put_back() {
    let mut before = two_panels();

    at(&mut before.logical, LOWER).x = 300;

    let turned = turned_to(&before, LeftUp);
    let layout = rotate(&turned, Normal, Some(&before.logical), None).expect("a layout");

    assert_eq!(layout, before.logical);
    assert_eq!(
        place(
            &rotate(&turned, Normal, None, None).expect("a layout"),
            LOWER
        ),
        (0, 1440, 0)
    );
}

#[test]
fn the_layout_from_before_the_turn_is_not_used_with_other_monitors() {
    let turned = turned_to(&lower_off(), LeftUp);
    let layout = rotate(&turned, Normal, Some(&two_panels().logical), None).expect("a layout");

    assert_eq!(layout.len(), 1);
    assert_eq!(place(&layout, UPPER), (0, 0, 0));
}

#[test]
fn the_layout_from_before_the_turn_is_not_used_at_another_scale() {
    let before = two_panels();
    let mut turned = turned_to(&before, LeftUp);

    for monitor in &mut turned.logical {
        monitor.scale = 1.0;
    }

    let layout = rotate(&turned, Normal, Some(&before.logical), None).expect("a layout");

    assert!(is_valid(&turned, &layout));
    assert_eq!(place(&layout, LOWER), (0, 1800, 0));
}

#[test]
fn the_layout_from_before_the_turn_is_not_used_for_a_turn_away_from_upright() {
    let before = two_panels();
    let layout = rotate(&before, LeftUp, Some(&before.logical), None).expect("a layout");

    assert_eq!(place(&layout, LOWER), (0, 0, 1));
}

#[test]
fn a_remembered_layout_that_is_in_place_already_changes_nothing() {
    assert_eq!(
        rotate(&two_panels(), Normal, Some(&two_panels().logical), None),
        None
    );
}

#[test]
fn the_order_of_the_panels_comes_from_the_layout_not_from_their_names() {
    let state = state(
        &[(UPPER, Some(PANEL)), (LOWER, Some(PANEL))],
        vec![panel(LOWER, 0, 0, false), panel(UPPER, 0, 1440, true)],
    );
    let layout = rotate(&state, LeftUp, None, None).expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 1));
    assert_eq!(place(&layout, LOWER), (1440, 0, 1));
}

/// Both panels turned to `transform`, the lower one at (`lower`) and the
/// upper one at (`upper`): the places need not make a valid layout.
fn turned_panels(transform: u32, upper: (i32, i32), lower: (i32, i32)) -> State {
    let mut state = state(
        &[(UPPER, Some(PANEL)), (LOWER, Some(PANEL))],
        vec![
            panel(UPPER, upper.0, upper.1, false),
            panel(LOWER, lower.0, lower.1, true),
        ],
    );

    for monitor in &mut state.logical {
        monitor.transform = transform;
    }
    state
}

#[rstest]
#[case::left_side_up(1, (1440, 3000), (0, 0))]
#[case::right_side_up(3, (0, 3000), (1440, 0))]
fn the_order_of_turned_panels_comes_from_one_axis_only(
    #[case] transform: u32,
    #[case] upper: (i32, i32),
    #[case] lower: (i32, i32),
) {
    // The panels lie far apart in the other axis: it does not count.
    let layout =
        rotate(&turned_panels(transform, upper, lower), Normal, None, None).expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 0));
    assert_eq!(place(&layout, LOWER), (0, 1440, 0));
}

#[test]
fn panels_of_the_same_place_keep_their_order() {
    let state = state(
        &[(UPPER, Some(PANEL)), (LOWER, Some(PANEL))],
        vec![panel(UPPER, 0, 0, true), panel(LOWER, 0, 0, false)],
    );

    assert_eq!(
        rotate(&state, RightUp, None, None).map(|layout| place(&layout, UPPER)),
        Some((0, 0, 3))
    );
}

#[test]
fn a_laptop_with_one_panel_has_that_one_turned() {
    let state = state(&[("eDP-3", Some(PANEL))], vec![panel("eDP-3", 0, 0, true)]);
    let mut state = state;

    state.builtin = vec!["eDP-3".to_owned()];
    assert_eq!(
        place(
            &rotate(&state, RightUp, None, None).expect("a layout"),
            "eDP-3"
        ),
        (0, 0, 3)
    );
}

fn at_the_dock() -> State {
    let mut state = state(
        &[
            (UPPER, Some(PANEL)),
            (LOWER, Some(PANEL)),
            ("DP-8", Some(size(2560, 1440))),
            ("DP-7", Some(size(5120, 2160))),
        ],
        vec![
            logical("DP-8", 1040, 0, false, 1.0),
            panel(LOWER, 0, 1440, false),
            logical("DP-7", 3600, 0, true, 1.0),
            panel(UPPER, 1800, 1440, false),
        ],
    );

    at(&mut state.logical, UPPER).transform = 1;
    at(&mut state.logical, LOWER).transform = 1;
    state.logical[1].scale = 1.0;
    state.logical[3].scale = 1.0;
    state
}

#[test]
fn the_layout_seen_on_the_hardware_is_one_the_compositor_can_take() {
    let state = at_the_dock();

    assert!(is_valid(&state, &state.logical));
}

#[test]
fn upright_from_a_saved_turned_layout_no_monitor_lies_over_another() {
    let state = at_the_dock();
    let layout = rotate(&state, Normal, None, None).expect("a layout");

    assert!(is_valid(&state, &layout));
    assert_eq!(place(&layout, UPPER).2, 0);
    assert_eq!(place(&layout, LOWER).2, 0);
    assert!(place(&layout, UPPER).1 < place(&layout, LOWER).1);
    assert_eq!(place(&layout, "DP-7"), (3600, 0, 0));
    assert_eq!(place(&layout, "DP-8"), (1040, 0, 0));
}

#[test]
fn when_nothing_else_fits_the_monitors_are_put_in_one_row() {
    let state = state(
        &[
            (UPPER, Some(PANEL)),
            ("DP-1", Some(size(2304, 600))),
            ("DP-2", Some(size(1000, 500))),
        ],
        vec![
            logical("DP-2", 2304, 1440, false, 1.0),
            panel(UPPER, 0, 0, true),
            logical("DP-1", 0, 1440, false, 1.0),
        ],
    );
    let layout = rotate(&state, LeftUp, None, None).expect("a layout");

    assert!(is_valid(&state, &layout));
    assert_eq!(place(&layout, UPPER), (0, 0, 1));
    assert_eq!(place(&layout, "DP-1"), (1440, 0, 0));
    assert_eq!(place(&layout, "DP-2"), (3744, 0, 0));
    assert_eq!(
        layout
            .iter()
            .map(|monitor| monitor.connectors[0].as_str())
            .collect::<Vec<_>>(),
        ["DP-2", UPPER, "DP-1"]
    );
}

#[test]
fn panels_that_are_upright_with_the_laptop_upright_are_left_where_they_are() {
    let state = state(
        &[
            (UPPER, Some(PANEL)),
            (LOWER, Some(PANEL)),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(LOWER, 0, 0, false),
            panel(UPPER, 2304, 0, true),
            logical("DP-7", 4608, 0, false, 1.0),
        ],
    );

    assert_eq!(rotate(&state, Normal, None, None), None);
    assert_eq!(
        rotate(&state, Normal, Some(&two_panels().logical), None),
        None
    );
}

#[test]
fn panels_that_are_turned_as_the_laptop_is_are_left_where_they_are() {
    let mut state = turned_to(&two_panels(), LeftUp);

    at(&mut state.logical, UPPER).x = 0;
    at(&mut state.logical, LOWER).x = 1440;
    assert_eq!(rotate(&state, LeftUp, None, None), None);
}

#[test]
fn panels_turned_differently_are_all_turned_to_the_orientation() {
    let mut state = two_panels();

    state.logical[0].transform = 1;
    assert!(rotate(&state, RightUp, None, None).is_some());
    assert!(rotate(&state, Normal, None, None).is_some());
    assert!(rotate(&state, LeftUp, None, None).is_some());
}

#[test]
fn the_layout_to_remember_is_the_one_from_before_the_first_turn() {
    let upright = two_panels();
    let turned = turned_to(&upright, LeftUp);

    assert_eq!(
        upright_after(&upright, LeftUp, None),
        Some(upright.logical.clone())
    );
    assert_eq!(
        upright_after(&turned, RightUp, Some(&upright.logical)),
        Some(upright.logical.clone())
    );
    assert_eq!(upright_after(&turned, Normal, Some(&upright.logical)), None);
    assert_eq!(upright_after(&turned, BottomUp, None), None);
}

#[test]
fn the_switch_is_for_a_sensor_else_the_buttons() {
    assert_eq!(control(false, true, true), Control::Switch);
    assert_eq!(control(false, false, true), Control::Buttons);
    assert_eq!(control(false, true, false), Control::None);
    assert_eq!(control(false, false, false), Control::None);
    assert_eq!(control(true, true, true), Control::None);
    assert_eq!(control(true, false, true), Control::None);
}

#[rstest]
#[case(Control::None, "none")]
#[case(Control::Switch, "switch")]
#[case(Control::Buttons, "buttons")]
fn a_control_has_a_name(#[case] control: Control, #[case] name: &str) {
    assert_eq!(control.as_str(), name);
}

#[rstest]
#[case(Normal, Direction::Clockwise, Some(RightUp))]
#[case(Normal, Direction::Counterclockwise, Some(LeftUp))]
#[case(LeftUp, Direction::Clockwise, Some(Normal))]
#[case(RightUp, Direction::Counterclockwise, Some(Normal))]
#[case(RightUp, Direction::Clockwise, None)]
#[case(LeftUp, Direction::Counterclockwise, None)]
#[case(BottomUp, Direction::Clockwise, None)]
#[case(Undefined, Direction::Counterclockwise, None)]
fn the_buttons_turn_by_90_degrees_from_left_side_up_to_right_side_up(
    #[case] from: Orientation,
    #[case] direction: Direction,
    #[case] to: Option<Orientation>,
) {
    assert_eq!(turned(from, direction), to);
}

#[test]
fn the_orientation_comes_from_the_turn_of_the_built_in_panels() {
    let upright = two_panels();

    assert_eq!(orientation_of(&upright), Some(Normal));
    assert_eq!(orientation_of(&lower_off()), Some(Normal));
    for orientation in [LeftUp, RightUp, BottomUp] {
        assert_eq!(
            orientation_of(&turned_to(&upright, orientation)),
            Some(orientation)
        );
    }
}

#[test]
fn panels_turned_differently_have_no_orientation() {
    let mut mixed = two_panels();

    mixed.logical[0].transform = 1;
    assert_eq!(orientation_of(&mixed), None);
    mixed.logical[0].transform = 2;
    assert_eq!(orientation_of(&mixed), None);
}

#[test]
fn without_a_panel_shown_by_itself_there_is_no_orientation() {
    let mut state = lower_off();

    state.builtin.clear();
    assert_eq!(orientation_of(&state), None);
}

#[rstest]
#[case("normal", Normal)]
#[case("left-up", LeftUp)]
#[case("right-up", RightUp)]
#[case("bottom-up", BottomUp)]
#[case("undefined", Undefined)]
#[case("constructor", Undefined)]
#[case("", Undefined)]
#[case("Normal", Undefined)]
fn the_text_of_the_sensor_gives_an_orientation(
    #[case] text: &str,
    #[case] orientation: Orientation,
) {
    assert_eq!(Orientation::from_sensor(text), orientation);
}

#[rstest]
fn an_orientation_has_the_text_of_the_sensor(
    #[values(Normal, LeftUp, RightUp, BottomUp, Undefined)] orientation: Orientation,
) {
    assert_eq!(Orientation::from_sensor(orientation.as_str()), orientation);
}

#[rstest]
#[case("clockwise", Some(Direction::Clockwise))]
#[case("counterclockwise", Some(Direction::Counterclockwise))]
#[case("sideways", None)]
#[case("", None)]
fn a_direction_is_read_from_text(#[case] text: &str, #[case] direction: Option<Direction>) {
    assert_eq!(Direction::parse(text), direction);
}

fn names(state: &State) -> Vec<(String, String, Option<Orientation>)> {
    screens(state)
        .into_iter()
        .map(|screen| (screen.connector, screen.name, screen.orientation))
        .collect()
}

#[test]
fn two_built_in_screens_are_the_upper_and_the_lower_one() {
    let mut state = two_panels();

    state.logical[1].transform = 2;
    assert_eq!(
        names(&state),
        [
            (UPPER.to_owned(), "Upper screen".to_owned(), Some(Normal)),
            (LOWER.to_owned(), "Lower screen".to_owned(), Some(BottomUp)),
        ]
    );
}

#[test]
fn one_built_in_screen_has_a_plain_name_and_a_screen_that_is_off_is_left_out() {
    assert_eq!(
        names(&lower_off()),
        [(UPPER.to_owned(), "Upper screen".to_owned(), Some(Normal))]
    );

    let mut single = state(&[(UPPER, Some(PANEL))], vec![panel(UPPER, 0, 0, true)]);

    single.builtin = vec![UPPER.to_owned()];

    assert_eq!(
        names(&single),
        [(UPPER.to_owned(), "Built-in screen".to_owned(), Some(Normal))]
    );
}

#[test]
fn more_built_in_screens_are_numbered_and_a_mirrored_one_has_no_orientation() {
    let mut three = state(
        &[
            (UPPER, Some(PANEL)),
            (LOWER, Some(PANEL)),
            ("eDP-3", Some(PANEL)),
        ],
        vec![
            panel(UPPER, 0, 0, false),
            panel(LOWER, 0, 1440, true),
            panel("eDP-3", 0, 2880, false),
        ],
    );

    three.builtin.push("eDP-3".to_owned());
    three.logical[2].transform = 4;
    assert_eq!(
        names(&three)
            .into_iter()
            .map(|(_, name, orientation)| (name, orientation))
            .collect::<Vec<_>>(),
        [
            ("Built-in screen 1".to_owned(), Some(Normal)),
            ("Built-in screen 2".to_owned(), Some(Normal)),
            ("Built-in screen 3".to_owned(), None),
        ]
    );
}

#[test]
fn the_lower_screen_turned_upside_down_leaves_the_upper_one_and_stays_below() {
    let layout = turn_one(&two_panels(), LOWER, BottomUp).expect("a layout");

    assert_eq!(place(&layout, UPPER), (0, 0, 0));
    assert_eq!(place(&layout, LOWER), (0, 1440, 2));
}

#[test]
fn a_monitor_that_does_not_touch_the_screens_is_left_where_it_is_by_the_ways_that_slide() {
    // Below the two screens, and right of them, with a gap: the ways that slide
    // along the new edge leave such a monitor alone. No way is valid then, but
    // the one with each monitor in a row.
    for (x, y) in [(0, 3300), (2400, 0)] {
        let layout = turn_one(&beside(x, y), LOWER, BottomUp).expect("a layout");

        assert_eq!(place(&layout, LOWER).2, 2);
        assert_eq!(place(&layout, UPPER).2, 0);
    }
}

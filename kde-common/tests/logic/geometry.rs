//! The geometry of layouts: places, sizes, validity and shifts.

use asus_ux8406_kde_common::geometry::{
    Area, DOWN, Monitor, Size, area_of, find, is_rest, is_valid, shifted, size_of, to_origin,
};
use rstest::rstest;

use crate::fixtures::{LOWER, PANEL, UPPER, at, lower_off, panel, size, state, two_panels};

fn sized(transform: u32, scale: f64) -> Monitor {
    Monitor {
        transform,
        ..crate::fixtures::logical(UPPER, 0, 0, false, scale)
    }
}

#[rstest]
#[case(0, 1.25, true, (2304, 1440))]
#[case(1, 1.25, true, (1440, 2304))]
#[case(2, 1.25, true, (2304, 1440))]
#[case(3, 1.25, true, (1440, 2304))]
#[case(4, 1.25, true, (2304, 1440))]
#[case(5, 1.25, true, (1440, 2304))]
#[case(0, 1.0, false, (2880, 1800))]
#[case(1, 1.0, false, (1800, 2880))]
fn a_monitor_takes_the_size_of_its_mode_turned_and_scaled(
    #[case] transform: u32,
    #[case] scale: f64,
    #[case] logical_mode: bool,
    #[case] expect: (i32, i32),
) {
    let size = size_of(&sized(transform, scale), PANEL, logical_mode);

    assert_eq!((size.width, size.height), expect);
}

#[rstest]
#[case(1.5, 1, 1, true, (1, 1))]
#[case(2.0, 3, 5, true, (2, 3))]
#[case(2.0, 5, 3, true, (3, 2))]
#[case(3.0, 2, 4, true, (1, 1))]
#[case(2.0, 5, 3, false, (5, 3))]
fn the_size_in_the_layout_is_rounded_half_up_as_javascript_does(
    #[case] scale: f64,
    #[case] width: i32,
    #[case] height: i32,
    #[case] logical_mode: bool,
    #[case] expect: (i32, i32),
) {
    let size = size_of(&sized(0, scale), Size { width, height }, logical_mode);

    assert_eq!((size.width, size.height), expect);
}

#[test]
fn the_area_of_a_monitor_has_its_place_and_its_size() {
    let state = two_panels();

    assert_eq!(
        area_of(&state, &state.logical[1]),
        Area {
            x: 0,
            y: 1440,
            width: 2304,
            height: 1440
        }
    );
}

#[test]
fn a_monitor_without_a_mode_takes_no_room() {
    let mut state = two_panels();

    state.connected.insert(LOWER.to_owned(), None);
    assert_eq!(area_of(&state, &state.logical[1]).width, 0);
    assert_eq!(area_of(&state, &state.logical[1]).height, 0);
}

#[test]
fn a_monitor_that_is_not_connected_takes_no_room() {
    let mut state = two_panels();

    state.connected.remove(LOWER);
    assert_eq!(area_of(&state, &state.logical[1]).width, 0);
}

#[test]
fn a_monitor_without_a_connector_takes_no_room() {
    let state = two_panels();
    let nothing = Monitor {
        connectors: vec![],
        ..panel(UPPER, 5, 7, false)
    };

    assert_eq!(
        area_of(&state, &nothing),
        Area {
            x: 5,
            y: 7,
            width: 0,
            height: 0
        }
    );
}

#[test]
fn the_monitor_that_shows_a_connector_is_found() {
    let state = two_panels();

    assert_eq!(find(&state.logical, LOWER), Some(&state.logical[1]));
    assert_eq!(find(&state.logical, "DP-7"), None);
}

#[test]
fn a_mirror_is_found_by_each_connector() {
    let mut layout = two_panels().logical;

    layout[0].connectors.push("DP-7".to_owned());
    assert_eq!(find(&layout, "DP-7"), Some(&layout[0]));
}

#[test]
fn the_layout_moves_to_the_origin() {
    let layout = vec![panel(UPPER, -100, 50, false), panel(LOWER, 20, 70, true)];
    let moved = to_origin(&layout);

    assert_eq!((moved[0].x, moved[0].y), (0, 0));
    assert_eq!((moved[1].x, moved[1].y), (120, 20));
    assert_eq!(moved[1].connectors, layout[1].connectors);
}

#[test]
fn an_empty_layout_stays_empty() {
    assert_eq!(to_origin(&[]), []);
}

#[test]
fn down_is_down_for_an_upright_panel_and_turns_with_it() {
    assert_eq!(DOWN, [(0, 1), (-1, 0), (0, -1), (1, 0)]);
}

fn with_lower_at(x: i32, y: i32) -> bool {
    let mut state = two_panels();

    let lower = at(&mut state.logical, LOWER);
    (lower.x, lower.y) = (x, y);
    is_valid(&state, &state.logical)
}

#[rstest]
#[case::below(0, 1440, true)]
#[case::beside(2304, 0, true)]
#[case::beside_shifted(2304, 700, true)]
#[case::beside_shifted_up(2304, -700, true)]
#[case::over(0, 100, false)]
#[case::over_a_little(2303, 0, false)]
#[case::gap_below(0, 1441, false)]
#[case::gap_beside(2305, 0, false)]
#[case::corner(2304, 1440, false)]
#[case::far(0, 2000, false)]
fn a_layout_is_valid_when_no_monitor_lies_over_another_and_all_touch(
    #[case] x: i32,
    #[case] y: i32,
    #[case] valid: bool,
) {
    assert_eq!(with_lower_at(x, y), valid);
}

#[test]
fn one_monitor_is_valid_and_an_empty_layout_is_not() {
    let state = lower_off();

    assert!(is_valid(&state, &state.logical));
    assert!(!is_valid(&state, &[]));
}

#[test]
fn monitors_that_touch_through_a_third_are_valid() {
    let state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(UPPER, 0, 0, true),
            panel("DP-7", 0, 2880, false),
            panel(LOWER, 0, 1440, false),
        ],
    );

    assert!(is_valid(&state, &state.logical));
}

#[test]
fn the_layout_in_panel_pixels_is_valid_when_the_edges_meet() {
    let mut state = two_panels();

    state.logical_mode = false;
    assert!(!is_valid(&state, &state.logical));
    at(&mut state.logical, LOWER).y = 1800;
    assert!(is_valid(&state, &state.logical));
}

/// Monitors of the size 100 by 100 around a hole of 100 by 100 at (100, 100):
/// the layout, and the area of the hole.
fn around_a_hole() -> (asus_ux8406_kde_common::geometry::State, Area) {
    let mut state = state(
        &[("A", Some(size(100, 100)))],
        ["N", "S", "W", "E", "NE", "SW"]
            .iter()
            .map(|name| crate::fixtures::logical(name, 0, 0, false, 1.0))
            .collect(),
    );
    let places = [
        (100, 0),
        (100, 200),
        (0, 100),
        (200, 100),
        (200, 0),
        (0, 200),
    ];

    for (monitor, (x, y)) in state.logical.iter_mut().zip(places) {
        (monitor.x, monitor.y) = (x, y);
        state
            .connected
            .insert(monitor.connectors[0].clone(), Some(size(100, 100)));
    }
    state.logical_mode = false;
    (
        state,
        Area {
            x: 100,
            y: 100,
            width: 100,
            height: 100,
        },
    )
}

fn names_moved(direction: (i32, i32), steps: i32) -> Vec<(String, i32, i32)> {
    let (state, hole) = around_a_hole();
    let moved = shifted(&state, &state.logical, hole, direction, steps);

    state
        .logical
        .iter()
        .zip(moved)
        .filter(|(before, after)| (before.x, before.y) != (after.x, after.y))
        .map(|(before, after)| {
            (
                before.connectors[0].clone(),
                after.x - before.x,
                after.y - before.y,
            )
        })
        .collect()
}

#[rstest]
#[case::down((0, 1), 1, ("S", 0, 100))]
#[case::up((0, -1), 1, ("N", 0, -100))]
#[case::right((1, 0), 1, ("E", 100, 0))]
#[case::left((-1, 0), 1, ("W", -100, 0))]
#[case::close_down((0, 1), -1, ("S", 0, -100))]
#[case::close_right((1, 0), -1, ("E", -100, 0))]
fn only_the_monitor_in_line_beyond_the_hole_moves_by_its_size(
    #[case] direction: (i32, i32),
    #[case] steps: i32,
    #[case] moved: (&str, i32, i32),
) {
    // The monitors at the corners share no row or column with the hole.
    let (name, dx, dy) = moved;

    assert_eq!(
        names_moved(direction, steps),
        vec![(name.to_owned(), dx, dy)]
    );
}

#[test]
fn a_hole_far_away_moves_nothing() {
    let (state, _) = around_a_hole();
    let far = Area {
        x: 5000,
        y: 5000,
        width: 100,
        height: 100,
    };

    assert_eq!(
        shifted(&state, &state.logical, far, (0, 1), 1),
        state.logical
    );
}

#[test]
fn the_monitors_that_are_not_moved_are_copied() {
    let (state, hole) = around_a_hole();
    let moved = shifted(&state, &state.logical, hole, (0, 1), 0);

    assert_eq!(moved, state.logical);
}

#[test]
fn a_layout_is_the_rest_of_another_with_the_same_connectors() {
    let both = two_panels().logical;
    let upper = lower_off().logical;

    assert!(is_rest(&both, &both, None));
    assert!(is_rest(&upper, &both, Some(LOWER)));
    assert!(!is_rest(&upper, &both, None));
    assert!(!is_rest(&both, &upper, None));
    assert!(is_rest(&[], &[], None));
}

#[test]
fn the_order_of_the_connectors_does_not_matter() {
    let both = two_panels().logical;
    let reversed = vec![both[1].clone(), both[0].clone()];

    assert!(is_rest(&both, &reversed, None));
}

#[test]
fn other_connectors_are_not_the_rest() {
    let both = two_panels().logical;
    let mut other = both.clone();

    other[1].connectors = vec!["DP-7".to_owned()];
    assert!(!is_rest(&both, &other, None));
    assert!(is_rest(&both, &both, Some("DP-9")));
    assert!(!is_rest(&both, &upper_only(), Some("DP-9")));
}

fn upper_only() -> Vec<Monitor> {
    lower_off().logical
}

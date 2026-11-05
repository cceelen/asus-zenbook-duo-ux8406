//! What the key for the lower panel does with the layout.

use asus_ux8406_kde_common::geometry::{Monitor, State, find, is_valid};
use asus_ux8406_kde_common::layout::{Toggle, toggle, with_below, without};
use rstest::rstest;

use crate::fixtures::{
    LOWER, UPPER, at, docked, logical, lower_off, panel, place, size, state, two_panels,
};

fn x_of(layout: &[Monitor], connector: &str) -> i32 {
    find(layout, connector).expect("in the layout").x
}

/// The laptop on its side: the panels lie side by side, and a monitor lies
/// beside them.
fn side_by_side(transform: u32, lower_x: i32, upper_x: i32, monitor_x: i32) -> State {
    let mut state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(UPPER, upper_x, 0, true),
            panel(LOWER, lower_x, 0, false),
            logical("DP-7", monitor_x, 0, false, 1.0),
        ],
    );

    at(&mut state.logical, UPPER).transform = transform;
    at(&mut state.logical, LOWER).transform = transform;
    state
}

#[test]
fn taking_the_lower_panel_out_leaves_the_upper_one_as_the_primary() {
    let rest = without(&two_panels(), LOWER, UPPER).expect("a layout");

    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].connectors, [UPPER]);
    assert!(rest[0].primary);
    assert_eq!(rest[0].y, 0);
}

#[test]
fn taking_the_upper_panel_out_gives_the_primary_to_the_heir() {
    let mut state = two_panels();

    state.logical[0].primary = true;
    state.logical[1].primary = false;

    let rest = without(&state, UPPER, LOWER).expect("a layout");

    assert!(rest[0].primary);
    assert_eq!(rest[0].connectors, [LOWER]);
}

#[test]
fn an_heir_that_is_not_in_the_layout_gives_the_primary_to_the_first() {
    let mut state = two_panels();

    state.logical[0].primary = true;
    state.logical[1].primary = false;

    let rest = without(&state, UPPER, "DP-9").expect("a layout");

    assert!(rest[0].primary);
}

#[test]
fn a_monitor_that_is_not_the_primary_leaves_the_primary_alone() {
    let rest = without(&two_panels(), UPPER, LOWER).expect("a layout");

    assert!(rest[0].primary);
}

#[test]
fn what_is_left_moves_to_the_origin() {
    let state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
        ],
        vec![panel(LOWER, 0, 0, true), panel(UPPER, 0, 1440, false)],
    );

    assert_eq!(without(&state, LOWER, UPPER).expect("a layout")[0].y, 0);
}

#[test]
fn the_only_monitor_a_monitor_not_shown_and_a_mirrored_one_stay() {
    let mut mirrored = two_panels();

    mirrored.logical = vec![
        Monitor {
            connectors: vec![UPPER.to_owned(), LOWER.to_owned()],
            ..panel(UPPER, 0, 0, true)
        },
        logical("DP-7", 2880, 0, false, 1.0),
    ];
    assert_eq!(without(&lower_off(), UPPER, UPPER), None);
    assert_eq!(without(&lower_off(), LOWER, UPPER), None);
    assert_eq!(without(&mirrored, LOWER, UPPER), None);
}

#[rstest]
#[case::right_side_up(3, 1440, 0, 2880, "DP-7", 1440)]
#[case::left_side_up(1, 2880, 4320, 0, UPPER, 2880)]
fn a_monitor_beyond_the_lower_panel_comes_nearer_when_the_panel_goes(
    #[case] transform: u32,
    #[case] lower_x: i32,
    #[case] upper_x: i32,
    #[case] monitor_x: i32,
    #[case] moved: &str,
    #[case] to: i32,
) {
    let state = side_by_side(transform, lower_x, upper_x, monitor_x);
    let rest = without(&state, LOWER, UPPER).expect("a layout");

    assert!(is_valid(&state, &state.logical));
    assert_eq!(rest.len(), 2);
    assert!(is_valid(&state, &rest));
    assert_eq!(x_of(&rest, moved), to);
}

#[test]
fn a_monitor_below_the_lower_panel_comes_up_when_the_panel_goes() {
    let state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(UPPER, 0, 0, true),
            panel(LOWER, 0, 1440, false),
            logical("DP-7", 0, 2880, false, 1.0),
        ],
    );
    let rest = without(&state, LOWER, UPPER).expect("a layout");

    assert_eq!(find(&rest, "DP-7").expect("in the layout").y, 1440);
}

#[test]
fn when_no_shift_closes_the_hole_the_rest_stays_as_it_is() {
    // The monitor far away touches nothing, whatever way is shifted.
    let state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(1000, 500))),
        ],
        vec![
            panel(UPPER, 0, 0, true),
            panel(LOWER, 0, 1440, false),
            logical("DP-7", 10000, 0, false, 1.0),
        ],
    );
    let rest = without(&state, LOWER, UPPER).expect("a layout");

    assert!(!is_valid(&state, &rest));
    assert_eq!(rest.len(), 2);
    assert_eq!(rest[1].x, 10000);
}

#[test]
fn the_lower_panel_goes_below_the_upper_one_at_its_scale() {
    let layout = with_below(&lower_off(), LOWER, UPPER).expect("a layout");

    assert_eq!(layout.len(), 2);
    assert_eq!(
        (layout[1].x, layout[1].y, layout[1].scale, layout[1].primary),
        (0, 1440, 1.25, false)
    );
}

#[test]
fn in_panel_pixels_the_lower_panel_is_1800_below() {
    let mut state = lower_off();

    state.logical_mode = false;
    assert_eq!(
        with_below(&state, LOWER, UPPER).expect("a layout")[1].y,
        1800
    );
}

#[rstest]
#[case(1, (-1440, 0))]
#[case(2, (0, -1440))]
#[case(3, (1440, 0))]
fn with_the_upper_panel_turned_the_lower_one_goes_where_the_bottom_is(
    #[case] transform: u32,
    #[case] offset: (i32, i32),
) {
    let mut state = lower_off();

    state.logical[0].transform = transform;

    let layout = with_below(&state, LOWER, UPPER).expect("a layout");
    let (upper, lower) = (
        find(&layout, UPPER).expect("upper"),
        find(&layout, LOWER).expect("lower"),
    );

    assert!(is_valid(&state, &layout));
    assert_eq!(lower.transform, transform);
    assert_eq!((lower.x - upper.x, lower.y - upper.y), offset);
}

#[rstest]
#[case::right_side_up(3, 0, 1440, [0, 1440, 2880])]
#[case::left_side_up(1, 2880, 0, [4320, 2880, 0])]
fn a_monitor_where_the_lower_panel_goes_makes_room(
    #[case] transform: u32,
    #[case] upper_x: i32,
    #[case] monitor_x: i32,
    #[case] xs: [i32; 3],
) {
    let mut state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(2880, 1800))),
        ],
        vec![
            panel(UPPER, upper_x, 0, true),
            logical("DP-7", monitor_x, 0, false, 1.0),
        ],
    );

    state.logical[0].transform = transform;

    let layout = with_below(&state, LOWER, UPPER).expect("a layout");

    assert!(is_valid(&state, &layout));
    assert_eq!([UPPER, LOWER, "DP-7"].map(|name| x_of(&layout, name)), xs);
}

#[test]
fn a_monitor_that_is_not_in_the_way_of_the_lower_panel_stays() {
    let mut state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(1440, 2304))),
            ("DP-8", Some(size(1000, 500))),
        ],
        vec![
            panel(UPPER, 0, 0, true),
            logical("DP-7", 1440, 0, false, 1.0),
            logical("DP-8", 1440, 2304, false, 1.0),
        ],
    );

    state.logical[0].transform = 3;

    let layout = with_below(&state, LOWER, UPPER).expect("a layout");

    assert!(is_valid(&state, &layout));
    assert_eq!(x_of(&layout, "DP-7"), 2880);
    assert_eq!(x_of(&layout, "DP-8"), 1440);
}

#[test]
fn no_valid_way_gives_the_layout_with_the_panel_added_as_it_is() {
    // The lower panel goes right of the turned upper one, where a monitor of
    // another height lies that touches nothing else after a shift.
    let mut state = state(
        &[
            (UPPER, Some(size(2880, 1800))),
            (LOWER, Some(size(2880, 1800))),
            ("DP-7", Some(size(100, 100))),
        ],
        vec![
            panel(UPPER, 0, 0, true),
            logical("DP-7", 5000, 5000, false, 1.0),
        ],
    );

    state.logical[0].transform = 3;

    let layout = with_below(&state, LOWER, UPPER).expect("a layout");

    assert!(!is_valid(&state, &layout));
    assert_eq!(layout.len(), 3);
}

#[test]
fn the_lower_panel_is_not_put_below_what_cannot_hold_it() {
    let mut mirrored = lower_off();

    mirrored.logical[0].connectors.push("DP-7".to_owned());
    assert_eq!(with_below(&mirrored, LOWER, UPPER), None);
    assert_eq!(with_below(&lower_off(), LOWER, "DP-9"), None);
    assert_eq!(with_below(&lower_off(), "DP-9", UPPER), None);
}

#[test]
fn an_anchor_without_a_mode_gives_nothing() {
    let mut state = lower_off();

    state.connected.insert(UPPER.to_owned(), None);
    assert_eq!(with_below(&state, LOWER, UPPER), None);
}

#[test]
fn pressing_the_key_with_both_panels_on_hides_the_lower_one_and_remembers() {
    let state = two_panels();
    let Toggle { apply, save } = toggle(&state, None, LOWER, UPPER);

    assert_eq!(apply.expect("a layout").len(), 1);
    assert_eq!(save, Some(state.logical));
}

#[test]
fn nothing_to_hide_keeps_what_was_saved() {
    let mut state = two_panels();
    let saved = lower_off().logical;

    state.logical[1].connectors.push("DP-7".to_owned());

    let result = toggle(&state, Some(&saved), LOWER, UPPER);

    assert_eq!(result.apply, None);
    assert_eq!(result.save, Some(saved));
    assert_eq!(toggle(&state, None, LOWER, UPPER).save, None);
}

#[test]
fn pressing_it_again_puts_the_remembered_layout_back() {
    let before = two_panels().logical;
    let result = toggle(&lower_off(), Some(&before), LOWER, UPPER);

    assert_eq!(result.apply, Some(before));
    assert_eq!(result.save, None);
}

#[test]
fn after_a_turn_of_the_laptop_the_remembered_layout_is_not_put_back() {
    let before = two_panels().logical;
    let mut state = lower_off();

    state.logical[0].transform = 1;

    let apply = toggle(&state, Some(&before), LOWER, UPPER)
        .apply
        .expect("a layout");

    assert_ne!(apply, before);
    assert!(is_valid(&state, &apply));
    assert_eq!(place(&apply, LOWER).2, 1);
}

#[test]
fn without_a_remembered_layout_the_panel_goes_below_the_upper_one() {
    let apply = toggle(&lower_off(), None, LOWER, UPPER)
        .apply
        .expect("a layout");

    assert_eq!(apply.len(), 2);
    assert_eq!(apply[1].y, 1440);
}

#[test]
fn a_remembered_layout_for_other_monitors_is_not_put_back() {
    let other = vec![
        logical("DP-7", 0, 0, true, 1.0),
        logical(LOWER, 2560, 0, false, 1.0),
    ];
    let apply = toggle(&lower_off(), Some(&other), LOWER, UPPER)
        .apply
        .expect("a layout");

    assert_eq!(apply[1].y, 1440);
}

#[test]
fn layouts_without_the_upper_panel_are_turned_the_same() {
    // Both layouts lack the upper panel: neither has a turn to compare.
    let mut state = state(
        &[
            ("DP-7", Some(size(1920, 1080))),
            (LOWER, Some(size(2880, 1800))),
        ],
        vec![logical("DP-7", 0, 0, true, 1.0)],
    );
    let saved = state.logical.clone();

    state.builtin.clear();

    let result = toggle(&state, Some(&saved), LOWER, "DP-9");

    assert_eq!(result.apply, Some(saved));
    assert_eq!(result.save, None);
}

#[test]
fn with_the_keyboard_on_the_panel_the_key_does_nothing() {
    let saved = two_panels().logical;
    let result = toggle(&docked(), Some(&saved), LOWER, UPPER);

    assert_eq!(result.apply, None);
    assert_eq!(result.save, Some(saved));
    assert_eq!(toggle(&docked(), None, LOWER, UPPER).save, None);
}

//! The screens follow the laptop.

use crate::{LOWER, Session, UPPER, panels};

#[test]
fn left_side_up_the_lower_panel_is_on_the_left() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("left-up");
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
}

#[test]
fn right_side_up_the_lower_panel_is_on_the_right_and_upright_it_comes_back() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("right-up");
    session.wait_places(&program, (0, 0, 3), (1440, 0, 3));
    session.sensor().turn("normal");
    session.wait_places(&program, (0, 0, 0), (0, 1440, 0));
}

#[test]
fn a_saved_turned_layout_is_put_upright_at_the_start() {
    // KWin keeps the last layout: the session starts turned, the laptop
    // stands upright.
    let mut outputs = panels();

    outputs[0].transform = 1;
    outputs[0].x = 1440;
    outputs[1].transform = 1;
    outputs[1].y = 0;

    let session = Session::new(outputs, Some("normal"));
    let program = session.start();

    session.wait_places(&program, (0, 0, 0), (0, 1440, 0));
}

#[test]
fn upside_down_the_lower_panel_is_on_top_and_upright_it_comes_back() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("bottom-up");
    session.wait_places(&program, (0, 1440, 2), (0, 0, 2));
    session.sensor().turn("normal");
    session.wait_places(&program, (0, 0, 0), (0, 1440, 0));
}

#[test]
fn from_a_side_to_upside_down() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("left-up");
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    session.sensor().turn("bottom-up");
    session.wait_places(&program, (0, 1440, 2), (0, 0, 2));
}

#[test]
fn upside_down_with_one_panel_turns_that_one() {
    let session = Session::new(vec![panels().remove(0)], Some("normal"));
    let program = session.start();

    session.sensor().turn("bottom-up");
    program.wait_for("the panel turned", || session.place(UPPER) == (0, 0, 2));
}

#[test]
fn an_undefined_orientation_changes_nothing() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("undefined");
    std::thread::sleep(crate::turns::SETTLED);
    assert!(!program.logged("turned to"));
    assert_eq!(session.place(LOWER), (0, 1440, 0));
}

#[test]
fn a_layout_that_kwin_refuses_stays_as_it_was() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    // The program sets KWin's policies at the start, in a configuration of
    // its own: the refusal is for the layout, after it.
    program.wait_for("never", || session.compositor.output(LOWER).policy == 0);
    session.compositor.refuse_next("no");
    session.sensor().turn("left-up");
    program.wait_log("KWin did not take the layout: no");
    assert_eq!(session.place(UPPER), (0, 0, 0));
}

#[test]
fn the_screens_wait_until_the_orientation_has_settled() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("left-up");
    std::thread::sleep(std::time::Duration::from_millis(250));
    assert_eq!(session.place(LOWER), (0, 1440, 0));
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
}

/// Longer than the program waits for the orientation to settle.
pub const SETTLED: std::time::Duration = std::time::Duration::from_millis(1200);

//! Which brightness the lower output gets.

use asus_ux8406_kde_common::brightness::{Panel, follow};

fn on(level: u32) -> Panel {
    Panel {
        enabled: true,
        level: Some(level),
    }
}

#[test]
fn a_lower_output_that_differs_gets_the_brightness_of_the_upper_one() {
    assert_eq!(follow(Some(on(3000)), Some(on(10000))), Some(3000));
    assert_eq!(follow(Some(on(10000)), Some(on(0))), Some(10000));
}

#[test]
fn equal_brightness_is_left_alone() {
    assert_eq!(follow(Some(on(3000)), Some(on(3000))), None);
}

#[test]
fn a_missing_output_is_left_alone() {
    assert_eq!(follow(None, Some(on(3000))), None);
    assert_eq!(follow(Some(on(3000)), None), None);
}

#[test]
fn an_output_that_is_off_is_left_alone() {
    let off = Some(Panel {
        enabled: false,
        level: Some(1000),
    });

    assert_eq!(follow(off, Some(on(3000))), None);
    assert_eq!(follow(Some(on(3000)), off), None);
}

#[test]
fn an_output_without_a_brightness_is_left_alone() {
    let without = Some(Panel {
        enabled: true,
        level: None,
    });

    assert_eq!(follow(without, Some(on(3000))), None);
    assert_eq!(follow(Some(on(3000)), without), None);
}

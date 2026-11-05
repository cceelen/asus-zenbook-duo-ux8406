//! Each touchscreen and pen gets its panel.

use asus_ux8406_kde_fakes::Output;
use asus_ux8406_kde_fakes::input::Device;

use crate::{LOWER, Session, UPPER, panels, touch};

#[test]
fn each_touchscreen_and_pen_gets_its_panel() {
    let session = Session::new(panels(), Some(touch()));
    let _program = session.start();
    let input = session.input();

    assert_eq!(input.output_name("event10"), UPPER);
    assert_eq!(input.output_name("event11"), UPPER);
    assert_eq!(input.output_name("event12"), LOWER);
    assert_eq!(input.output_name("event13"), LOWER);
}

#[test]
fn a_device_that_acts_on_an_output_keeps_it() {
    let devices = vec![Device {
        output_name: "DP-1".to_owned(),
        ..Device::touchscreen("event10", 0x4447)
    }];
    let session = Session::new(panels(), Some(devices));
    let _program = session.start();

    assert_eq!(session.input().output_name("event10"), "DP-1");
}

#[test]
fn the_lower_touchscreen_waits_for_its_panel() {
    let session = Session::new(vec![Output::panel(UPPER, 0, 0)], Some(touch()));
    let program = session.start();

    assert_eq!(session.input().output_name("event12"), "");
    // The keyboard is taken off the lower panel.
    session.compositor.plug(Output::panel(LOWER, 0, 1440));
    program.wait_for("the lower touchscreen on its panel", || {
        session.input().output_name("event12") == LOWER
    });
}

#[test]
fn a_new_device_gets_its_panel() {
    let session = Session::new(panels(), Some(Vec::new()));
    let program = session.start();

    session.input().add(Device::pen("event20", 0x4448));
    program.wait_for("the pen on its panel", || {
        session.input().output_name("event20") == LOWER
    });
}

#[test]
fn other_devices_and_devices_that_fail_are_left() {
    let devices = vec![
        // A touchpad, and a touchscreen of another controller.
        Device {
            touch: false,
            ..Device::touchscreen("event1", 0x4447)
        },
        Device::touchscreen("event2", 0x1234),
        // A device that does not take an output, and one whose object is
        // not there.
        Device {
            locked: true,
            ..Device::touchscreen("event3", 0x4448)
        },
        Device::touchscreen("event-4", 0x4448),
    ];
    let session = Session::new(panels(), Some(devices));
    let program = session.start();

    assert_eq!(session.input().output_name("event1"), "");
    assert_eq!(session.input().output_name("event2"), "");
    assert!(program.logged("input device event3: "));
    assert!(program.logged("input device event-4: "));
}

#[test]
fn without_kwin_input_devices_the_key_still_operates() {
    let session = Session::new(panels(), None);
    let program = session.start();

    assert!(program.logged("KWin's input devices: "));
    session.press();
    session
        .compositor
        .wait_for("the lower panel off", |c| !c.output(LOWER).enabled);
}

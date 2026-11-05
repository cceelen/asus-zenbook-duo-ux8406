//! Which panel each touch controller belongs to.

use asus_ux8406_kde_common::touch::{Assignment, Controller, assignments, controllers_of};
use rstest::rstest;

use crate::fixtures::{LOWER, UPPER, docked, lower_off, size, state, two_panels};

/// A list of i2c devices from names.
fn devices(list: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    list.iter()
        .map(|(name, children)| {
            (
                (*name).to_owned(),
                children.iter().map(|child| (*child).to_owned()).collect(),
            )
        })
        .collect()
}

/// What /sys/bus/i2c/devices holds on the UX8406CA, in part.
fn machine() -> Vec<(String, Vec<String>)> {
    devices(&[
        ("i2c-0", &["name", "subsystem"]),
        (
            "i2c-ELAN9008:00",
            &["0018:04F3:4447.0006", "driver", "modalias", "power"],
        ),
        (
            "i2c-ELAN9009:00",
            &["0018:04F3:4448.0005", "driver", "modalias", "power"],
        ),
        ("i2c-ASUE1234:00", &["0018:0B05:1234.0001", "driver"]),
    ])
}

fn controller(name: &str, id: &str) -> Controller {
    Controller {
        controller: name.to_owned(),
        id: id.to_owned(),
    }
}

fn assignment(id: &str, connector: &str) -> Assignment {
    Assignment {
        id: id.to_owned(),
        connector: connector.to_owned(),
    }
}

#[test]
fn the_two_touch_controllers_are_found_with_their_ids_in_small_letters() {
    assert_eq!(
        controllers_of(&machine()),
        [
            controller("ELAN9008", "04f3:4447"),
            controller("ELAN9009", "04f3:4448")
        ]
    );
}

#[test]
fn the_ids_are_read_from_the_machine_not_written_down() {
    let other = devices(&[
        ("i2c-ELAN9008:00", &["0018:04F3:425B.0007"]),
        ("i2c-ELAN9009:00", &["0018:04F3:425A.0008"]),
    ]);

    assert_eq!(
        controllers_of(&other)
            .iter()
            .map(|found| found.id.as_str())
            .collect::<Vec<_>>(),
        ["04f3:425b", "04f3:425a"]
    );
}

#[test]
fn a_controller_without_a_hid_device_or_with_two_is_left_out() {
    assert_eq!(
        controllers_of(&devices(&[("i2c-ELAN9008:00", &["driver"])])),
        []
    );
    assert_eq!(
        controllers_of(&devices(&[(
            "i2c-ELAN9009:00",
            &["0018:04F3:4448.0005", "0018:04F3:4449.0006"]
        )])),
        []
    );
}

#[test]
fn a_machine_without_these_controllers_has_none() {
    assert_eq!(
        controllers_of(&devices(&[("i2c-SYNA0001:00", &["0018:06CB:0001.0001"])])),
        []
    );
    assert_eq!(controllers_of(&[]), []);
}

#[rstest]
#[case::lower_case_hex("0018:04f3:44ab.0006", true)]
#[case::long_instance("0018:04F3:4447.0006ABCDEF", true)]
#[case::short_instance("0018:04F3:4447.0", true)]
#[case::no_instance("0018:04F3:4447.", false)]
#[case::no_dot("0018:04F3:4447", false)]
#[case::colon_for_dot("0018:04F3:4447:0006", false)]
#[case::dash("0018-04F3-4447.0006", false)]
#[case::dash_in_the_middle("0018:04F3-4447.0006", false)]
#[case::short_bus("018:04F3:4447.0006", false)]
#[case::short_product("0018:04F3:447.0006", false)]
#[case::long_product("0018:04F3:44470.0006", false)]
#[case::not_hex_instance("0018:04F3:4447.000G", false)]
#[case::not_hex_bus("00G8:04F3:4447.0006", false)]
#[case::newline("0018:04F3:4447.0006\n", false)]
#[case::space(" 0018:04F3:4447.0006", false)]
#[case::second_dot("0018:04F3:4447.00.06", false)]
#[case::not_ascii("0018:04F3:444\u{e9}.0006", false)]
#[case::empty("", false)]
fn a_hid_device_name_is_read_by_its_shape(#[case] child: &str, #[case] found: bool) {
    let found_ids = controllers_of(&devices(&[("i2c-ELAN9008:00", &[child])]));

    assert_eq!(found_ids.len(), usize::from(found), "{child:?}");
}

#[rstest]
#[case::small_letters("i2c-elan9008:00")]
#[case::no_colon("i2c-ELAN9008")]
#[case::no_prefix("ELAN9008:00")]
#[case::before("i2c-x-ELAN9008:00")]
#[case::another_controller("i2c-ELAN9010:00")]
#[case::empty_name("i2c-:00")]
#[case::nothing("")]
fn a_name_that_is_not_a_known_controller_is_left_out(#[case] name: &str) {
    assert_eq!(
        controllers_of(&devices(&[(name, &["0018:04F3:4447.0006"])])),
        []
    );
}

#[test]
fn more_after_the_colon_does_not_matter() {
    assert_eq!(
        controllers_of(&devices(&[(
            "i2c-ELAN9009:00:more",
            &["driver", "0018:04F3:4448.0005"]
        )])),
        [controller("ELAN9009", "04f3:4448")]
    );
}

#[test]
fn each_controller_gets_its_panel_named_with_the_connector() {
    assert_eq!(
        assignments(&controllers_of(&machine()), &two_panels(), UPPER, LOWER),
        [
            assignment("04f3:4447", UPPER),
            assignment("04f3:4448", LOWER)
        ]
    );
}

#[test]
fn with_the_keyboard_on_the_lower_panel_only_the_upper_one_is_assigned() {
    assert_eq!(
        assignments(&controllers_of(&machine()), &docked(), UPPER, LOWER),
        [assignment("04f3:4447", UPPER)]
    );
}

#[test]
fn a_panel_that_is_connected_but_switched_off_is_assigned_too() {
    assert_eq!(
        assignments(&controllers_of(&machine()), &lower_off(), UPPER, LOWER).len(),
        2
    );
}

#[test]
fn a_panel_without_a_mode_is_known() {
    let state = state(&[(UPPER, None), (LOWER, Some(size(2880, 1800)))], vec![]);

    assert_eq!(
        assignments(&controllers_of(&machine()), &state, UPPER, LOWER).len(),
        2
    );
}

#[test]
fn the_panels_are_named_by_the_caller() {
    let state = state(&[("DP-1", None), ("DP-2", None)], vec![]);

    assert_eq!(
        assignments(&controllers_of(&machine()), &state, "DP-2", "DP-1"),
        [
            assignment("04f3:4447", "DP-2"),
            assignment("04f3:4448", "DP-1")
        ]
    );
}

#[test]
fn a_controller_that_is_not_known_has_no_panel() {
    let other = [controller("ELAN1234", "04f3:1234")];

    assert_eq!(assignments(&other, &two_panels(), UPPER, LOWER), []);
}

#[test]
fn no_controllers_have_no_assignments() {
    assert_eq!(assignments(&[], &two_panels(), UPPER, LOWER), []);
}

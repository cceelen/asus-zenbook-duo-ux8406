//! Which input devices get which panel.

use std::collections::HashMap;

use asus_ux8406_kde_common::touch::Assignment;
use asus_ux8406_kde_keys::input::{Device, plan};
use zbus::zvariant::{OwnedValue, Value};

fn device(sys_name: &str, product: u32, touch: bool, output_name: &str) -> Device {
    Device {
        sys_name: sys_name.to_owned(),
        vendor: 0x04f3,
        product,
        touch,
        output_name: output_name.to_owned(),
    }
}

fn panels() -> Vec<Assignment> {
    [("04f3:4447", "eDP-1"), ("04f3:4448", "eDP-2")]
        .map(|(id, connector)| Assignment {
            id: id.to_owned(),
            connector: connector.to_owned(),
        })
        .into()
}

#[test]
fn the_id_is_that_of_the_controllers_in_small_letters() {
    assert_eq!(device("event1", 0x425b, true, "").id(), "04f3:425b");
}

#[test]
fn each_touch_device_without_an_output_gets_its_panel() {
    let devices = [
        device("event10", 0x4447, true, ""),
        device("event12", 0x4448, true, ""),
    ];

    assert_eq!(
        plan(&devices, &panels()),
        [
            ("event10".to_owned(), "eDP-1".to_owned()),
            ("event12".to_owned(), "eDP-2".to_owned()),
        ]
    );
}

#[test]
fn a_device_with_an_output_another_device_and_no_panel_get_nothing() {
    let devices = [
        device("event10", 0x4447, true, "DP-1"),
        device("event1", 0x4447, false, ""),
        device("event2", 0x1234, true, ""),
    ];

    assert_eq!(plan(&devices, &panels()), []);
    // The keyboard lies on the lower panel: it has no assignment.
    assert_eq!(
        plan(&[device("event12", 0x4448, true, "")], &panels()[..1]),
        []
    );
}

fn properties(pairs: Vec<(&str, Value<'_>)>) -> HashMap<String, OwnedValue> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.try_into().expect("owned")))
        .collect()
}

#[test]
fn a_device_comes_from_the_properties_of_kwin() {
    let read = Device::from_properties(
        "event10",
        &properties(vec![
            ("vendor", Value::U32(0x04f3)),
            ("product", Value::U32(0x4447)),
            ("tabletTool", Value::Bool(true)),
            ("outputName", Value::from("eDP-1")),
        ]),
    )
    .expect("device");

    assert_eq!(read, device("event10", 0x4447, true, "eDP-1"));
}

#[test]
fn the_flags_and_the_output_may_be_missing() {
    let device = Device::from_properties(
        "event1",
        &properties(vec![
            ("vendor", Value::U32(0x04f3)),
            ("product", Value::U32(1)),
        ]),
    )
    .expect("device");

    assert!(!device.touch);
    assert_eq!(device.output_name, "");
}

#[rstest::rstest]
#[case::no_vendor(vec![("product", Value::U32(1))], "no property vendor")]
#[case::no_product(vec![("vendor", Value::U32(1))], "no property product")]
#[case::vendor_of_another_type(
    vec![("vendor", Value::from("x")), ("product", Value::U32(1))],
    "property vendor has the type s"
)]
#[case::flag_of_another_type(
    vec![("vendor", Value::U32(1)), ("product", Value::U32(1)), ("touch", Value::U32(1))],
    "property touch has the type u"
)]
#[case::tablet_tool_of_another_type(
    vec![("vendor", Value::U32(1)), ("product", Value::U32(1)), ("tabletTool", Value::U32(1))],
    "property tabletTool has the type u"
)]
#[case::output_of_another_type(
    vec![("vendor", Value::U32(1)), ("product", Value::U32(1)), ("outputName", Value::U32(1))],
    "property outputName has the type u"
)]
fn a_device_that_is_not_as_documented_is_an_error(
    #[case] pairs: Vec<(&str, Value<'_>)>,
    #[case] reason: &str,
) {
    assert_eq!(
        Device::from_properties("event1", &properties(pairs)),
        Err(reason.to_owned())
    );
}

//! Which names are taken for HID devices of the keyboard.

use asus_ux8406_keyboard_state::{Device, Error};
use rstest::rstest;

use crate::common::{BLUETOOTH, USB};

#[rstest]
#[case::usb(USB)]
#[case::bluetooth(BLUETOOTH)]
#[case::usb_of_the_ux8406ma("0003:0B05:1B2C.0001")]
#[case::after_many_connections("0003:0B05:1BF2.0001F3A0")]
fn a_device_of_the_keyboard_is_taken(#[case] name: &str) {
    let device: Device = name.parse().unwrap();

    assert_eq!(device.to_string(), name);
}

#[rstest]
#[case::nothing("")]
#[case::another_device("0003:056A:0357.0007")]
#[case::keyboard_on_the_wrong_bus("0005:0B05:1BF2.005D")]
#[case::no_instance("0003:0B05:1BF2.")]
#[case::short_instance("0003:0B05:1BF2.05D")]
#[case::long_instance("0003:0B05:1BF2.000000001")]
#[case::lower_case("0003:0b05:1bf2.005d")]
#[case::not_hexadecimal("0003:0B05:1BF2.005G")]
#[case::a_path("0003:0B05:1BF2.005D/../../x")]
#[case::a_path_in_the_instance("0003:0B05:1BF2./../x")]
#[case::a_line_break("0003:0B05:1BF2.005D\n")]
fn another_name_is_refused(#[case] name: &str) {
    let error = name.parse::<Device>().unwrap_err();

    assert!(matches!(error, Error::NotTheKeyboard(refused) if refused == name));
}

#[test]
fn the_pins_are_in_a_directory_named_after_the_device() {
    let device: Device = USB.parse().unwrap();

    assert_eq!(device.pin_name(), "0003_0B05_1BF2_005D");
}

#[rstest]
#[case::usb("0003_0B05_1BF2_005D", Some(USB))]
#[case::bluetooth("0005_0B05_1BF3_0060", Some(BLUETOOTH))]
#[case::another_device("0003_056A_0357_0007", None)]
#[case::too_few_parts("0003_0B05_1BF2", None)]
#[case::too_many_parts("0003_0B05_1BF2_005D_1", None)]
#[case::not_a_device("programs", None)]
fn a_directory_of_pins_names_its_device(#[case] directory: &str, #[case] name: Option<&str>) {
    let device = Device::from_pin_name(directory);

    assert_eq!(device.map(|device| device.to_string()).as_deref(), name);
}

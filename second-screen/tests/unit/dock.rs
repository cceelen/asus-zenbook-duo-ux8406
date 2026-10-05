//! Recognising the dock port and the lower panel by name.

use asus_ux8406_second_screen::dock::{
    Panel, is_dock_port, is_lower_panel, panel_of_backlight, scale_brightness,
};
use rstest::rstest;

const HUB: &str = "/sys/devices/pci0000:00/0000:00:14.0";

#[test]
fn the_dock_port_is_port_6_of_the_root_hub() {
    assert!(is_dock_port(&format!("{HUB}/usb3/3-6")));
    assert!(is_dock_port(&format!("{HUB}/usb12/12-6")));
}

#[test]
fn the_dock_port_is_found_below_any_root() {
    assert!(is_dock_port(
        "/tmp/x/devices/pci0000:00/0000:00:14.0/usb3/3-6"
    ));
}

#[rstest]
fn other_ports_are_not_the_dock_port(
    #[values("3-2", "3-66", "3-6.1", "3-2/3-2.6", "4-6", "3", "")] device: &str,
) {
    assert!(!is_dock_port(&format!("{HUB}/usb3/{device}")), "{device}");
}

#[test]
fn an_interface_of_the_docked_device_is_not_the_device() {
    assert!(!is_dock_port(&format!("{HUB}/usb3/3-6/3-6:1.0")));
}

#[test]
fn a_hub_without_a_bus_number_is_refused() {
    assert!(!is_dock_port(&format!("{HUB}/usb/-6")));
    assert!(!is_dock_port(&format!("{HUB}/usbx/x-6")));
    assert!(!is_dock_port(&format!("{HUB}/usb3")));
}

#[test]
fn another_controller_is_refused() {
    assert!(!is_dock_port(
        "/sys/devices/pci0000:00/0000:00:0d.0/usb3/3-6"
    ));
    assert!(!is_dock_port(""));
}

#[test]
fn a_backlight_belongs_to_the_connector_it_hangs_below() {
    let drm = "/sys/devices/pci0000:00/0000:00:02.0/drm/card1";

    assert_eq!(
        panel_of_backlight(&format!("{drm}/card1-eDP-1/intel_backlight")),
        Some(Panel::Upper)
    );
    assert_eq!(
        panel_of_backlight(&format!("{drm}/card1-eDP-2/card1-eDP-2-backlight")),
        Some(Panel::Lower)
    );
}

#[rstest]
fn a_backlight_elsewhere_belongs_to_neither_panel(
    #[values(
        "/sys/devices/platform/asus-nb-wmi/backlight/asus_screenpad",
        "/sys/devices/pci0000:00/0000:00:02.0/drm/card1/card1-DP-1/x",
        "/sys/devices/pci0000:00/0000:00:02.0/drm/card1/card1-eDP-1",
        "intel_backlight",
        ""
    )]
    path: &str,
) {
    assert_eq!(panel_of_backlight(path), None, "{path}");
}

#[test]
fn brightness_is_scaled_and_rounded() {
    assert_eq!(scale_brightness(0, 400, 400), Some(0));
    assert_eq!(scale_brightness(400, 400, 400), Some(400));
    assert_eq!(scale_brightness(200, 400, 255), Some(128));
    assert_eq!(scale_brightness(1, 400, 255), Some(1));
    assert_eq!(
        scale_brightness(u32::MAX, u32::MAX, u32::MAX),
        Some(u32::MAX)
    );
}

#[test]
fn a_brightness_off_its_scale_is_refused() {
    assert_eq!(scale_brightness(5, 0, 400), None);
    assert_eq!(scale_brightness(401, 400, 400), None);
}

#[rstest]
fn the_lower_panel_is_edp_2_of_any_card(
    #[values("card0-eDP-2", "card1-eDP-2", "card12-eDP-2")] name: &str,
) {
    assert!(is_lower_panel(name), "{name}");
}

#[rstest]
fn other_drm_entries_are_not_the_lower_panel(
    #[values(
        "card1-eDP-1",
        "card1-eDP-2-backlight",
        "card1-eDP-20",
        "card-eDP-2",
        "cardx-eDP-2",
        "xcard1-eDP-2",
        "card1",
        "renderD128",
        ""
    )]
    name: &str,
) {
    assert!(!is_lower_panel(name), "{name}");
}

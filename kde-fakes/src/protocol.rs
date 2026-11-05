//! The server side of the two KWin protocols for the outputs, from the XML
//! files of kde-common.
#![allow(missing_docs, reason = "generated from the protocol XML")]
#![allow(
    clippy::all,
    clippy::pedantic,
    reason = "generated from the protocol XML"
)]

/// `kde_output_device_registry_v2`, `kde_output_device_v2` and
/// `kde_output_device_mode_v2`.
pub mod device {
    use wayland_server;

    pub mod __interfaces {
        wayland_scanner::generate_interfaces!("../kde-common/protocols/kde-output-device-v2.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("../kde-common/protocols/kde-output-device-v2.xml");
}

/// `kde_output_management_v2`, `kde_output_configuration_v2` and
/// `kde_mode_list_v2`.
pub mod management {
    use super::device::*;
    use wayland_server;

    pub mod __interfaces {
        use super::super::device::__interfaces::*;
        wayland_scanner::generate_interfaces!(
            "../kde-common/protocols/kde-output-management-v2.xml"
        );
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("../kde-common/protocols/kde-output-management-v2.xml");
}

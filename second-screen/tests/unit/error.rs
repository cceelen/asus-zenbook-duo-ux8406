//! Why asus-ux8406-second-screen could not do its work.

use std::error::Error as _;
use std::io;
use std::path::PathBuf;

use asus_ux8406_second_screen::{Error, Panel};

#[test]
fn an_io_error_names_the_file_and_keeps_its_cause() {
    let error = Error::Io {
        path: PathBuf::from("/sys/x/status"),
        source: io::Error::from(io::ErrorKind::PermissionDenied),
    };

    assert!(error.to_string().starts_with("/sys/x/status: "));
    assert!(error.source().is_some());
}

#[test]
fn a_busy_device_counts_as_taken_over() {
    const EBUSY: i32 = 16;
    let busy = Error::Io {
        path: PathBuf::from("/sys/x/brightness"),
        source: io::Error::from_raw_os_error(EBUSY),
    };
    let denied = Error::Io {
        path: PathBuf::from("/sys/x/brightness"),
        source: io::Error::from(io::ErrorKind::PermissionDenied),
    };

    assert!(busy.is_taken_over());
    assert!(!denied.is_taken_over());
    assert!(!Error::NoLowerPanel.is_taken_over());
}

#[test]
fn the_other_errors_say_what_is_missing() {
    assert_eq!(
        Error::UnsupportedModel.to_string(),
        "this is not a Zenbook Duo UX8406; nothing done"
    );
    assert_eq!(Error::NoLowerPanel.to_string(), "no lower panel connector");
    assert!(Error::NoLowerPanel.source().is_none());
    assert_eq!(
        Error::NoBacklight(Panel::Lower).to_string(),
        "no backlight for the lower panel"
    );
    assert_eq!(
        Error::BadBrightness(PathBuf::from("/sys/x/brightness")).to_string(),
        "/sys/x/brightness: not a brightness"
    );
}

#[test]
fn a_missing_upper_backlight_is_named_as_such() {
    assert_eq!(
        Error::NoBacklight(Panel::Upper).to_string(),
        "no backlight for the upper panel"
    );
}

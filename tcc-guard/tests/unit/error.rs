//! What the guard says when it does not start, or stops.

use std::error::Error as _;
use std::io;
use std::path::{Path, PathBuf};

use asus_ux8406_tcc_guard::error::Error;
use rstest::rstest;

#[test]
fn a_missing_file_is_told_from_other_failures() {
    let path = Path::new("/nowhere");
    let missing = Error::io(path)(io::Error::from(io::ErrorKind::NotFound));
    let denied = Error::io(path)(io::Error::from(io::ErrorKind::PermissionDenied));

    assert!(missing.is_missing());
    assert!(!denied.is_missing());
    assert!(!Error::BadValue(path.to_owned()).is_missing());
    assert!(Error::io(path)(rustix::io::Errno::NOENT).is_missing());
}

#[rstest]
#[case(
    Error::BadConfig { path: PathBuf::from("/etc/asus-ux8406-tcc-guard.toml"), reason: "hold_temp: no".to_owned() },
    "/etc/asus-ux8406-tcc-guard.toml: hold_temp: no"
)]
#[case(
    Error::BadValue(PathBuf::from("/sys/x")),
    "/sys/x: not the value expected there"
)]
#[case(Error::TwoSensors(1), "package 1 has two temperature sensors")]
#[case(Error::TooManyPackages, "more than 8 packages")]
#[case(Error::NoSensor, "no coretemp package temperature sensor")]
#[case(Error::NoTjmax(2), "package 2: no usable temp1_crit; set tjmax")]
#[case(Error::NotPresent(3), "package 3 is configured but not present")]
#[case(
    Error::NoControl(1),
    "package 1: name the file of its offset in a [[package]] table of the configuration"
)]
#[case(
    Error::WrongModel { wanted: "UX8406CA".to_owned(), found: "Other".to_owned() },
    "this machine is \"Other\", not a UX8406CA: set \"model\" in the configuration to run here"
)]
fn messages_name_what_they_are_about(#[case] error: Error, #[case] text: &str) {
    assert_eq!(error.to_string(), text);
}

#[test]
fn what_the_system_said_is_kept() {
    let path = Path::new("/sys/offset");
    let denied = || io::Error::from(io::ErrorKind::PermissionDenied);
    let read_only = Error::ReadOnly {
        path: path.to_owned(),
        source: denied(),
    };

    assert!(read_only.to_string().ends_with("(needs root)"));
    assert!(read_only.source().is_some());
    assert!(Error::io(path)(denied()).source().is_some());
    assert!(Error::Signals(denied()).source().is_some());
    assert!(
        Error::Signals(denied())
            .to_string()
            .starts_with("signals: ")
    );
    assert!(Error::NoSensor.source().is_none());
    assert!(Error::Contradiction.to_string().contains("contradict"));
    assert!(
        Error::NotTrusted(path.to_owned())
            .to_string()
            .starts_with("/sys/offset: has to be")
    );
}

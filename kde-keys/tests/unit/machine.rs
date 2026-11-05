//! What the program reads from sysfs.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use asus_ux8406_kde_keys::machine::{i2c_devices, is_model, sorted};

const UX8406CA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ux8406ca");

#[test]
fn the_ux8406ca_is_the_model() {
    assert!(is_model(Path::new(UX8406CA)));
}

#[test]
fn another_model_or_no_product_name_is_not() {
    let root = TempDir::new().expect("directory");

    assert!(!is_model(root.path()));
    root.child("class/dmi/id/product_name")
        .write_str("ASUS Zenbook 14 UX3405MA_UX3405MA\n")
        .expect("product name");
    assert!(!is_model(root.path()));
}

#[test]
fn the_i2c_devices_come_sorted_with_their_children() {
    let devices = i2c_devices(Path::new(UX8406CA)).expect("devices");
    let names: Vec<&str> = devices.iter().map(|(name, _)| name.as_str()).collect();

    assert_eq!(names, ["i2c-0", "i2c-ELAN9008:00", "i2c-ELAN9009:00"]);
    assert_eq!(devices[1].1, ["0018:04F3:4447.0006", "name"]);
}

#[test]
fn without_the_directory_it_is_an_error() {
    let root = TempDir::new().expect("directory");
    let error = i2c_devices(root.path()).expect_err("no directory");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn a_device_directory_that_cannot_be_read_is_an_error() {
    let root = TempDir::new().expect("directory");

    root.child("bus/i2c/devices/i2c-ELAN9008:00/name")
        .write_str("x\n")
        .expect("name");

    let device = root.path().join("bus/i2c/devices/i2c-ELAN9008:00");

    std::fs::set_permissions(&device, std::fs::Permissions::from_mode(0o000)).expect("closed");

    let error = i2c_devices(root.path()).expect_err("unreadable");

    std::fs::set_permissions(&device, std::fs::Permissions::from_mode(0o755)).expect("open");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn an_entry_that_cannot_be_listed_is_an_error() {
    // A file in place of a device directory cannot be listed.
    let root = TempDir::new().expect("directory");

    root.child("bus/i2c/devices/i2c-0")
        .write_str("not a directory")
        .expect("file");
    assert!(i2c_devices(root.path()).is_err());
}

#[test]
fn names_come_sorted_and_an_entry_that_fails_is_an_error() {
    let names = ["b", "a"].map(|name| Ok(name.into()));

    assert_eq!(sorted(names.into_iter()).expect("names"), ["a", "b"]);

    let failing = [
        Ok("a".into()),
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
    ];
    let error = sorted(failing.into_iter()).expect_err("entry");

    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

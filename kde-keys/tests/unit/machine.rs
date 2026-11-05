//! What the program reads from sysfs.

use std::path::Path;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use asus_ux8406_kde_keys::machine::{i2c_devices, is_model};

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
    let devices = i2c_devices(Path::new(UX8406CA));
    let names: Vec<&str> = devices.iter().map(|(name, _)| name.as_str()).collect();

    assert_eq!(names, ["i2c-0", "i2c-ELAN9008:00", "i2c-ELAN9009:00"]);
    assert_eq!(devices[1].1, ["0018:04F3:4447.0006", "name"]);
}

#[test]
fn without_the_directory_there_are_none() {
    let root = TempDir::new().expect("directory");

    assert_eq!(i2c_devices(root.path()), Vec::new());
}

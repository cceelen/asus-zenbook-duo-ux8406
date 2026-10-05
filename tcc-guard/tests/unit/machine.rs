//! The machine as sysfs shows it.

use std::path::{Path, PathBuf};

use asus_ux8406_tcc_guard::config;
use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::input::Settings;
use asus_ux8406_tcc_guard::machine::{Package, check, check_model, control, packages};
use rstest::rstest;

use crate::common::{
    HWMON, Machine, OFFSET, OFFSET_TWO, PRODUCT_NAME, TEMP, TEMP_TWO, machine, two_packages,
};

/// Settings with these lines of a configuration file.
fn configured(lines: &[&str]) -> Settings {
    let mut settings = Settings::default();

    config::apply(&mut settings, &lines.join("\n")).unwrap();
    settings
}

fn first() -> String {
    format!("[[package]]\nid = 0\noffset = \"/{OFFSET}\"")
}

fn second() -> String {
    format!("[[package]]\nid = 1\noffset = \"/{OFFSET_TWO}\"")
}

#[rstest]
fn the_only_package_is_found_at_the_usual_place(machine: Machine) {
    let found = packages(&machine.sys(), &Settings::default()).unwrap();

    assert_eq!(
        found,
        [Package {
            id: 0,
            tjmax: 105,
            temp: machine.join(TEMP),
            offset: machine.join(OFFSET),
        }]
    );
    assert!(check(&found).is_ok());
}

#[rstest]
fn two_packages_have_to_be_configured(two_packages: Machine) {
    let sys = two_packages.sys();

    assert!(matches!(
        packages(&sys, &Settings::default()),
        Err(Error::NoControl(0))
    ));
    assert!(matches!(
        packages(&sys, &configured(&[&first()])),
        Err(Error::NoControl(1))
    ));
    let found = packages(&sys, &configured(&[&first(), &second()])).unwrap();

    assert_eq!(found.len(), 2);
    assert_eq!(found[0].offset, two_packages.join(OFFSET));
    assert_eq!(found[1].offset, two_packages.join(OFFSET_TWO));
    assert_eq!(found[1].temp, two_packages.join(TEMP_TWO));
    assert!(check(&found).is_ok());
}

#[rstest]
fn a_configured_package_has_to_be_there(machine: Machine) {
    assert!(matches!(
        packages(&machine.sys(), &configured(&[&second()])),
        Err(Error::NotPresent(1))
    ));
}

#[rstest]
fn the_limit_comes_from_the_kernel_or_the_configuration(machine: Machine) {
    let sys = machine.sys();

    machine.put(&format!("{HWMON}/temp1_crit"), "999000\n");
    assert!(matches!(
        packages(&sys, &Settings::default()),
        Err(Error::NoTjmax(0))
    ));
    assert_eq!(
        packages(&sys, &configured(&["tjmax = 100"])).unwrap()[0].tjmax,
        100
    );
    machine.remove(&format!("{HWMON}/temp1_crit"));
    assert!(matches!(
        packages(&sys, &Settings::default()),
        Err(Error::NoTjmax(0))
    ));
}

#[rstest]
fn sensors_that_do_not_count_or_count_twice(machine: Machine) {
    let sys = machine.sys();

    machine.sensor(7, 0, 60_000);
    assert!(matches!(
        packages(&sys, &Settings::default()),
        Err(Error::TwoSensors(0))
    ));
    machine.remove("sys/class/hwmon/hwmon7/temp1_label");
    assert_eq!(
        packages(&sys, &Settings::default()).unwrap().len(),
        1,
        "a coretemp device without a label is passed over"
    );
    machine.put("sys/class/hwmon/hwmon7/temp1_label", "Core 0\n");
    machine.put(&format!("{HWMON}/temp1_label"), "Core 0\n");
    assert!(matches!(
        packages(&sys, &Settings::default()),
        Err(Error::NoSensor)
    ));
}

#[rstest]
fn nine_packages_are_one_too_many(machine: Machine) {
    for package in 1..8 {
        machine.sensor(10 + u32::try_from(package).unwrap(), package, 60_000);
    }
    assert!(matches!(
        packages(&machine.sys(), &Settings::default()),
        Err(Error::NoControl(0)),
    ));
    machine.sensor(18, 8, 60_000);
    assert!(matches!(
        packages(&machine.sys(), &Settings::default()),
        Err(Error::TooManyPackages)
    ));
}

#[rstest]
fn files_that_are_not_usable_fail_the_check(machine: Machine) {
    let found = packages(&machine.sys(), &Settings::default()).unwrap();

    machine.put(TEMP, "rubbish\n");
    assert!(matches!(check(&found), Err(Error::BadValue(path)) if path == machine.join(TEMP)));
    machine.put(TEMP, "95000\n");
    machine.put(OFFSET, "99\n");
    assert!(matches!(check(&found), Err(Error::BadValue(path)) if path == machine.join(OFFSET)));
    machine.remove(OFFSET);
    assert!(check(&found).unwrap_err().is_missing());
}

#[rstest]
fn the_model_is_checked_against_the_product_name(machine: Machine) {
    let sys = machine.sys();

    assert!(check_model(&sys, &Settings::default()).is_ok());
    machine.put(PRODUCT_NAME, "ASUS Zenbook 14 UX3405MA_UX3405MA\n");
    assert!(matches!(
        check_model(&sys, &Settings::default()),
        Err(Error::WrongModel { wanted, found })
            if wanted == "UX8406" && found == "ASUS Zenbook 14 UX3405MA_UX3405MA"
    ));
    assert!(check_model(&sys, &configured(&["model = \"UX3405MA\""])).is_ok());
    machine.remove(PRODUCT_NAME);
    assert!(
        check_model(&sys, &Settings::default())
            .unwrap_err()
            .is_missing()
    );
    assert!(
        check_model(&sys, &configured(&["model = \"any\""])).is_ok(),
        "no name is needed for any model"
    );
}

#[test]
fn the_usual_place_is_only_for_a_package_that_is_alone() {
    let sys = Path::new("/sys");

    assert_eq!(
        control(sys, &Settings::default(), 0, true),
        Some(PathBuf::from(format!("/{OFFSET}")))
    );
    assert_eq!(control(sys, &Settings::default(), 0, false), None);
    assert_eq!(
        control(sys, &configured(&[&second()]), 1, false),
        Some(PathBuf::from(format!("/{OFFSET_TWO}")))
    );
    assert_eq!(
        control(Path::new("/mnt/sys"), &configured(&[&second()]), 1, false),
        Some(PathBuf::from(format!("/mnt/{OFFSET_TWO}")))
    );
}

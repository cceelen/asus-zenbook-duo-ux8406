//! The small D-Bus helpers.

use std::time::Duration;

use asus_ux8406_kde_live_test::dbus::{call, has_owner, property, wait_owner};

use crate::support::services::{RotationService, Scene, panels};

const SERVICE: &str = "io.github.cceelen.BuiltinScreenRotation";
const PATH: &str = "/io/github/cceelen/BuiltinScreenRotation";

#[test]
fn a_property_is_read_and_a_wrong_type_is_an_error_with_the_name() {
    let scene = Scene::new(panels());
    let _service = RotationService::new(scene.bus.address());
    let bus = scene.bus.connect();

    assert_eq!(
        property::<bool>(&bus, SERVICE, PATH, SERVICE, "Enabled"),
        Ok(true)
    );

    let error = property::<String>(&bus, SERVICE, PATH, SERVICE, "Enabled").unwrap_err();

    assert!(error.contains("Enabled"), "{error}");
    assert!(
        property::<bool>(&bus, SERVICE, PATH, SERVICE, "Nothing")
            .unwrap_err()
            .contains("Nothing")
    );
}

#[test]
fn a_call_that_fails_names_the_method() {
    let scene = Scene::new(panels());
    let bus = scene.bus.connect();
    let error = call::<_, ()>(
        &bus,
        "org.kde.KWin",
        "/Nothing",
        "org.kde.Nothing",
        "nothing",
        &(),
    )
    .unwrap_err();

    assert!(error.contains("org.kde.Nothing.nothing"), "{error}");
}

#[test]
fn a_name_is_waited_for_until_it_has_an_owner_or_the_time_is_up() {
    let scene = Scene::new(panels());
    let bus = scene.bus.connect();
    let poll = Duration::from_millis(5);

    assert_eq!(has_owner(&bus, SERVICE), Ok(false));
    assert!(!wait_owner(&bus, SERVICE, Duration::from_millis(50), poll));

    let _service = RotationService::new(scene.bus.address());

    assert!(wait_owner(&bus, SERVICE, Duration::from_millis(50), poll));
}

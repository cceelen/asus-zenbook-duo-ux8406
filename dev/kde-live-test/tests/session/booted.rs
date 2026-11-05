//! The state as the image left it.

use asus_ux8406_kde_live_test::booted::checks;

use crate::support::services::{Accel, KeysService, PolicyKeeper, RotationService, Scene, panels};
use crate::support::{Fixture, detail, expect, rows};

const EMOJI: &str = "/component/asus_zenbook_duo_ux8406_shortcuts_desktop";
const ROTATION: &str = "/component/plasma_builtin_screen_rotation";
const KEYS_UNIT: &str = "app-asus\\x2dux8406\\x2dkde\\x2dkeys@autostart.service";
const SCRIPT_CHECK: &str = "KWin loaded the script by itself (as booted)";
const KEYS_CHECK: &str = "asus-ux8406-kde-keys started by its autostart unit (as booted)";
const ROTATION_CHECK: &str =
    "plasma-builtin-screen-rotation started by its autostart unit (as booted)";

fn accel(scene: &Scene) -> Accel {
    Accel::new(
        scene.bus.address(),
        &[
            (
                "/component/kwin",
                &[
                    "ASUS UX8406 Toggle Lower Screen",
                    "ASUS UX8406 Swap Windows",
                ],
            ),
            (EMOJI, &["EmojiSelector"]),
            (ROTATION, &["ToggleAutoRotate"]),
        ],
    )
}

fn load_script(scene: &Scene) {
    scene
        .scripts
        .lock()
        .unwrap()
        .loaded
        .push(("asus-zenbook-duo-ux8406-keys".to_owned(), String::new()));
}

#[test]
fn a_session_as_it_booted_passes_every_check() {
    let scene = Scene::new(panels());
    let _accel = accel(&scene);
    let _keys = KeysService::new(scene.bus.address(), || {});
    let _rotation = RotationService::new(scene.bus.address());
    let _keeper = PolicyKeeper::start(&scene);
    let fixture = scene.fixture();

    load_script(&scene);

    let mut ctx = fixture.ctx();

    checks(&mut ctx);
    for check in [
        SCRIPT_CHECK,
        KEYS_CHECK,
        ROTATION_CHECK,
        "shortcut: lower screen key",
        "auto-rotate policy never on both panels (as booted)",
    ] {
        expect(&ctx, check, "OK");
    }
    assert!(detail(&ctx, KEYS_CHECK).contains(KEYS_UNIT));
    assert!(
        fixture
            .runner
            .called("systemctl --user is-active --quiet app-")
    );
    assert!(
        fixture
            .runner
            .calls()
            .iter()
            .all(|call| !call.contains("daemon-reload") && !call.starts_with("sudo -n udevadm"))
    );
    assert_eq!(scene.scripts.lock().unwrap().loaded.len(), 1);
    assert!(fixture.console.has_said("installed in the image already"));
}

#[test]
fn a_script_that_kwin_did_not_load_and_units_that_did_not_start_fail() {
    let scene = Scene::new(panels());
    let _keys = KeysService::new(scene.bus.address(), || {});
    let fixture = scene.fixture();

    fixture
        .runner
        .rule("systemctl --user is-active --quiet app-plasma", 3, "");

    let mut ctx = fixture.ctx();

    checks(&mut ctx);
    expect(&ctx, SCRIPT_CHECK, "FAIL");
    assert!(detail(&ctx, SCRIPT_CHECK).contains("is not loaded"));
    expect(&ctx, KEYS_CHECK, "OK");
    expect(&ctx, ROTATION_CHECK, "FAIL");
    assert!(detail(&ctx, ROTATION_CHECK).contains("is not active"));
}

#[test]
fn an_active_unit_without_a_bus_name_and_a_missing_unit_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    let mut ctx = fixture.ctx();

    checks(&mut ctx);
    assert!(detail(&ctx, KEYS_CHECK).contains("has no owner"));

    fixture.runner.rule("systemctl --user cat", 1, "");

    let mut ctx = fixture.ctx();

    checks(&mut ctx);
    assert!(detail(&ctx, KEYS_CHECK).contains("finds none of"));
}

#[test]
fn a_session_without_a_bus_fails_the_checks() {
    let dark = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    let mut ctx = dark.ctx();

    checks(&mut ctx);
    assert!(detail(&ctx, SCRIPT_CHECK).contains("session bus"));
    assert!(detail(&ctx, KEYS_CHECK).contains("session bus"));
    assert!(rows(&ctx).iter().any(|row| row.0 == KEYS_CHECK));
}

#[test]
fn a_policy_that_is_not_never_fails_without_a_change() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    checks(&mut ctx);
    expect(
        &ctx,
        "auto-rotate policy never on both panels (as booted)",
        "FAIL",
    );
    assert!(
        rows(&ctx)
            .iter()
            .all(|row| row.0 != "a change to always is reverted to never")
    );
}

#[test]
fn a_bus_without_kwin_fails_the_script_check() {
    let bus = asus_ux8406_kde_fakes::Bus::new();
    let fixture = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());
    let mut ctx = fixture.ctx();

    checks(&mut ctx);
    expect(&ctx, SCRIPT_CHECK, "FAIL");
}

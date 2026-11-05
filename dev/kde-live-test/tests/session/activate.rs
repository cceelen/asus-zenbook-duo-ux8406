//! The activation without a reboot or a logout.

use asus_ux8406_kde_live_test::activate::{entry_command, keyboard_hid, run};

use crate::support::services::{Accel, KeysService, RotationService, Scene, panels};
use crate::support::{Fixture, detail, expect, rows};

const SCRIPT: &str = "/usr/share/kwin/scripts/asus-zenbook-duo-ux8406-keys/contents/code/main.js";

fn installed(fixture: &Fixture) {
    fixture.file(SCRIPT, "");
    fixture.file(
        "/etc/xdg/autostart/asus-ux8406-kde-keys.desktop",
        "[Desktop Entry]\nExec=/usr/libexec/asus-ux8406-kde-keys\n",
    );
    fixture.file(
        "/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop",
        "[Desktop Entry]\nType=Application\nExec=/usr/bin/plasma-builtin-screen-rotation run %U\n",
    );
    fixture.file("/sys/bus/hid/devices/0003:0B05:1BF2.0005/uevent", "");
    fixture.file("/sys/bus/hid/devices/0003:046D:C52B.0001/uevent", "");
}

#[test]
fn the_system_pieces_are_reloaded_and_the_keyboard_triggered() {
    let scene = Scene::new(panels());
    let _accel = Accel::new(scene.bus.address(), &[]);
    let fixture = scene.fixture();

    installed(&fixture);

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    for check in [
        "udev rules reloaded",
        "hwdb updated",
        "HID device 0003:0B05:1BF2.0005 re-triggered",
        "dock port re-triggered",
        "panel connectors re-triggered",
        "backlight re-triggered",
        "udev settled",
    ] {
        expect(&ctx, check, "OK");
    }
    assert!(
        fixture
            .runner
            .called("sudo -n udevadm trigger --action=add /")
    );
    assert!(
        fixture
            .runner
            .calls()
            .iter()
            .all(|call| !call.contains("046D"))
    );
    assert!(fixture.runner.called("sudo -n systemd-hwdb update"));
    assert_eq!(keyboard_hid(&ctx), ["0003:0B05:1BF2.0005"]);
}

#[test]
fn a_failed_privileged_command_is_a_failed_row_with_its_text() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    fixture.runner.rule("sudo -n udevadm control", 1, "");

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "udev rules reloaded", "FAIL");
    assert!(detail(&ctx, "udev rules reloaded").contains("ended with Some(1)"));
}

#[test]
fn no_keyboard_is_a_skipped_row() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    std::fs::remove_dir_all(fixture.root.path().join("sys/bus/hid")).unwrap();

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "keyboard HID devices re-triggered", "SKIP");
}

const KEYS_UNIT: &str = "app-asus\\x2dux8406\\x2dkde\\x2dkeys@autostart.service";
const ROTATION_UNIT: &str = "app-plasma\\x2dbuiltin\\x2dscreen\\x2drotation@autostart.service";
const TARGET_ACTIVE: &str = "systemctl --user is-active --quiet xdg-desktop-autostart.target";

#[test]
fn the_script_is_loaded_and_the_programs_start_through_the_target() {
    let scene = Scene::new(panels());
    let _accel = Accel::new(scene.bus.address(), &[]);
    let _keys = KeysService::new(scene.bus.address(), || {});
    let _rotation = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();

    installed(&fixture);
    fixture.runner.rule(TARGET_ACTIVE, 3, "");

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    assert!(
        !fixture
            .runner
            .calls()
            .iter()
            .any(|call| call.contains("kglobalaccel")),
        "{:?}",
        fixture.runner.calls()
    );
    expect(&ctx, "KWin script loaded over D-Bus", "OK");
    expect(&ctx, "KWin loads the script by itself", "SKIP");
    expect(&ctx, "autostart units made again from the entries", "OK");
    expect(&ctx, "asus-ux8406-kde-keys has an autostart unit", "OK");
    expect(&ctx, "autostart units started", "OK");
    expect(&ctx, "asus-ux8406-kde-keys running", "OK");
    expect(&ctx, "plasma-builtin-screen-rotation running", "OK");
    assert!(detail(&ctx, "autostart units started").contains("started xdg-desktop-autostart"));
    assert!(
        fixture
            .runner
            .called("systemctl --user start xdg-desktop-autostart.target")
    );
    assert!(fixture.runner.called("systemctl --user daemon-reload"));
    assert!(
        fixture
            .runner
            .called(&format!("systemctl --user cat {KEYS_UNIT}"))
    );
    assert_eq!(fixture.runner.starts(), Vec::<String>::new());
    assert!(detail(&ctx, "KWin script loaded over D-Bus").contains("id 7"));
    assert_eq!(
        scene
            .scripts
            .lock()
            .unwrap()
            .loaded
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["asus-zenbook-duo-ux8406-keys"]
    );
}

#[test]
fn a_target_that_is_active_does_not_start_the_new_units_so_they_start_by_name() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "autostart units started", "OK");
    assert!(detail(&ctx, "autostart units started").contains("was active already"));
    assert!(fixture.runner.called(&format!(
        "systemctl --user start {KEYS_UNIT} {ROTATION_UNIT}"
    )));
    assert!(
        !fixture
            .runner
            .called("systemctl --user start xdg-desktop-autostart.target")
    );
}

#[test]
fn a_unit_with_the_plain_name_is_found() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    fixture
        .runner
        .rule("systemctl --user cat app-asus\\x2d", 1, "");

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "asus-ux8406-kde-keys has an autostart unit", "OK");
    assert_eq!(
        detail(&ctx, "asus-ux8406-kde-keys has an autostart unit"),
        "app-asus-ux8406-kde-keys@autostart.service"
    );
}

#[test]
fn entries_without_units_fail_and_nothing_is_started() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    fixture.runner.rule("systemctl --user cat", 1, "");
    fixture
        .runner
        .rule("systemctl --user daemon-reload", 1, "no");

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "autostart units made again from the entries", "FAIL");
    expect(&ctx, "asus-ux8406-kde-keys has an autostart unit", "FAIL");
    assert!(
        detail(&ctx, "asus-ux8406-kde-keys has an autostart unit")
            .contains("finds none of app-asus")
    );
    expect(&ctx, "autostart units started", "FAIL");
    assert!(detail(&ctx, "autostart units started").contains("no autostart unit to start"));
    assert!(rows(&ctx).iter().all(|row| !row.0.ends_with(" running")));
}

#[test]
fn a_failed_start_and_a_unit_that_stays_inactive_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    fixture
        .runner
        .rule("systemctl --user start", 1, "Job failed\n");
    fixture
        .runner
        .rule("systemctl --user is-active --quiet app-", 3, "");

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "autostart units started", "FAIL");
    assert!(detail(&ctx, "autostart units started").contains("Job failed"));
    expect(&ctx, "asus-ux8406-kde-keys running", "FAIL");
    assert!(detail(&ctx, "asus-ux8406-kde-keys running").contains("is not active"));
}

#[test]
fn an_active_unit_without_its_name_on_the_bus_fails() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "asus-ux8406-kde-keys running", "FAIL");
    assert!(detail(&ctx, "asus-ux8406-kde-keys running").contains("has no owner"));
}

#[test]
fn a_script_that_kwin_loaded_already_is_loaded_again() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    scene
        .scripts
        .lock()
        .unwrap()
        .loaded
        .push(("asus-zenbook-duo-ux8406-keys".to_owned(), "old".to_owned()));

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "KWin script loaded over D-Bus", "OK");
    assert_eq!(
        scene.scripts.lock().unwrap().loaded[0].1,
        fixture.root.path().join(&SCRIPT[1..]).display().to_string()
    );
}

#[test]
fn kwin_that_refuses_the_script_and_a_missing_file_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    scene.scripts.lock().unwrap().refuse = true;

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    expect(&ctx, "KWin script loaded over D-Bus", "FAIL");
    assert!(detail(&ctx, "KWin script loaded over D-Bus").contains("loadScript gave -1"));

    std::fs::remove_file(fixture.root.path().join(&SCRIPT[1..])).unwrap();

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    assert!(detail(&ctx, "KWin script loaded over D-Bus").contains("is not installed"));
}

#[test]
fn an_entry_without_an_exec_line_has_no_command() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    fixture.file(
        "/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop",
        "[Desktop Entry]\n",
    );

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    assert!(entry_command(&ctx, "nothing").is_err());
    assert!(
        entry_command(&ctx, "plasma-builtin-screen-rotation")
            .unwrap_err()
            .contains("has no Exec line")
    );
}

#[test]
fn a_session_without_a_bus_fails_the_running_check() {
    let dark = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");

    installed(&dark);

    let mut ctx = dark.ctx();

    run(&mut ctx, false);
    assert!(detail(&ctx, "asus-ux8406-kde-keys running").contains("session bus"));
    assert!(
        rows(&ctx)
            .iter()
            .any(|row| row.0 == "KWin script loaded over D-Bus" && row.2 == "FAIL")
    );
}

#[test]
fn a_bus_without_kwin_fails_the_script() {
    let bus = asus_ux8406_kde_fakes::Bus::new();
    let fixture = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", bus.address());

    installed(&fixture);

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    assert!(detail(&ctx, "KWin script loaded over D-Bus").contains("isScriptLoaded"));
}

#[test]
fn a_kwin_that_fails_the_load_or_the_start_is_reported() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    installed(&fixture);
    scene.scripts.lock().unwrap().fail_load = true;

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    assert!(detail(&ctx, "KWin script loaded over D-Bus").contains("loadScript"));

    {
        let mut scripts = scene.scripts.lock().unwrap();

        scripts.fail_load = false;
        scripts.fail_start = true;
    }

    let mut ctx = fixture.ctx();

    run(&mut ctx, false);
    assert!(detail(&ctx, "KWin script loaded over D-Bus").contains("start"));
}

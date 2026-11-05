//! The runtime dependencies that rpm does not say.

use asus_ux8406_kde_fakes::Style;
use asus_ux8406_kde_live_test::deps::runtime;

use crate::support::services::{Accel, Scene, panels};
use crate::support::{Fixture, detail, expect};

const OBJECT: &str = "/usr/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o";
const QML: &str =
    "/usr/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/contents/ui/main.qml";

fn complete(scene: &Scene) -> Fixture {
    let fixture = scene.fixture();

    fixture.file(OBJECT, "");
    fixture.file("/sys/kernel/btf/vmlinux", "");
    fixture.file(
        QML,
        "import QtQuick\nimport org.kde.kirigami as Kirigami\nimport org.kde.plasma.workspace.dbus as DBus\n  import org.kde.plasma.core\n",
    );
    for module in [
        "org/kde/kirigami",
        "org/kde/plasma/workspace/dbus",
        "org/kde/plasma/core",
    ] {
        fixture.file(&format!("/usr/lib64/qt6/qml/{module}/qmldir"), "");
    }
    fixture
        .runner
        .rule("udev-hid-bpf --version", 0, "udev-hid-bpf 2.1\n")
        .rule(
            "systemd-hwdb query",
            0,
            "HID_BPF_ZENBOOK_DUO_000=0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o\n",
        );
    fixture
}

#[test]
fn a_complete_system_passes_every_check() {
    let scene = Scene::new(panels());
    let _sensor = scene.sensor("normal");
    let _accel = Accel::new(scene.bus.address(), &[]);
    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    for check in [
        "udev-hid-bpf can load the keyboard object",
        "iio-sensor-proxy on the system bus",
        "kglobalacceld on the session bus",
        "KWin scripting and input device interfaces",
        "KWin Wayland output protocols",
        "Plasma QML modules of the applet",
    ] {
        expect(&ctx, check, "OK");
    }
    assert!(
        detail(&ctx, "KWin Wayland output protocols").contains(
            "kde_output_management_v2 version 22; kde_output_device_registry_v2 version 26"
        )
    );
    assert!(detail(&ctx, "Plasma QML modules of the applet").contains("3 modules found"));
    assert!(detail(&ctx, "KWin scripting and input device interfaces").contains("3 input devices"));
}

#[test]
fn old_kwin_with_outputs_as_globals_is_accepted() {
    let scene = Scene::with(Style::Globals, panels());
    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    expect(&ctx, "KWin Wayland output protocols", "OK");
    assert!(
        detail(&ctx, "KWin Wayland output protocols").contains("kde_output_device_v2 version 20")
    );
}

#[test]
fn a_management_that_cannot_set_the_policy_fails() {
    let scene = Scene::with(Style::Old, panels());
    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    expect(&ctx, "KWin Wayland output protocols", "FAIL");
    assert!(detail(&ctx, "KWin Wayland output protocols").contains("version 4"));
}

#[test]
fn a_compositor_that_is_not_kwin_fails() {
    let scene = Scene::with(Style::NoManagement, panels());
    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    expect(&ctx, "KWin Wayland output protocols", "FAIL");
    assert!(detail(&ctx, "KWin Wayland output protocols").contains("no kde_output_management_v2"));
}

#[test]
fn a_session_without_a_compositor_or_a_bus_fails_with_the_reason() {
    let mut fixture = Fixture::new()
        .var("WAYLAND_DISPLAY", "/nonexistent/wayland")
        .var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus")
        .var("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent/bus");

    fixture.runner.missing("udev-hid-bpf");

    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    for check in [
        "udev-hid-bpf can load the keyboard object",
        "iio-sensor-proxy on the system bus",
        "kglobalacceld on the session bus",
        "KWin scripting and input device interfaces",
        "KWin Wayland output protocols",
        "Plasma QML modules of the applet",
    ] {
        expect(&ctx, check, "FAIL");
    }
    assert!(
        detail(&ctx, "KWin Wayland output protocols")
            .contains("Wayland socket /nonexistent/wayland")
    );
    assert!(
        detail(&ctx, "udev-hid-bpf can load the keyboard object")
            .contains("udev-hid-bpf does not run")
    );
    assert!(detail(&ctx, "udev-hid-bpf can load the keyboard object").contains("btf"));
    assert!(detail(&ctx, "Plasma QML modules of the applet").contains("is not installed"));
    fixture.vars.clear();

    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    assert!(detail(&ctx, "KWin Wayland output protocols").contains("WAYLAND_DISPLAY is not set"));
}

#[test]
fn a_bus_without_the_services_fails_with_the_missing_piece() {
    let scene = Scene::new(panels());
    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    // No sensor, no kglobalacceld; the module org.kde.plasma.core is missing.
    std::fs::remove_file(
        fixture
            .root
            .path()
            .join("usr/lib64/qt6/qml/org/kde/plasma/core/qmldir"),
    )
    .unwrap();
    runtime(&mut ctx);
    expect(&ctx, "iio-sensor-proxy on the system bus", "FAIL");
    assert!(
        detail(&ctx, "iio-sensor-proxy on the system bus")
            .contains("net.hadess.SensorProxy has no owner")
    );
    expect(&ctx, "kglobalacceld on the session bus", "FAIL");
    assert!(detail(&ctx, "kglobalacceld on the session bus").contains("not on the session bus"));
    expect(&ctx, "Plasma QML modules of the applet", "FAIL");
    assert!(detail(&ctx, "Plasma QML modules of the applet").contains("org.kde.plasma.core"));
    // The scene's KWin does not know the keyboard script: it answers false.
    expect(&ctx, "KWin scripting and input device interfaces", "OK");
}

#[test]
fn a_sensor_without_an_accelerometer_fails() {
    let scene = Scene::new(panels());
    let mut sensor = scene.sensor("normal");

    sensor.start(false, "undefined");

    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    expect(&ctx, "iio-sensor-proxy on the system bus", "FAIL");
    assert!(
        detail(&ctx, "iio-sensor-proxy on the system bus").contains("HasAccelerometer is false")
    );
}

#[test]
fn the_hid_bpf_pieces_are_named_when_they_are_missing() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    fixture
        .runner
        .rule("systemd-hwdb query", 0, "something else\n");
    fixture.file(
        "/lib/firmware/hid/bpf/0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o",
        "",
    );

    let mut ctx = fixture.ctx();

    runtime(&mut ctx);

    let text = detail(&ctx, "udev-hid-bpf can load the keyboard object");

    assert!(text.contains("hwdb does not name the object") && !text.contains("BTF"));
    assert!(!text.contains("is not in firmware/hid/bpf"), "{text}");
}

#[test]
fn a_kwin_without_any_output_global_is_named() {
    let scene = Scene::with(Style::Globals, Vec::new());
    let fixture = complete(&scene);
    let mut ctx = fixture.ctx();

    runtime(&mut ctx);
    expect(&ctx, "KWin Wayland output protocols", "FAIL");
    assert!(
        detail(&ctx, "KWin Wayland output protocols")
            .contains("neither kde_output_device_registry_v2")
    );
}

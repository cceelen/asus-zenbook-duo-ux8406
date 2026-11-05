//! The tray applet.

use asus_ux8406_kde_live_test::guided::applet;

use crate::support::services::{RotationService, Scene, panels};
use crate::support::{Fixture, detail, expect, with_hands};

#[test]
fn two_clicks_switch_the_program_off_and_on() {
    let scene = Scene::new(panels());
    let service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    with_hands(
        &fixture.console,
        vec![
            (
                "Middle-click the Auto-rotate icon in the system tray",
                Box::new(|| service.set_enabled(false)),
            ),
            (
                "Middle-click the Auto-rotate icon again",
                Box::new(|| service.set_enabled(true)),
            ),
        ],
        || {
            applet(&mut ctx);
        },
    );
    expect(&ctx, "tray applet: click switches rotation off", "OK");
    expect(&ctx, "tray applet: click switches rotation on", "OK");
}

#[test]
fn no_click_in_time_fails() {
    let scene = Scene::new(panels());
    let _service = RotationService::new(scene.bus.address());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    applet(&mut ctx);
    expect(&ctx, "tray applet: click switches rotation off", "FAIL");
    assert!(detail(&ctx, "tray applet: click switches rotation off").contains("did not change"));
}

#[test]
fn a_program_that_is_not_there_and_a_bus_that_is_not_there_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    applet(&mut ctx);
    assert!(
        detail(&ctx, "tray applet: click switches rotation off").contains("lost the connection")
    );

    let dark = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    let mut ctx = dark.ctx();

    applet(&mut ctx);
    expect(&ctx, "tray applet", "FAIL");
}

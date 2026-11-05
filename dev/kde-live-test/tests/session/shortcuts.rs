//! The shortcuts in kglobalacceld.

use asus_ux8406_kde_live_test::auto::{Phase, shortcuts};

use crate::support::services::{Accel, Scene, panels};
use crate::support::{Fixture, detail, expect};

const EMOJI: &str = "/component/asus_zenbook_duo_ux8406_shortcuts_desktop";
const ROTATION: &str = "/component/plasma_builtin_screen_rotation";
const RC: &str = "[kwin]\nASUS UX8406 Toggle Lower Screen=Launch (9),Launch (9),Switch the lower screen on or off\nASUS UX8406 Swap Windows=F19,F19,Swap\n\n[services][asus-zenbook-duo-ux8406-shortcuts.desktop]\nEmojiSelector=Launch (1)\n\n[plasma-builtin-screen-rotation]\nToggleAutoRotate=none,none,Toggle auto-rotate\n";

fn registered(scene: &Scene) -> Accel {
    Accel::new(
        scene.bus.address(),
        &[
            (
                "/component/kwin",
                &[
                    "ASUS UX8406 Toggle Lower Screen",
                    "ASUS UX8406 Swap Windows",
                    "Other",
                ],
            ),
            (EMOJI, &["EmojiSelector"]),
            (ROTATION, &["ToggleAutoRotate"]),
        ],
    )
}

#[test]
fn the_four_shortcuts_are_registered_with_their_keys() {
    let scene = Scene::new(panels());
    let _accel = registered(&scene);
    let fixture = scene.fixture();

    fixture.home_file(".config/kglobalshortcutsrc", RC);

    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Booted);
    for check in [
        "shortcut: lower screen key",
        "shortcut: swap windows",
        "shortcut: emoji selector",
        "shortcut: toggle auto-rotate",
    ] {
        expect(&ctx, check, "OK");
    }
    assert!(detail(&ctx, "shortcut: lower screen key").contains("Launch (9)"));
}

#[test]
fn a_wrong_key_and_a_missing_action_are_told_apart() {
    let scene = Scene::new(panels());
    let _accel = Accel::new(
        scene.bus.address(),
        &[(
            "/component/kwin",
            &[
                "ASUS UX8406 Toggle Lower Screen",
                "ASUS UX8406 Swap Windows",
            ],
        )],
    );
    let fixture = scene.fixture();

    fixture.home_file(
        ".config/kglobalshortcutsrc",
        &RC.replace("Swap Windows=F19", "Swap Windows=F20"),
    );

    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Booted);
    expect(&ctx, "shortcut: lower screen key", "OK");
    expect(&ctx, "shortcut: swap windows", "FAIL");
    assert!(
        detail(&ctx, "shortcut: swap windows")
            .contains("registered, but the key in kglobalshortcutsrc is Some(\"F20\")")
    );
    expect(&ctx, "shortcut: emoji selector", "FAIL");
    assert!(detail(&ctx, "shortcut: emoji selector").contains("no action \"EmojiSelector\""));
    assert!(detail(&ctx, "shortcut: emoji selector").contains("/component/kwin"));
    expect(&ctx, "shortcut: toggle auto-rotate", "FAIL");
    assert!(detail(&ctx, "shortcut: toggle auto-rotate").contains("ToggleAutoRotate"));
}

#[test]
fn a_shortcut_without_an_entry_in_the_file_is_registered_with_its_default() {
    let scene = Scene::new(panels());
    let _accel = registered(&scene);
    let fixture = scene.fixture();

    fixture.home_file(".config/kglobalshortcutsrc", "[kwin]\nOther=A\n");

    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Booted);
    expect(&ctx, "shortcut: emoji selector", "OK");
    assert!(detail(&ctx, "shortcut: emoji selector").contains("has no entry"));
    assert!(
        ctx.report
            .dir()
            .join("raw/kglobalaccel-components.txt")
            .is_file()
    );
}

#[test]
fn components_that_take_the_context_as_an_argument_are_read_too() {
    let scene = Scene::new(panels());
    let _accel = Accel::requiring_context(
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
    );
    let fixture = scene.fixture();

    fixture.home_file(".config/kglobalshortcutsrc", RC);

    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Booted);
    expect(&ctx, "shortcut: emoji selector", "OK");
    expect(&ctx, "shortcut: lower screen key", "OK");
}

#[test]
fn no_kglobalacceld_fails_every_shortcut_with_the_reason() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Booted);
    expect(&ctx, "shortcut: emoji selector", "FAIL");
    assert!(detail(&ctx, "shortcut: emoji selector").contains("org.kde.kglobalaccel"));

    let dark = Fixture::new().var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus");
    let mut ctx = dark.ctx();

    shortcuts(&mut ctx, Phase::Booted);
    assert!(detail(&ctx, "shortcut: swap windows").contains("session bus"));
}

#[test]
fn after_activation_the_rows_have_a_suffix_and_the_file_row_is_a_skip_when_installed_at_run_time() {
    let scene = Scene::new(panels());
    let _accel = registered(&scene);
    let fixture = scene.fixture();

    fixture.home_file(".config/kglobalshortcutsrc", RC);

    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Activated { booted: false });
    expect(&ctx, "shortcut: lower screen key (after activation)", "OK");
    expect(&ctx, "shortcut: swap windows (after activation)", "OK");
    expect(&ctx, "shortcut: emoji selector (after activation)", "SKIP");
    assert!(
        detail(&ctx, "shortcut: emoji selector (after activation)")
            .contains("only when the session starts")
    );
}

#[test]
fn after_activation_of_an_image_as_booted_the_file_row_is_checked() {
    let scene = Scene::new(panels());
    let _accel = registered(&scene);
    let fixture = scene.fixture();

    fixture.home_file(".config/kglobalshortcutsrc", RC);

    let mut ctx = fixture.ctx();

    shortcuts(&mut ctx, Phase::Activated { booted: true });
    expect(&ctx, "shortcut: emoji selector (after activation)", "OK");
}

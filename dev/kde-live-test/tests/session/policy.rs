//! KWin's auto-rotate policy, and the restore command.

use asus_ux8406_kde_live_test::auto::{policy_never, restore};

use crate::support::services::{LOWER, PolicyKeeper, Scene, UPPER, panels};
use crate::support::{Fixture, detail, expect, rows};

const NEVER: &str = "auto-rotate policy never on both panels";
const REVERT: &str = "a change to always is reverted to never";
const ENTRY: &str = "/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop";
const PROGRAM: &str = "/usr/bin/plasma-builtin-screen-rotation";
const RESTART: &str =
    "systemctl --user restart app-plasma\\x2dbuiltin\\x2dscreen\\x2drotation@autostart.service";

#[test]
fn a_program_that_keeps_never_passes_both_checks() {
    let scene = Scene::new(panels());
    let _keeper = PolicyKeeper::start(&scene);
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    policy_never(&mut ctx);
    expect(&ctx, NEVER, "OK");
    expect(&ctx, REVERT, "OK");
    assert!(detail(&ctx, NEVER).contains("eDP-1: never, eDP-2: never"));
}

#[test]
fn the_default_policy_fails_and_stops_the_check() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    policy_never(&mut ctx);
    expect(&ctx, NEVER, "FAIL");
    assert!(detail(&ctx, NEVER).contains("not never: eDP-1: in-tablet-mode"));
    assert_eq!(rows(&ctx).len(), 1);
}

#[test]
fn a_policy_that_nobody_reverts_fails() {
    let scene = Scene::new(panels());

    for name in [UPPER, LOWER] {
        scene.compositor.set_policy(name, 0);
    }

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    policy_never(&mut ctx);
    expect(&ctx, NEVER, "OK");
    expect(&ctx, REVERT, "FAIL");
    assert!(detail(&ctx, REVERT).contains("did not revert it: eDP-1: always"));
}

#[test]
fn a_refusal_of_kwin_is_reported() {
    let scene = Scene::new(panels());

    for name in [UPPER, LOWER] {
        scene.compositor.set_policy(name, 0);
    }

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    // Settle first, then refuse the change.
    scene.compositor.refuse_next("busy");
    policy_never(&mut ctx);
    expect(&ctx, REVERT, "FAIL");
    assert!(
        detail(&ctx, REVERT).contains("KWin refused: busy"),
        "{}",
        detail(&ctx, REVERT)
    );
}

#[test]
fn no_compositor_fails_with_the_reason() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    policy_never(&mut ctx);
    expect(&ctx, "auto-rotate policy never", "FAIL");
}

fn restorable(scene: &Scene) -> Fixture {
    let fixture = scene.fixture();

    fixture.file(ENTRY, &format!("[Desktop Entry]\nExec={PROGRAM} run\n"));
    fixture.home_file(
        ".config/plasma-builtin-screen-rotation.toml",
        "enabled = true\n[policies]\neDP-1 = \"in-tablet-mode\"\neDP-2 = \"in-tablet-mode\"\n",
    );
    for name in [UPPER, LOWER] {
        scene.compositor.set_policy(name, 0);
    }

    let compositor = std::sync::Arc::clone(&scene.compositor);
    let home = fixture.home.path().to_owned();

    fixture.runner.on_command(move |line| {
        if line == format!("{PROGRAM} restore") {
            for name in [UPPER, LOWER] {
                compositor.set_policy(name, 1);
            }
            std::fs::write(
                home.join(".config/plasma-builtin-screen-rotation.toml"),
                "enabled = true\n",
            )
            .unwrap();
        } else if line == RESTART {
            for name in [UPPER, LOWER] {
                compositor.set_policy(name, 0);
            }
        }
    });
    fixture
}

#[test]
fn restore_gives_kwin_its_policy_and_the_program_comes_back() {
    let scene = Scene::new(panels());
    let fixture = restorable(&scene);
    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    expect(&ctx, "restore gives KWin's own policy back", "OK");
    expect(&ctx, "program started again after restore", "OK");
    assert!(fixture.runner.called(RESTART));
    assert!(ctx.report.dir().join("raw/restore.txt").is_file());
}

#[test]
fn restore_with_a_configuration_directory_of_its_own() {
    let scene = Scene::new(panels());
    let mut fixture = restorable(&scene);
    let config = fixture.home.path().join("xdg");

    fixture
        .vars
        .insert("XDG_CONFIG_HOME".to_owned(), config.display().to_string());
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("plasma-builtin-screen-rotation.toml"),
        "enabled = true\n",
    )
    .unwrap();

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    expect(&ctx, "restore gives KWin's own policy back", "OK");
}

#[test]
fn a_failed_restore_a_policy_that_stays_and_a_file_that_keeps_policies_fail() {
    let scene = Scene::new(panels());
    let fixture = restorable(&scene);

    fixture.runner.rule(&format!("{PROGRAM} restore"), 1, "");

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    expect(&ctx, "restore gives KWin's own policy back", "FAIL");
    assert!(
        detail(&ctx, "restore gives KWin's own policy back").contains("restore ended with Some(1)")
    );
}

#[test]
fn policies_that_stay_never_fail() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();

    fixture.file(ENTRY, &format!("[Desktop Entry]\nExec={PROGRAM} run\n"));
    for name in [UPPER, LOWER] {
        scene.compositor.set_policy(name, 0);
    }

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    expect(&ctx, "restore gives KWin's own policy back", "FAIL");
    assert!(detail(&ctx, "restore gives KWin's own policy back").contains("still never"));
    expect(&ctx, "program started again after restore", "OK");
}

#[test]
fn a_file_that_keeps_the_policies_fails() {
    let scene = Scene::new(panels());
    let fixture = restorable(&scene);
    let path = fixture
        .home
        .path()
        .join(".config/plasma-builtin-screen-rotation.toml");
    let compositor = std::sync::Arc::clone(&scene.compositor);

    fixture.runner.on_command(move |line| {
        if line == format!("{PROGRAM} restore") {
            for name in [UPPER, LOWER] {
                compositor.set_policy(name, 1);
            }
        } else if line == RESTART {
            for name in [UPPER, LOWER] {
                compositor.set_policy(name, 0);
            }
        }
    });

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    assert!(path.is_file());
    assert!(detail(&ctx, "restore gives KWin's own policy back").contains("still lists policies"));
}

#[test]
fn a_program_that_does_not_restart_or_comes_back_wrong_fails() {
    let scene = Scene::new(panels());
    let fixture = restorable(&scene);

    fixture.runner.rule(RESTART, 1, "Job failed\n");

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    assert!(detail(&ctx, "program started again after restore").contains("Job failed"));

    // It restarts, but the policies stay as the user's.
    let fixture = restorable(&scene);

    fixture.runner.on_command(|_| {});
    for name in [UPPER, LOWER] {
        scene.compositor.set_policy(name, 1);
    }

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    assert!(
        detail(&ctx, "program started again after restore")
            .contains("running, but the policies are")
    );
}

#[test]
fn a_program_without_an_autostart_unit_fails() {
    let scene = Scene::new(panels());
    let fixture = restorable(&scene);

    fixture.runner.rule("systemctl --user cat", 1, "");

    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    assert!(detail(&ctx, "program started again after restore").contains("finds none of"));
}

#[test]
fn restore_without_an_entry_or_a_compositor_fails() {
    let scene = Scene::new(panels());
    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    restore(&mut ctx);
    expect(&ctx, "restore command", "FAIL");
    assert!(detail(&ctx, "restore command").contains("is not installed"));

    let none = Fixture::new();

    none.file(ENTRY, &format!("[Desktop Entry]\nExec={PROGRAM} run\n"));

    let mut ctx = none.ctx();

    restore(&mut ctx);
    expect(&ctx, "restore command", "FAIL");
    assert!(detail(&ctx, "restore command").contains("WAYLAND_DISPLAY"));
}

#[test]
fn a_compositor_that_ends_while_the_tool_asks_is_reported() {
    let scene = Scene::new(panels());

    for name in [UPPER, LOWER] {
        scene.compositor.set_policy(name, 0);
    }
    scene.compositor.stop_on_next_apply();

    let fixture = scene.fixture();
    let mut ctx = fixture.ctx();

    policy_never(&mut ctx);
    expect(&ctx, REVERT, "FAIL");
}

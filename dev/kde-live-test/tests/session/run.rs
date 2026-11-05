//! The whole run, with the tester skipping every action.

use asus_ux8406_kde_live_test::{Error, run};

use crate::support::services::{Accel, KeysService, RotationService, Scene, panels};
use crate::support::{Fixture, expect, rows};

struct World {
    scene: Scene,
    fixture: Fixture,
    _services: (
        Accel,
        KeysService,
        RotationService,
        asus_ux8406_kde_fakes::Sensor,
    ),
}

fn world() -> World {
    let scene = Scene::new(panels());
    let services = (
        Accel::new(
            scene.bus.address(),
            &[("/component/kwin", &["ASUS UX8406 Swap Windows"])],
        ),
        KeysService::new(scene.bus.address(), || {}),
        RotationService::new(scene.bus.address()),
        scene.sensor("normal"),
    );
    let fixture = scene.fixture();
    let tools = fixture.root.path().join("bin");

    std::fs::create_dir_all(&tools).unwrap();
    for tool in [
        "kscreen-doctor",
        "libinput",
        "konsole",
        "journalctl",
        "systemd-hwdb",
        "pgrep",
        "stdbuf",
    ] {
        std::fs::write(tools.join(tool), "").unwrap();
    }
    std::fs::write(fixture.dir.path().join("rpms/a.rpm"), "").unwrap();
    fixture.file("/sys/class/dmi/id/product_name", "Zenbook Duo UX8406CA\n");
    fixture.file(
        "/etc/xdg/autostart/asus-ux8406-kde-keys.desktop",
        "Exec=/usr/libexec/asus-ux8406-kde-keys\n",
    );
    fixture.file(
        "/etc/xdg/autostart/plasma-builtin-screen-rotation.desktop",
        "Exec=/usr/bin/plasma-builtin-screen-rotation run\n",
    );
    fixture
        .runner
        .rule("rpm -qp --qf", 0, "pkg-a\n")
        .rule("rpm -qp --qf %{NEVRA}", 0, "pkg-a-1-1.x86_64\n")
        .rule("rpm -q pkg-a-1-1.x86_64", 1, "")
        .rule("rpm -qp --requires", 0, "kwin\n")
        .rule("rpm -qp --provides", 0, "pkg-a = 1\n");
    fixture.console.skip_at(">>>");
    World {
        scene,
        fixture: fixture.var("PATH", &tools.display().to_string()),
        _services: services,
    }
}

#[test]
fn a_run_with_every_action_skipped_goes_through_all_parts() {
    let world = world();
    let mut ctx = world.fixture.ctx();

    let counts = run(&mut ctx).unwrap();
    let text = std::fs::read_to_string(ctx.report.dir().join("report.md")).unwrap();

    for section in [
        "## Preflight",
        "## Dependencies and installation",
        "## Activation",
        "## Runtime dependencies",
        "## Automatic checks",
        "## Guided checks",
    ] {
        assert!(text.contains(section), "{section}");
    }
    assert_eq!(counts, ctx.report.counts());
    assert!(counts.skip > 5);
    expect(&ctx, "dnf install of the packages", "OK");
    expect(&ctx, "asus-ux8406-kde-keys running", "OK");
    for stage in ["before", "after"] {
        assert!(
            ctx.report
                .dir()
                .join(format!("raw/{stage}-journal-user.txt"))
                .is_file()
        );
    }
    assert!(world.fixture.console.has_said("Plug the stick back"));
    let _ = &world.scene;
}

#[test]
fn a_run_that_stops_before_the_activation_has_no_later_parts() {
    let world = world();

    world.fixture.runner.rule("sudo -n dnf install", 1, "");

    let mut ctx = world.fixture.ctx();

    run(&mut ctx).unwrap();

    let text = std::fs::read_to_string(ctx.report.dir().join("report.md")).unwrap();

    assert!(!text.contains("## Activation"));
}

#[test]
fn results_that_cannot_be_written_are_an_error() {
    let world = world();
    let mut ctx = world.fixture.ctx();

    std::fs::remove_dir_all(ctx.report.dir()).unwrap();
    assert!(matches!(run(&mut ctx), Err(Error::Write(_))));
}

#[test]
fn a_run_without_sudo_stops_in_the_preflight() {
    let world = world();

    world.fixture.runner.rule("sudo -n true", 1, "");

    let mut ctx = world.fixture.ctx();

    run(&mut ctx).unwrap();
    expect(&ctx, "sudo", "FAIL");

    let text = std::fs::read_to_string(ctx.report.dir().join("report.md")).unwrap();

    assert!(!text.contains("## Dependencies"));
}

#[test]
fn the_run_renews_sudo_until_it_ends() {
    let world = world();
    let mut ctx = world.fixture.ctx();

    run(&mut ctx).unwrap();
    assert!(world.fixture.runner.called("sudo -n -v"));
}

#[test]
fn packages_in_the_image_are_tested_as_booted_before_anything_changes() {
    let world = world();
    let scripts = std::sync::Arc::clone(&world.scene.scripts);

    scripts
        .lock()
        .unwrap()
        .loaded
        .push(("asus-zenbook-duo-ux8406-keys".to_owned(), String::new()));
    world.fixture.runner.rule("rpm -q pkg-a-1-1.x86_64", 0, "");

    let mut ctx = world.fixture.ctx();

    run(&mut ctx).unwrap();

    let text = std::fs::read_to_string(ctx.report.dir().join("report.md")).unwrap();
    let booted = text.find("## As booted").expect("section");

    assert!(booted < text.find("## Dependencies").unwrap());
    expect(&ctx, "KWin loaded the script by itself (as booted)", "OK");
    expect(&ctx, "packages already in the image", "OK");
    assert!(
        rows(&ctx)
            .iter()
            .all(|row| row.0 != "KWin loads the script by itself")
    );
    assert!(!world.fixture.runner.called("sudo -n dnf install"));

    let calls = world.fixture.runner.calls();
    let first_change = calls
        .iter()
        .position(|call| call.starts_with("sudo -n udevadm"))
        .unwrap();
    let units = calls
        .iter()
        .position(|call| call.starts_with("systemctl --user cat"))
        .unwrap();

    assert!(units < first_change);
}

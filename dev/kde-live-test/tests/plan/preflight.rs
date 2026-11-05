//! The checks before the test.

use asus_ux8406_kde_live_test::preflight::{run, sudo};

use crate::support::{Fixture, detail, expect};

fn fixture() -> Fixture {
    let fixture = Fixture::new()
        .var("XDG_SESSION_TYPE", "wayland")
        .var("WAYLAND_DISPLAY", "wayland-0");

    fixture.file("/sys/class/dmi/id/product_name", "Zenbook Duo UX8406CA\n");
    fixture.file("/etc/os-release", "NAME=Fedora\n");
    fixture
        .runner
        .rule("plasmashell --version", 0, "plasmashell 6.6.0\n")
        .rule("kwin_wayland --version", 0, "kwin 6.6.0\n")
        .rule("uname -r", 0, "7.2.11\n");
    fixture
}

#[test]
fn a_ux8406_in_a_wayland_session_with_sudo_passes() {
    let fixture = fixture();
    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx));
    for check in [
        "product name contains UX8406",
        "Wayland session",
        "Plasma and KWin versions",
        "kernel version",
        "network for dnf",
        "sudo",
    ] {
        expect(&ctx, check, "OK");
    }
    assert_eq!(
        detail(&ctx, "Plasma and KWin versions"),
        "plasmashell 6.6.0; kwin 6.6.0"
    );
    assert!(
        std::fs::read_to_string(ctx.report.dir().join("raw/versions.txt"))
            .unwrap()
            .contains("NAME=Fedora")
    );
}

#[test]
fn another_model_stops_the_test() {
    let fixture = fixture();

    fixture.file("/sys/class/dmi/id/product_name", "Other\n");

    let mut ctx = fixture.ctx();

    assert!(!run(&mut ctx));
    expect(&ctx, "product name contains UX8406", "FAIL");
}

#[test]
fn another_session_type_stops_the_test() {
    let fixture = fixture().var("XDG_SESSION_TYPE", "x11");
    let mut ctx = fixture.ctx();

    assert!(!run(&mut ctx));
    expect(&ctx, "Wayland session", "FAIL");
}

#[test]
fn a_session_without_a_display_stops_the_test() {
    let mut fixture = fixture();

    fixture.vars.remove("WAYLAND_DISPLAY");

    let mut ctx = fixture.ctx();

    assert!(!run(&mut ctx));
    expect(&ctx, "Wayland session", "FAIL");
}

#[test]
fn no_network_is_a_skip_and_the_other_failures_stop_the_test() {
    let fixture = fixture();

    fixture
        .runner
        .rule("getent", 2, "")
        .rule("sudo -n true", 1, "");
    fixture.runner.missing("plasmashell --version");

    let mut ctx = fixture.ctx();

    assert!(!run(&mut ctx));
    expect(&ctx, "network for dnf", "SKIP");
    expect(&ctx, "sudo", "FAIL");
    expect(&ctx, "Plasma and KWin versions", "FAIL");
}

#[test]
fn sudo_that_works_at_once_is_not_asked() {
    let fixture = fixture();
    let mut ctx = fixture.ctx();

    assert!(sudo(&mut ctx));
    expect(&ctx, "sudo", "OK");
    assert_eq!(detail(&ctx, "sudo"), "sudo -n works");
    assert!(!fixture.runner.called("sudo -v"));
    assert!(!fixture.console.has_said("press Enter"));
}

#[test]
fn sudo_that_needs_the_enter_key_is_asked_once() {
    let fixture = fixture();

    fixture.runner.sequence("sudo -n true", &[1, 0]);

    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx));
    expect(&ctx, "sudo", "OK");
    assert!(fixture.console.has_said("password of liveuser"));
    assert!(fixture.console.has_said("press Enter"));
    assert_eq!(
        fixture
            .runner
            .calls()
            .iter()
            .filter(|call| call.starts_with("sudo "))
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["sudo -n true", "sudo -v", "sudo -n true"]
    );
}

#[test]
fn sudo_that_fails_twice_names_what_failed() {
    let fixture = fixture();

    fixture.runner.rule("sudo -n true", 1, "");

    let mut ctx = fixture.ctx();

    assert!(!sudo(&mut ctx));
    expect(&ctx, "sudo", "FAIL");
    assert!(detail(&ctx, "sudo").contains("sudo -v worked, but sudo -n true failed"));
}

#[test]
fn sudo_that_may_not_ask_fails_without_asking() {
    let fixture = fixture();

    fixture.runner.rule("sudo -n true", 1, "");

    let mut ctx = fixture.ctx();

    ctx.prompt = false;
    assert!(!sudo(&mut ctx));
    assert!(detail(&ctx, "sudo").contains("may not ask"));
    assert!(!fixture.runner.called("sudo -v"));
}

#[test]
fn a_refused_sudo_v_is_named_with_its_code() {
    let fixture = fixture();

    fixture
        .runner
        .rule("sudo -n true", 1, "")
        .rule("sudo -v", 1, "");

    let mut ctx = fixture.ctx();

    assert!(!sudo(&mut ctx));
    assert!(detail(&ctx, "sudo").contains("sudo -v ended with Some(1)"));
}

//! The tablet-mode switch and the keyboard.

use asus_ux8406_kde_live_test::auto::{Phase, keyboard, tablet};

use crate::support::{Fixture, detail, expect};

#[test]
fn a_tablet_mode_switch_is_reported_and_none_is_not_a_fault() {
    let fixture = Fixture::new();

    fixture.file(
        "/proc/bus/input/devices",
        "N: Name=\"Intel HID switches\"\nB: SW=2\n",
    );
    fixture
        .runner
        .rule("sudo -n libinput list-devices", 0, "Device: x\n");

    let mut ctx = fixture.ctx();

    tablet(&mut ctx);
    expect(&ctx, "tablet-mode switch", "OK");
    assert_eq!(
        detail(&ctx, "tablet-mode switch"),
        "present: Intel HID switches"
    );
    assert!(
        ctx.report
            .dir()
            .join("raw/libinput-list-devices.txt")
            .is_file()
    );

    let none = Fixture::new();
    let mut ctx = none.ctx();

    tablet(&mut ctx);
    expect(&ctx, "tablet-mode switch", "OK");
    assert!(detail(&ctx, "tablet-mode switch").starts_with("none:"));
}

#[test]
fn a_pinned_program_for_the_keyboard_is_an_attached_program() {
    let fixture = Fixture::new();

    fixture.file("/sys/bus/hid/devices/0003:0B05:1BF2.0005/uevent", "");
    fixture.runner.rule(
        "sudo -n ls -R /sys/fs/bpf/hid",
        0,
        "/sys/fs/bpf/hid/0003_0b05_1bf2_0005:\nprog\n",
    );

    let mut ctx = fixture.ctx();

    keyboard(&mut ctx, Phase::Booted);
    expect(&ctx, "keyboard connected", "OK");
    expect(&ctx, "keyboard program attached", "OK");
    assert!(ctx.report.dir().join("raw/bpf-pins.txt").is_file());
}

#[test]
fn no_keyboard_and_no_pinned_program_fail() {
    let fixture = Fixture::new();

    fixture.runner.rule("sudo -n ls -R", 2, "");

    let mut ctx = fixture.ctx();

    keyboard(&mut ctx, Phase::Booted);
    expect(&ctx, "keyboard connected", "FAIL");
    expect(&ctx, "keyboard program attached", "FAIL");
    assert!(detail(&ctx, "keyboard program attached").contains("no pinned program for 0b05"));
}

#[test]
fn the_rows_after_activation_have_a_suffix() {
    let fixture = Fixture::new();

    fixture.runner.rule("sudo -n ls -R", 2, "");

    let mut ctx = fixture.ctx();

    keyboard(&mut ctx, Phase::Activated { booted: true });
    expect(&ctx, "keyboard connected (after activation)", "FAIL");
    expect(&ctx, "keyboard program attached (after activation)", "FAIL");
}

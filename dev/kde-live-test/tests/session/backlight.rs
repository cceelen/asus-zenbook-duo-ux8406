//! The keys of the keyboard backlight, read in the state map.

use asus_ux8406_kde_live_test::guided::backlight;

use crate::support::{Fixture, detail, expect, rows};

const HELPER: &str = "/usr/libexec/asus-ux8406-keyboard-state";
const READ: &str = "sudo -n /usr/libexec/asus-ux8406-keyboard-state -s /nonexistent/";

fn level(number: u8) -> (i32, &'static str) {
    (
        0,
        match number {
            0 => "ASUS_UX8406_KBD_BACKLIGHT=0\nASUS_UX8406_KBD_FN_LOCK=1\n",
            1 => "ASUS_UX8406_KBD_BACKLIGHT=1\nASUS_UX8406_KBD_FN_LOCK=1\n",
            2 => "ASUS_UX8406_KBD_BACKLIGHT=2\nASUS_UX8406_KBD_FN_LOCK=1\n",
            _ => "ASUS_UX8406_KBD_BACKLIGHT=3\nASUS_UX8406_KBD_FN_LOCK=1\n",
        },
    )
}

fn installed() -> Fixture {
    let fixture = Fixture::new();

    fixture.file(HELPER, "");
    fixture
}

#[test]
fn a_level_that_changes_with_each_press_is_a_good_press() {
    let fixture = installed();
    let mut ctx = fixture.ctx();

    // The level before, then the looks of each press.
    fixture
        .runner
        .sequence_texts(READ, &[level(1), level(1), level(2), level(2), level(0)]);
    backlight(&mut ctx);
    expect(&ctx, "keyboard backlight key, press 1 of 2", "OK");
    expect(&ctx, "keyboard backlight key, press 2 of 2", "OK");
    assert!(detail(&ctx, "keyboard backlight key, press 1 of 2").contains("went from 1 to 2"));
    assert!(detail(&ctx, "keyboard backlight key, press 2 of 2").contains("went from 2 to 0"));
    assert!(
        fixture
            .runner
            .called("sudo -n /usr/libexec/asus-ux8406-keyboard-state -s /nonexistent/asus-ux8406-keyboard-state load 0003:0B05:1BF2.0000")
    );
    assert!(fixture.console.has_said("type s and Enter to skip"));
}

#[test]
fn a_level_that_does_not_change_fails_and_stops_the_asks() {
    let fixture = installed();
    let mut ctx = fixture.ctx();

    fixture.runner.rule(READ, level(1).0, level(1).1);
    backlight(&mut ctx);
    expect(&ctx, "keyboard backlight key, press 1 of 2", "FAIL");
    assert!(detail(&ctx, "keyboard backlight key, press 1 of 2").contains("did not change"));
    assert_eq!(rows(&ctx).len(), 1);
}

#[test]
fn a_skip_while_the_tool_looks_is_a_skip() {
    let fixture = installed();
    let mut ctx = fixture.ctx();

    fixture.runner.rule(READ, level(3).0, level(3).1);
    fixture.console.skip_at("Press the keyboard backlight key");
    backlight(&mut ctx);
    expect(&ctx, "keyboard backlight key, press 1 of 2", "SKIP");
}

#[test]
fn without_the_program_the_tester_says_whether_the_light_changed() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    fixture.console.answer(&[true, false]);
    backlight(&mut ctx);
    expect(&ctx, "keyboard backlight key, press 1 of 2", "OK");
    expect(&ctx, "keyboard backlight key, press 2 of 2", "FAIL");
    assert!(detail(&ctx, "keyboard backlight key, press 1 of 2").contains("no state map"));
    assert!(fixture.console.has_said("Did the keyboard light change?"));
}

#[test]
fn a_map_that_cannot_be_read_is_asked_about_too() {
    // No pinned program, no output, an error, and a text that is no level.
    for (code, text) in [
        (0, ""),
        (1, "error\n"),
        (0, "ASUS_UX8406_KBD_BACKLIGHT=x\n"),
    ] {
        let again = installed();
        let mut again_ctx = again.ctx();

        again.runner.rule(READ, code, text);
        again.console.answer(&[true, true]);
        backlight(&mut again_ctx);
        expect(&again_ctx, "keyboard backlight key, press 2 of 2", "OK");
    }
}

#[test]
fn a_map_that_cannot_be_read_during_a_look_is_looked_at_again() {
    let fixture = installed();
    let mut ctx = fixture.ctx();

    fixture
        .runner
        .sequence_texts(READ, &[level(1), (1, ""), level(2), level(2), level(3)]);
    backlight(&mut ctx);
    expect(&ctx, "keyboard backlight key, press 1 of 2", "OK");
    expect(&ctx, "keyboard backlight key, press 2 of 2", "OK");
    assert!(detail(&ctx, "keyboard backlight key, press 2 of 2").contains("went from 2 to 3"));
}

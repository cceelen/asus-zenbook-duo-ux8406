//! Keys that start a program.

use asus_ux8406_kde_live_test::guided::key_starts;

use crate::support::{Fixture, detail, expect, rows, with_hands};

#[test]
fn a_key_that_starts_the_program_passes_and_the_program_is_closed() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    // The window is there at the first look, and gone at the next one.
    fixture
        .runner
        .sequence("pgrep -x plasma-emojier", &[1, 0, 1]);
    with_hands(
        &fixture.console,
        vec![("Press F11", Box::new(|| {}))],
        || {
            key_starts(
                &mut ctx,
                "F11 opens the emoji selector",
                "F11 (the emoji key)",
                "plasma-emojier",
            );
        },
    );
    expect(&ctx, "F11 opens the emoji selector", "OK");
    assert!(fixture.runner.called("pkill -x plasma-emojier"));
    assert_eq!(rows(&ctx).len(), 1, "closed: no information row");
}

#[test]
fn a_window_that_stays_open_is_said() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    fixture.runner.rule("pgrep -x plasma-emojier", 0, "123\n");
    key_starts(
        &mut ctx,
        "F11 opens the emoji selector",
        "F11 (the emoji key)",
        "plasma-emojier",
    );
    expect(&ctx, "F11 opens the emoji selector", "OK");
    expect(&ctx, "plasma-emojier closed", "OK");
    assert!(detail(&ctx, "plasma-emojier closed").contains("close it by hand"));
}

#[test]
fn a_key_that_starts_nothing_fails() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    fixture.runner.rule("pgrep", 1, "");
    key_starts(
        &mut ctx,
        "F12 opens System Settings",
        "F12",
        "systemsettings",
    );
    expect(&ctx, "F12 opens System Settings", "FAIL");
    assert!(detail(&ctx, "F12 opens System Settings").contains("no process systemsettings"));
}

#[test]
fn without_a_limit_the_tool_waits_for_the_key_and_says_so() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    ctx.timing.guided = None;
    fixture.runner.rule("pgrep", 1, "");
    with_hands(
        &fixture.console,
        vec![(
            "Press F12",
            Box::new(|| {
                std::thread::sleep(std::time::Duration::from_millis(2500));
                fixture.runner.sequence("pgrep -x systemsettings", &[0, 1]);
            }),
        )],
        || {
            key_starts(
                &mut ctx,
                "F12 opens System Settings",
                "F12",
                "systemsettings",
            );
        },
    );
    expect(&ctx, "F12 opens System Settings", "OK");
    assert!(fixture.console.has_said("(type s and Enter to skip)"));
    assert!(!fixture.console.has_said("gives up"));
}

#[test]
fn with_a_limit_the_tool_says_when_it_gives_up() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    fixture.runner.rule("pgrep", 1, "");
    key_starts(
        &mut ctx,
        "F12 opens System Settings",
        "F12",
        "systemsettings",
    );
    assert!(fixture.console.has_said("the tool gives up after 2 s"));
}

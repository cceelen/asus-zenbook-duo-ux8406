//! The two built-in panels.

use asus_ux8406_kde_fakes::Output;
use asus_ux8406_kde_live_test::auto::panels;

use crate::support::services::{Scene, panels as both};
use crate::support::{Fixture, detail, expect};

const CHECK: &str = "eDP-1 and eDP-2 present, built-in, distinct UUIDs";
const DOCTOR: &str = "\u{1b}[0;1mOutput: 1 eDP-1 aaaa-1111\u{1b}[0m\nOutput: 2 eDP-2 bbbb-2222\n";

#[test]
fn two_built_in_panels_with_two_uuids_in_a_valid_layout_pass() {
    let scene = Scene::new(both());
    let fixture = scene.fixture();

    fixture.runner.rule("kscreen-doctor -o", 0, DOCTOR);

    let mut ctx = fixture.ctx();

    panels(&mut ctx);
    expect(&ctx, CHECK, "OK");
    assert!(
        detail(&ctx, CHECK).contains("eDP-2 at 0,1440"),
        "{}",
        detail(&ctx, CHECK)
    );
}

#[test]
fn a_missing_panel_is_named() {
    let scene = Scene::new(vec![Output::panel("eDP-1", 0, 0)]);
    let fixture = scene.fixture();

    fixture.runner.rule("kscreen-doctor -o", 0, DOCTOR);

    let mut ctx = fixture.ctx();

    panels(&mut ctx);
    expect(&ctx, CHECK, "FAIL");
    assert!(detail(&ctx, CHECK).contains("not present as built-in outputs: eDP-2"));
}

#[test]
fn the_same_uuid_twice_or_none_fails() {
    let scene = Scene::new(both());
    let fixture = scene.fixture();

    fixture.runner.rule(
        "kscreen-doctor -o",
        0,
        "Output: 1 eDP-1 same\nOutput: 2 eDP-2 same\n",
    );

    let mut ctx = fixture.ctx();

    panels(&mut ctx);
    expect(&ctx, CHECK, "FAIL");
    assert!(detail(&ctx, CHECK).contains("not two different ones"));

    fixture.runner.rule("kscreen-doctor -o", 1, "");

    let mut ctx = fixture.ctx();

    panels(&mut ctx);
    expect(&ctx, CHECK, "FAIL");
}

#[test]
fn panels_that_lie_over_each_other_fail() {
    let scene = Scene::new(vec![
        Output::panel("eDP-1", 0, 0),
        Output::panel("eDP-2", 0, 0),
    ]);
    let fixture = scene.fixture();

    fixture.runner.rule("kscreen-doctor -o", 0, DOCTOR);

    let mut ctx = fixture.ctx();

    panels(&mut ctx);
    expect(&ctx, CHECK, "FAIL");
    assert!(detail(&ctx, CHECK).contains("gap or an overlap"));
}

#[test]
fn no_compositor_fails_with_the_reason() {
    let fixture = Fixture::new();
    let mut ctx = fixture.ctx();

    panels(&mut ctx);
    expect(&ctx, "built-in panels", "FAIL");
    assert!(detail(&ctx, "built-in panels").contains("WAYLAND_DISPLAY is not set"));
}

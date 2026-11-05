//! The raw state.

use asus_ux8406_kde_live_test::collect::run;

use crate::support::Fixture;

#[test]
fn the_state_of_the_session_is_saved_under_the_stage() {
    let fixture = Fixture::new();

    fixture.home_file(".config/kwinoutputconfig.json", "{}");
    fixture.home_file(".config/kcminputrc", "[Libinput]\n");
    fixture
        .runner
        .rule("journalctl --user", 0, "user journal\n");
    fixture
        .runner
        .rule("kscreen-doctor", 0, "Output: 1 eDP-1\n");

    let mut ctx = fixture.ctx();

    run(&mut ctx, "before");

    let raw = ctx.report.dir().join("raw");

    assert_eq!(
        std::fs::read_to_string(raw.join("before-journal-user.txt")).unwrap(),
        "user journal\n"
    );
    assert_eq!(
        std::fs::read_to_string(raw.join("before-kwinoutputconfig.json")).unwrap(),
        "{}"
    );
    assert_eq!(
        std::fs::read_to_string(raw.join("before-kcminputrc")).unwrap(),
        "[Libinput]\n"
    );
    assert_eq!(
        std::fs::read_to_string(raw.join("before-kglobalshortcutsrc")).unwrap(),
        ""
    );
    for name in [
        "journal-kernel",
        "kscreen-doctor",
        "libinput-list-devices",
        "rpm-qa",
    ] {
        assert!(raw.join(format!("before-{name}.txt")).is_file(), "{name}");
    }
}

//! The "ready" lines of the two programs.

use asus_ux8406_kde_live_test::auto::ready;

use crate::support::{Fixture, detail, expect};

#[test]
fn a_ready_line_in_the_journal_counts() {
    let fixture = Fixture::new();

    fixture.runner.rule(
        "journalctl --user -b --no-pager -o cat -t asus-ux8406-kde-keys",
        0,
        "x\nready\n",
    );
    fixture.runner.rule(
        "journalctl --user -b --no-pager -o cat -t plasma-builtin-screen-rotation",
        0,
        "not yet\n",
    );

    let mut ctx = fixture.ctx();

    ready(&mut ctx);
    expect(&ctx, "asus-ux8406-kde-keys says ready", "OK");
    expect(&ctx, "plasma-builtin-screen-rotation says ready", "FAIL");
    assert!(
        detail(&ctx, "plasma-builtin-screen-rotation says ready").contains("no \"ready\" line")
    );
}

#[test]
fn a_log_file_of_the_tool_does_not_count() {
    let fixture = Fixture::new();
    let dir = fixture.dir.path().join("results/raw");

    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("asus-ux8406-kde-keys.log"), "ready\n").unwrap();
    fixture.runner.rule("journalctl", 0, "");

    let mut ctx = fixture.ctx();

    ready(&mut ctx);
    expect(&ctx, "asus-ux8406-kde-keys says ready", "FAIL");
}

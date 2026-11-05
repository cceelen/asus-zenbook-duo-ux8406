//! The installation.

use asus_ux8406_kde_live_test::install::run;

use crate::support::{Fixture, detail, expect, rows};

fn fixture() -> Fixture {
    let fixture = Fixture::new();
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
    fixture
        .runner
        .rule("rpm -qp --qf", 0, "pkg-a\n")
        .rule("rpm -qp --requires", 0, "kwin\n")
        .rule("rpm -qp --provides", 0, "pkg-a = 1\n");
    fixture.var("PATH", &tools.display().to_string())
}

#[test]
fn the_packages_are_installed_and_checked() {
    let fixture = fixture();
    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx, false));
    expect(&ctx, "dnf install of the packages", "OK");
    expect(&ctx, "packages installed", "OK");
    expect(&ctx, "test tools present", "OK");
    assert!(fixture.runner.called("sudo -n dnf install -y "));
    assert!(
        fixture
            .runner
            .calls()
            .iter()
            .any(|call| call.ends_with("a.rpm"))
    );
    assert!(!fixture.runner.called("sudo -n dnf install -y /usr/bin"));
    assert!(ctx.report.dir().join("raw/dnf-install.txt").is_file());
}

#[test]
fn packages_in_the_image_are_not_installed_again_but_still_checked() {
    let fixture = fixture();
    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx, true));
    expect(&ctx, "packages already in the image", "OK");
    assert!(detail(&ctx, "packages already in the image").contains("pkg-a"));
    assert!(!fixture.runner.called("sudo -n dnf"));
    assert!(
        rows(&ctx)
            .iter()
            .all(|row| row.0 != "dnf install of the packages")
    );
    expect(&ctx, "packages installed", "OK");
    expect(&ctx, "requirements after dnf", "OK");
    expect(&ctx, "files verified (rpm -V)", "OK");
}

#[test]
fn nothing_to_fetch_runs_dnf_without_repositories() {
    let fixture = fixture();

    fixture.runner.rule("getent", 2, "");

    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx, false));
    assert!(
        fixture
            .runner
            .called("sudo -n dnf install -y --disablerepo=* /")
    );
    expect(&ctx, "dnf install of the packages", "OK");
}

#[test]
fn something_to_fetch_with_a_network_runs_dnf_with_repositories() {
    let fixture = fixture();

    fixture.runner.rule("rpm -q --whatprovides", 1, "");

    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx, false));
    assert!(fixture.runner.called("sudo -n dnf install -y /"));
    assert!(
        !fixture
            .runner
            .called("sudo -n dnf install -y --disablerepo")
    );
}

#[test]
fn missing_tools_without_a_network_are_skipped_with_the_reason() {
    let fixture = fixture().var("PATH", "/nonexistent");

    fixture.runner.rule("getent", 2, "");

    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx, false));
    expect(&ctx, "test tools install", "SKIP");
    assert!(detail(&ctx, "test tools install").contains("/usr/bin/kscreen-doctor"));
    expect(&ctx, "test tools present", "FAIL");
    assert!(
        !fixture
            .runner
            .calls()
            .iter()
            .any(|call| call.contains("/usr/bin/"))
    );
}

#[test]
fn missing_tools_are_installed_by_dnf() {
    let fixture = fixture().var("PATH", "/nonexistent");
    let mut ctx = fixture.ctx();

    assert!(run(&mut ctx, false));
    assert!(fixture.runner.called(
        "sudo -n dnf install -y /usr/bin/kscreen-doctor /usr/bin/libinput /usr/bin/konsole"
    ));
    // dnf says yes but the programs are still not there.
    expect(&ctx, "test tools present", "FAIL");
    assert!(detail(&ctx, "test tools present").contains("kscreen-doctor, libinput"));
    assert!(ctx.report.dir().join("raw/dnf-tools.txt").is_file());
}

#[test]
fn no_package_files_stop_the_test() {
    let fixture = fixture();
    let mut ctx = fixture.ctx();

    std::fs::remove_file(fixture.dir.path().join("rpms/a.rpm")).unwrap();
    assert!(!run(&mut ctx, false));
    expect(&ctx, "packages in rpms/", "FAIL");
}

#[test]
fn a_missing_network_stops_the_test_before_dnf() {
    let fixture = fixture();

    fixture.runner.rule("rpm -q --whatprovides", 1, "");
    fixture.runner.rule("getent", 2, "");

    let mut ctx = fixture.ctx();

    assert!(!run(&mut ctx, false));
    assert!(!fixture.runner.called("sudo -n dnf"));
}

#[test]
fn a_failed_dnf_stops_the_test() {
    let fixture = fixture();

    fixture.runner.rule("sudo -n dnf install", 1, "");

    let mut ctx = fixture.ctx();

    assert!(!run(&mut ctx, false));
    expect(&ctx, "dnf install of the packages", "FAIL");
    assert!(rows(&ctx).iter().all(|row| row.0 != "packages installed"));
}

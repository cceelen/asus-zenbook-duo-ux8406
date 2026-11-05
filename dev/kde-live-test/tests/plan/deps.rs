//! The checks of the dependencies that rpm expresses.

use asus_ux8406_kde_live_test::deps::{Packages, after, before, network, rpm_files};

use crate::support::{Fixture, detail, expect};

fn with_rpms() -> (Fixture, Vec<std::path::PathBuf>) {
    let fixture = Fixture::new();
    let rpms = fixture.dir.path().join("rpms");

    for name in ["b.rpm", "a.rpm"] {
        std::fs::write(rpms.join(name), "").unwrap();
    }
    std::fs::write(rpms.join("notes.txt"), "").unwrap();

    let files = vec![rpms.join("a.rpm"), rpms.join("b.rpm")];

    for (file, name, requires, provides) in [
        (
            &files[0],
            "pkg-a",
            "kwin >= 6\nrpmlib(X) <= 1\nb-lib\niio-sensor-proxy\n(x if y)\n",
            "pkg-a = 1\n",
        ),
        (&files[1], "pkg-b", "kwin\n", "pkg-b = 1\nb-lib = 1\n"),
    ] {
        let path = file.display();

        fixture.runner.rule(
            &format!("rpm -qp --qf %{{NAME}}\\n {path}"),
            0,
            &format!("{name}\n"),
        );
        fixture
            .runner
            .rule(&format!("rpm -qp --requires {path}"), 0, requires);
        fixture
            .runner
            .rule(&format!("rpm -qp --provides {path}"), 0, provides);
    }
    fixture
        .runner
        .rule("rpm -q --whatprovides iio-sensor-proxy", 1, "");
    (fixture, files)
}

#[test]
fn the_package_files_are_listed_in_order() {
    let (fixture, files) = with_rpms();

    assert_eq!(rpm_files(&fixture.ctx()), files);
    assert_eq!(
        rpm_files(&Fixture::new().ctx()),
        Vec::<std::path::PathBuf>::new()
    );
}

#[test]
fn what_dnf_fetches_is_named_before_it_runs() {
    let (fixture, files) = with_rpms();
    let mut ctx = fixture.ctx();

    assert_eq!(
        before(&mut ctx, &files),
        Some(Packages {
            names: names(),
            fetch: true,
        })
    );
    expect(&ctx, "requirements before dnf", "OK");

    let text = detail(&ctx, "requirements before dnf");

    assert!(text.contains("2 packages, 3 requirements"), "{text}");
    assert!(
        text.contains(
            "1 present in the live image, 1 from the stick, 1 for dnf to fetch (iio-sensor-proxy)"
        ),
        "{text}"
    );
    assert!(
        text.contains("1 conditional left to dnf; network yes"),
        "{text}"
    );
}

#[test]
fn a_missing_network_is_reported_with_the_missing_packages() {
    let (fixture, files) = with_rpms();

    fixture.runner.rule("getent", 2, "");

    let mut ctx = fixture.ctx();

    assert_eq!(before(&mut ctx, &files), None);
    expect(&ctx, "requirements before dnf", "FAIL");
    assert!(detail(&ctx, "requirements before dnf").contains("no network, and dnf has to fetch"));
    assert!(detail(&ctx, "requirements before dnf").contains("iio-sensor-proxy"));
}

#[test]
fn nothing_to_fetch_needs_no_network() {
    let (fixture, files) = with_rpms();

    fixture
        .runner
        .rule("rpm -q --whatprovides iio-sensor-proxy", 0, "x");
    fixture.runner.rule("getent", 2, "");

    let mut ctx = fixture.ctx();

    assert_eq!(
        before(&mut ctx, &files),
        Some(Packages {
            names: names(),
            fetch: false,
        })
    );
    assert!(!network(&ctx));
    expect(&ctx, "requirements before dnf", "OK");
    assert!(
        detail(&ctx, "requirements before dnf")
            .contains("0 for dnf to fetch; 1 conditional left to dnf; network no")
    );
}

fn names() -> Vec<String> {
    vec!["pkg-a".to_owned(), "pkg-b".to_owned()]
}

#[test]
fn an_installation_that_is_complete_passes_all_three_checks() {
    let fixture = Fixture::new();

    fixture
        .runner
        .rule("rpm -q pkg-a pkg-b", 0, "pkg-a-1\npkg-b-1\n");
    fixture
        .runner
        .rule("rpm -q --requires", 0, "kwin\nrpmlib(X)\n");

    let mut ctx = fixture.ctx();

    after(&mut ctx, &names());
    expect(&ctx, "packages installed", "OK");
    expect(&ctx, "requirements after dnf", "OK");
    expect(&ctx, "files verified (rpm -V)", "OK");
    assert!(detail(&ctx, "packages installed").contains("pkg-a-1, pkg-b-1"));
    assert!(detail(&ctx, "requirements after dnf").contains("2 requirements, all provided"));
}

#[test]
fn a_missing_package_a_missing_requirement_and_a_changed_file_fail() {
    let fixture = Fixture::new();

    fixture
        .runner
        .rule("rpm -q pkg-a pkg-b", 1, "package pkg-b is not installed\n");
    fixture
        .runner
        .rule("rpm -q --requires", 0, "gone-lib\nkwin\n");
    fixture.runner.rule("rpm -q --whatprovides gone-lib", 1, "");
    fixture.runner.rule("rpm -V", 1, "S.5....T.  c /etc/x\n");

    let mut ctx = fixture.ctx();

    after(&mut ctx, &names());
    expect(&ctx, "packages installed", "FAIL");
    expect(&ctx, "requirements after dnf", "FAIL");
    expect(&ctx, "files verified (rpm -V)", "FAIL");
    assert!(detail(&ctx, "requirements after dnf").contains("nothing provides: gone-lib"));
    assert!(detail(&ctx, "files verified (rpm -V)").contains("/etc/x"));
}

#[test]
fn rpm_v_output_with_a_good_code_is_still_a_failure() {
    let fixture = Fixture::new();

    fixture.runner.rule("rpm -V", 0, "missing   /usr/bin/x\n");

    let mut ctx = fixture.ctx();

    after(&mut ctx, &names());
    expect(&ctx, "files verified (rpm -V)", "FAIL");
}

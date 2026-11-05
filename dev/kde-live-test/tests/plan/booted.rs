//! The detection of packages that the image has already.

use asus_ux8406_kde_live_test::booted::installed;

use crate::support::Fixture;

const NEVRA: &str = "rpm -qp --qf %{NEVRA}\\n";

fn files(fixture: &Fixture) -> Vec<std::path::PathBuf> {
    let rpms = fixture.dir.path().join("rpms");

    ["a.rpm", "b.rpm"]
        .iter()
        .map(|name| {
            std::fs::write(rpms.join(name), "").unwrap();
            rpms.join(name)
        })
        .collect()
}

#[test]
fn packages_with_the_same_version_in_the_image_are_found() {
    let fixture = Fixture::new();
    let files = files(&fixture);

    fixture.runner.rule(NEVRA, 0, "pkg-1-1.fc45.x86_64\n");
    assert!(installed(&fixture.ctx(), &files));
    assert!(fixture.runner.called("rpm -q pkg-1-1.fc45.x86_64"));
}

#[test]
fn one_package_that_is_not_installed_with_this_version_is_not_enough() {
    let fixture = Fixture::new();
    let files = files(&fixture);

    fixture.runner.rule(NEVRA, 0, "pkg-1-1.fc45.x86_64\n");
    fixture
        .runner
        .sequence("rpm -q pkg-1-1.fc45.x86_64", &[0, 1]);
    assert!(!installed(&fixture.ctx(), &files));
}

#[test]
fn no_files_and_unreadable_headers_are_not_installed() {
    let fixture = Fixture::new();
    let files = files(&fixture);

    assert!(!installed(&fixture.ctx(), &[]));
    fixture.runner.rule(NEVRA, 1, "");
    assert!(!installed(&fixture.ctx(), &files));
    fixture.runner.rule(NEVRA, 0, "error: a b\n");
    assert!(!installed(&fixture.ctx(), &files));
}

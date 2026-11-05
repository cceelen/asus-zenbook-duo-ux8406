//! The results directory.

use asus_ux8406_kde_live_test::report::{Counts, Kind, Report, Status};

#[test]
fn a_row_is_written_when_it_is_added() {
    let dir = tempfile::tempdir().unwrap();
    let results = dir.path().join("results");
    let mut report = Report::open(&results, "Title").unwrap();

    report.section("Part");
    report.add("check | one", Kind::Auto, Status::Ok, "line one\nline two");
    report.add("two", Kind::Guided, Status::Fail, "bad");
    report.add("three", Kind::Guided, Status::Skip, "later");
    report.note("A note.");

    let text = std::fs::read_to_string(results.join("report.md")).unwrap();

    assert!(text.starts_with("# Title\n"));
    assert!(text.contains("| check / one | automatic | OK | line one line two |"));
    assert!(text.contains("| two | guided | FAIL | bad |"));
    assert!(text.contains("| three | guided | SKIP | later |"));
    assert!(text.contains("\nA note.\n"));
    assert_eq!(
        report.counts(),
        Counts {
            ok: 1,
            fail: 1,
            skip: 1
        }
    );
    assert_eq!(report.dir(), results);
    assert_eq!(report.broken(), None);
}

#[test]
fn a_raw_log_is_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let results = dir.path().join("results");
    let mut report = Report::open(&results, "T").unwrap();

    report.raw("log.txt", "text");
    assert_eq!(
        std::fs::read_to_string(results.join("raw/log.txt")).unwrap(),
        "text"
    );
}

#[test]
fn an_earlier_report_moves_aside() {
    let dir = tempfile::tempdir().unwrap();
    let results = dir.path().join("results");
    let mut first = Report::open(&results, "First").unwrap();

    first.add("old", Kind::Auto, Status::Ok, "");

    // The old run is kept in results.previous, also the second time.
    for _ in 0..2 {
        let mut fresh = Report::open(&results, "Fresh").unwrap();

        fresh.add("fresh", Kind::Auto, Status::Ok, "");
    }

    let previous = std::fs::read_to_string(dir.path().join("results.previous/report.md")).unwrap();

    assert!(previous.contains("| fresh |"));
    assert!(
        !std::fs::read_to_string(results.join("report.md"))
            .unwrap()
            .contains("old")
    );
}

#[test]
fn a_directory_that_cannot_be_made_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");

    std::fs::write(&file, "").unwrap();
    assert!(Report::open(&file.join("results"), "T").is_err());
}

#[test]
fn a_failed_write_is_remembered() {
    let dir = tempfile::tempdir().unwrap();
    let results = dir.path().join("results");
    let mut report = Report::open(&results, "T").unwrap();

    std::fs::remove_dir_all(&results).unwrap();
    report.add("x", Kind::Auto, Status::Ok, "");
    report.raw("x", "");
    assert!(report.broken().unwrap().contains("report.md"));
}

#[test]
fn a_failed_raw_write_is_remembered_too() {
    let dir = tempfile::tempdir().unwrap();
    let results = dir.path().join("results");
    let mut report = Report::open(&results, "T").unwrap();

    std::fs::remove_dir_all(results.join("raw")).unwrap();
    report.raw("x", "");
    assert!(report.broken().unwrap().contains("raw"));
}

#[test]
fn an_earlier_report_that_cannot_move_aside_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let results = dir.path().join("results");

    Report::open(&results, "First").unwrap();
    // A file where the old results should go: it cannot be removed as a directory.
    std::fs::write(dir.path().join("results.previous"), "").unwrap();
    assert!(Report::open(&results, "Second").is_err());
}

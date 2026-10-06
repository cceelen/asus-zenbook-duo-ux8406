//! Each table pairs a text with what must come of it; most entries are
//! texts that have to be rejected.

use asus_ux8406_tcc_guard::input::*;
use rstest::rstest;

// What the limits and defaults are meant to be, written out.
const TEXT_SIZE: usize = 96;
const PATH_SIZE: usize = 160;
const TJMAX: (i32, i32) = (70, 130);
const EVENT_TEXT: &str = "asus_wmi: Unknown key code 0x6d";

#[test]
fn numbers() {
    let accepted = [
        ("0", 0),
        ("87", 87),
        ("-40", -40),
        ("100", 100),
        ("-100", -100),
        ("007", 7),
        ("-0", 0),
    ];
    let rejected = [
        "101",
        "-101",
        "",
        "-",
        "+5",
        " 5",
        "5 ",
        "5\n",
        "5x",
        "0x10",
        "1e2",
        "1.5",
        "--5",
        "5-",
        "eighty",
        "1234567890", // ten digits
        "99999999999999999999999999",
    ];

    for (text, value) in accepted {
        assert_eq!(parse_int(text, -100, 100), Some(value), "{text:?}");
    }
    for text in rejected {
        assert_eq!(parse_int(text, -100, 100), None, "{text:?}");
    }
    // The longest numbers there are.
    assert_eq!(parse_int("999999999", 0, i32::MAX), Some(999_999_999));
    assert_eq!(parse_int("-999999999", i32::MIN, 0), Some(-999_999_999));
}

/// Run one table of sysfs values through its parser.
fn run(parse: fn(&str) -> Option<i32>, cases: &[(&str, Option<i32>)]) {
    for (text, value) in cases {
        assert_eq!(parse(text), *value, "{text:?}");
    }
}

#[test]
fn temperatures() {
    run(
        parse_temp,
        &[
            ("61000\n", Some(61)),
            ("61999\n", Some(61)),
            ("0\n", Some(0)),
            ("-40000\n", Some(-40)),
            ("150000\n", Some(150)),
            ("95000", Some(95)),
            ("95000\n\n", Some(95)),
            ("150001\n", None),
            ("-40001\n", None),
            ("\n", None),
            ("", None),
            ("95000\n1\n", None),
            ("95000 \n", None),
            ("garbage\n", None),
        ],
    );
}

#[test]
fn offsets() {
    run(
        parse_offset,
        &[
            ("0\n", Some(0)),
            ("10\n", Some(10)),
            ("63\n", Some(63)),
            ("64\n", None),
            ("-1\n", None),
            ("10 20\n", None),
        ],
    );
}

#[test]
fn limits() {
    run(
        parse_tjmax,
        &[
            ("105000\n", Some(105)),
            ("70000\n", Some(70)),
            ("130000\n", Some(130)),
            ("69999\n", None),
            ("130001\n", None),
            ("105\n", None),
        ],
    );
}

#[test]
fn labels() {
    run(
        parse_package_label,
        &[
            ("Package id 0\n", Some(0)),
            ("Package id 1\n", Some(1)),
            ("Package id 1023\n", Some(1023)),
            ("Package id 1024\n", None),
            ("Core 0\n", None),
            ("Package id \n", None),
            ("Package id -1\n", None),
            ("package id 0\n", None),
            ("Package id 0 extra\n", None),
        ],
    );
}

#[test]
fn kernel_log_records() {
    let cases = [
        // As the kernel delivers it.
        ("6,1234,5678901,-;asus_wmi: Unknown key code 0x6d\n", true),
        (
            "6,1234,5678901,-,caller=T1;asus_wmi: Unknown key code 0x6d\n SUBSYSTEM=platform\n",
            true,
        ),
        ("4,1,1,-;asus_wmi: Unknown key code 0x6d\n", true),
        // Another key code, or the text further inside the message.
        ("6,1,1,-;asus_wmi: Unknown key code 0x6e\n", false),
        ("6,1,1,-;x asus_wmi: Unknown key code 0x6d\n", false),
        ("6,1,1,-;asus_wmi: Unknown key code 0x6\n", false),
        // Written by user space: facility 1 and above.
        ("14,1,1,-;asus_wmi: Unknown key code 0x6d\n", false),
        ("30,1,1,-;asus_wmi: Unknown key code 0x6d\n", false),
        // The text only in the lines after the message.
        (
            "6,1,1,-;something else\n DEVICE=asus_wmi: Unknown key code 0x6d\n",
            false,
        ),
        // Not a record at all.
        ("asus_wmi: Unknown key code 0x6d\n", false),
        (";asus_wmi: Unknown key code 0x6d\n", false),
        ("6;asus_wmi: Unknown key code 0x6d\n", false),
        ("6,1,1,-asus_wmi: Unknown key code 0x6d\n", false),
        ("-6,1,1,-;asus_wmi: Unknown key code 0x6d\n", false),
        ("999999,1,1,-;asus_wmi: Unknown key code 0x6d\n", false),
        ("", false),
    ];

    for (record, event) in cases {
        assert_eq!(is_event(EVENT_TEXT, record), event, "{record:?}");
    }
    assert!(!is_event("", "6,1,1,-;anything\n"), "empty event text");
}

/// The file below `/sys` where a package's offset is written.
#[rstest]
#[case::outside_sys("/etc/passwd")]
#[case::sys_itself("/sys")]
#[case::sys_and_a_slash("/sys/")]
#[case::only_looks_like_sys("/sysfoo/a")]
#[case::relative("sys/a")]
#[case::climbs_out("/sys/a/../../etc/passwd")]
#[case::a_blank_in_the_name("/sys/a b")]
#[case::a_character_outside_the_allowed_set("/sys/a;b")]
#[case::not_ascii("/sys/gr\u{f6}sse")]
#[case::empty("")]
fn offset_files_that_are_refused(#[case] path: &str) {
    let mut settings = Settings::default();

    assert!(settings.add_control(0, path).is_err());
    assert_eq!(settings.controls, []);
}

#[test]
fn offset_files_are_kept_without_the_sys_in_front() {
    let mut settings = Settings::default();

    settings
        .add_control(1, "/sys/devices/a-b/c_d:0.1/offset")
        .unwrap();
    assert_eq!(
        settings.controls,
        [Control {
            package: 1,
            path: "/devices/a-b/c_d:0.1/offset".to_owned()
        }]
    );
    assert!(settings.valid());
}

#[rstest]
#[case(-1)]
#[case(1024)]
#[case(i64::from(i32::MAX) + 1)]
#[case(i64::MIN)]
fn package_numbers_that_are_refused(#[case] package: i64) {
    assert!(Settings::default().add_control(package, "/sys/a").is_err());
}

/// What depends on more than one package.
#[test]
fn packages_together() {
    let mut settings = Settings::default();

    settings.add_control(0, "/sys/a").unwrap();
    assert!(
        settings.add_control(0, "/sys/b").is_err(),
        "second file for a package"
    );
    for package in 1..8 {
        settings.add_control(package, "/sys/a").unwrap();
    }
    assert_eq!(settings.controls.len(), MAX_PACKAGES);
    assert!(
        settings.add_control(99, "/sys/a").is_err(),
        "one package too many"
    );
    assert!(settings.add_control(1023, "/sys/a").is_err());
}

/// The longest texts fit; one more character does not.
#[test]
fn the_longest_texts() {
    let mut settings = Settings::default();
    let text = "x".repeat(TEXT_SIZE - 1);
    let path = "a".repeat(PATH_SIZE - 2);

    assert!(settings.set_event_text(&text).is_ok());
    assert_eq!(settings.event_text, text);
    assert!(settings.set_event_text(&format!("{text}x")).is_err());
    assert!(settings.set_model(&text).is_ok());
    assert!(settings.set_model(&format!("{text}x")).is_err());
    // A file below /sys: 159 characters after "/sys".
    assert!(settings.add_control(0, &format!("/sys/{path}")).is_ok());
    assert!(
        Settings::default()
            .add_control(0, &format!("/sys/{path}a"))
            .is_err()
    );
}

#[rstest]
#[case::empty("")]
#[case::a_newline("two\nlines")]
#[case::a_tab("a\tb")]
#[case::a_carriage_return("a\r")]
#[case::a_control_character("a\u{1}")]
#[case::the_last_one("a\u{7f}")]
#[case::not_ascii("gr\u{f6}sse")]
fn texts_that_are_refused(#[case] text: &str) {
    let mut settings = Settings::default();

    assert!(settings.set_event_text(text).is_err());
    assert!(settings.set_model(text).is_err());
    assert_eq!(settings, Settings::default(), "left as they were");
}

#[test]
fn texts_are_taken_as_they_are() {
    let mut settings = Settings::default();

    settings.set_event_text("acme: too = hot #1").unwrap();
    assert_eq!(settings.event_text, "acme: too = hot #1");
    settings.set_model("any").unwrap();
    assert_eq!(settings.model, ANY_MODEL);
}

/// A setting given twice, as on the command line: the last one counts, and
/// the ranges hold.
#[test]
fn numbers_are_set_within_their_ranges() {
    let mut settings = Settings::default();

    assert!(Number::HoldTemp.set(&mut settings, 90).is_ok());
    assert!(Number::HoldTemp.set(&mut settings, 91).is_ok());
    assert_eq!(settings.guard.hold_temp, 91);
    for refused in [200, 49, 106, -87, i64::MAX, i64::MIN] {
        let rejected = Number::HoldTemp.set(&mut settings, refused).unwrap_err();

        assert_eq!(rejected.to_string(), "hold_temp: has to be from 50 to 105");
    }
    assert_eq!(settings.guard.hold_temp, 91, "left as it was");
    assert!(
        Number::MaxOffset.set(&mut settings, 64).is_err(),
        "more than the register holds"
    );
    assert!(
        Number::Interval.set(&mut settings, 0).is_err(),
        "would spin"
    );
    assert!(
        Number::ProbeBackoff.set(&mut settings, 0).is_err(),
        "would divide by zero"
    );
    assert_eq!(Number::find("event_text"), None);
}

#[test]
fn settings_together() {
    let defaults = Settings::default();

    assert!(defaults.valid());
    for number in Number::ALL {
        let (min, max) = number.range();

        assert!(
            (min..=max).contains(&number.get(&defaults)),
            "default of {} within its range",
            number.name()
        );
        assert_eq!(Number::find(number.name()), Some(number));

        // Each name sets its own value, and no other.
        let mut settings = defaults.clone();
        assert!(number.set(&mut settings, i64::from(max)).is_ok());
        assert_eq!(number.get(&settings), max);
        for other in Number::ALL {
            if other != number {
                assert_eq!(other.get(&settings), other.get(&defaults));
            }
        }
        assert!(number.set(&mut settings, i64::from(max) + 1).is_err());
        assert!(number.set(&mut settings, i64::from(min) - 1).is_err());
        assert_eq!(number.get(&settings), max, "left as it was");
    }

    let mut settings = defaults.clone();
    settings.guard.probe_interval = settings.guard.interval - 1;
    assert!(!settings.valid(), "probe_interval below interval");

    let mut settings = defaults.clone();
    settings.guard.probe_max = settings.guard.probe_interval - 1;
    assert!(!settings.valid(), "probe_max below probe_interval");

    let mut settings = defaults.clone();
    settings.tjmax = TJMAX.0 - 1;
    assert!(!settings.valid(), "tjmax neither 0 nor plausible");
    settings.tjmax = TJMAX.0;
    assert!(settings.valid(), "lowest tjmax");

    let mut settings = defaults;
    settings.event_text.clear();
    assert!(!settings.valid(), "no event text");
}

#[test]
fn the_model_has_to_be_in_the_product_name() {
    let mut settings = Settings::default();

    assert!(settings.model_matches("ASUS Zenbook Duo UX8406CA_UX8406CA"));
    assert!(settings.model_matches("ASUS Zenbook Duo UX8406MA_UX8406MA"));
    assert!(!settings.model_matches("ASUS Zenbook 14 UX3405MA_UX3405MA"));
    assert!(!settings.model_matches(""));
    settings.model = ANY_MODEL.to_owned();
    assert!(settings.model_matches("anything"));
}

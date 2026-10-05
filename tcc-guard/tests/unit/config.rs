//! The configuration file.

use asus_ux8406_tcc_guard::config::{apply, load};
use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::input::{Control, Settings};
use rstest::rstest;

use crate::common::{CONFIG, CONFIG_TWO, Machine, empty, machine};

/// A configuration file is at most this big.
const SIZE_MAX: usize = 16_384;

fn accepted(text: &str) -> Settings {
    let mut settings = Settings::default();

    apply(&mut settings, text).unwrap_or_else(|rejected| panic!("{text:?}: {rejected}"));
    settings
}

#[rstest]
fn a_file_sets_what_it_names(machine: Machine) {
    let mut settings = Settings::default();

    load(&mut settings, &machine.join(CONFIG)).unwrap();
    assert_eq!(settings.guard.interval, 1);
    assert_eq!(settings.guard.settle, 2);
    assert_eq!(settings.event_text, "test: thermal warning");
    assert_eq!(
        settings.guard.hold_temp,
        Settings::default().guard.hold_temp
    );
}

#[rstest]
#[case::empty("")]
#[case::blank("   \t \n\n")]
#[case::a_comment("# hold_temp = 1\n")]
fn a_file_that_names_nothing_changes_nothing(#[case] text: &str) {
    assert_eq!(accepted(text), Settings::default());
}

#[test]
fn every_setting_can_be_named() {
    let settings = accepted(
        "hold_temp = 85\n\tstep_up=3  # three\nmax_offset = 20\ninterval = 2\nsettle = 10\n\
         probe_interval = 60\nprobe_backoff = 3\nprobe_max = 600\ntjmax = 100\n\
         use_margin = 4\nidle_release = 300\nevent_text = \"acme: too = hot \"\n\
         model = 'any'\n\n[[package]]\nid = 1\noffset = \"/sys/devices/a-b/c_d:0.1/offset\"",
    );
    let guard = settings.guard;

    assert_eq!(
        (
            guard.hold_temp,
            guard.step_up,
            guard.max_offset,
            guard.interval,
            guard.settle
        ),
        (85, 3, 20, 2, 10)
    );
    assert_eq!(
        (guard.probe_interval, guard.probe_backoff, guard.probe_max),
        (60, 3, 600)
    );
    assert_eq!(
        (settings.tjmax, guard.use_margin, guard.idle_release),
        (100, 4, 300)
    );
    assert_eq!(settings.event_text, "acme: too = hot ");
    assert_eq!(settings.model, "any");
    assert_eq!(
        settings.controls,
        [Control {
            package: 1,
            path: "/devices/a-b/c_d:0.1/offset".to_owned()
        }]
    );
    assert!(settings.valid());
}

#[test]
fn two_packages() {
    let settings = accepted(CONFIG_TWO);

    assert_eq!(settings.controls.len(), 2);
    assert_eq!(settings.controls[1].package, 1);
    assert_eq!(
        settings.controls[1].path,
        "/bus/pci/devices/0000:80:04.0/tcc_offset_degree_celsius"
    );
}

/// Each for the reason beside it.
#[rstest]
#[case::no_value("hold_temp\n")]
#[case::an_empty_value("hold_temp =\n")]
#[case::no_name("= 85\n")]
#[case::no_equals_sign("hold_temp 85\n")]
#[case::two_values("hold_temp = 85 86\n")]
#[case::a_unit("hold_temp = 85C\n")]
#[case::a_word("hold_temp = eighty\n")]
#[case::a_text_for_a_number("hold_temp = \"85\"\n")]
#[case::a_fraction("hold_temp = 85.0\n")]
#[case::a_truth_value("hold_temp = true\n")]
#[case::below_its_range("hold_temp = 49\n")]
#[case::above_its_range("hold_temp = 106\n")]
#[case::negative("hold_temp = -87\n")]
#[case::beyond_any_number("hold_temp = 99999999999999999999999999\n")]
#[case::more_than_the_register_holds("max_offset = 64\n")]
#[case::would_spin("interval = 0\n")]
#[case::would_divide_by_zero("probe_backoff = 0\n")]
#[case::names_are_exact("Hold_Temp = 85\n")]
#[case::a_blank_in_the_name("hold temp = 85\n")]
#[case::an_unknown_name("release_temp = 80\n")]
#[case::a_control_character("hold_temp = 85\u{1}\n")]
#[case::the_last_one("hold_temp = 85\u{7f}\n")]
#[case::not_ascii("hold_temp = 8\u{e5}\n")]
#[case::a_name_given_twice("settle = 5\nsettle = 6\n")]
#[case::a_text_given_twice("event_text = \"a\"\nevent_text = \"b\"\n")]
#[case::an_empty_text("event_text = \"\"\n")]
#[case::an_empty_model("model = \"\"\n")]
#[case::a_number_for_a_text("event_text = 5\n")]
#[case::a_text_that_is_not_plain("event_text = \"gr\u{f6}sse\"\n")]
#[case::a_text_of_two_lines("event_text = \"\"\"a\nb\"\"\"\n")]
#[case::a_text_without_its_end("event_text = \"a\n")]
#[case::a_table_that_is_not_known("[guard]\nhold_temp = 85\n")]
#[case::package_as_a_value("package = \"0 /sys/a\"\n")]
#[case::a_package_without_a_file("[[package]]\nid = 0\n")]
#[case::a_package_without_a_number("[[package]]\noffset = \"/sys/a\"\n")]
#[case::a_package_number_that_is_none("[[package]]\nid = \"x\"\noffset = \"/sys/a\"\n")]
#[case::a_package_number_out_of_range("[[package]]\nid = 1024\noffset = \"/sys/a\"\n")]
#[case::a_package_with_something_else(
    "[[package]]\nid = 0\noffset = \"/sys/a\"\ntemp = \"/sys/b\"\n"
)]
#[case::an_offset_file_outside_sys("[[package]]\nid = 0\noffset = \"/etc/passwd\"\n")]
#[case::an_offset_file_that_climbs_out(
    "[[package]]\nid = 0\noffset = \"/sys/a/../../etc/passwd\"\n"
)]
#[case::a_package_named_twice(
    "[[package]]\nid = 0\noffset = \"/sys/a\"\n[[package]]\nid = 0\noffset = \"/sys/b\"\n"
)]
fn what_is_refused(#[case] text: &str) {
    let mut settings = Settings::default();

    assert!(
        apply(&mut settings, text).is_err(),
        "{text:?} was not refused"
    );
}

#[test]
fn a_refusal_says_what_is_wrong() {
    let refusal = |text: &str| {
        apply(&mut Settings::default(), text)
            .unwrap_err()
            .to_string()
    };

    assert_eq!(
        refusal("hold_temp = 200"),
        "hold_temp: has to be from 50 to 105"
    );
    assert_eq!(
        refusal("model = \"\""),
        "model: has to be 1 to 95 printable ASCII characters"
    );
    assert!(refusal("interval = 1\nrelease_temp = 80\n").contains("line 2"));
    assert!(refusal("interval = 1\nrelease_temp = 80\n").contains("release_temp"));
    assert!(
        refusal("[[package]]\nid = 3\noffset = \"/etc/passwd\"")
            .starts_with("package 3: its offset has to be a file below /sys")
    );
}

#[test]
fn nine_packages_are_one_too_many() {
    let table = |id: usize| format!("[[package]]\nid = {id}\noffset = \"/sys/a\"\n");
    let eight: String = (0..8).map(table).collect();

    assert_eq!(accepted(&eight).controls.len(), 8);
    assert!(apply(&mut Settings::default(), &(eight + &table(8))).is_err());
}

#[rstest]
fn files_that_are_not_read(empty: Machine) {
    let big = empty.put("big", "# filler\n".repeat(SIZE_MAX / 9 + 1));
    let fits = empty.put("fits", "#".repeat(SIZE_MAX));
    let bad = empty.put("bad", "interval = 1\nbogus\n");
    let not_text = empty.put("not-text", b"# gr\xe5sse\n");
    let mut settings = Settings::default();

    assert!(matches!(
        load(&mut settings, &big),
        Err(Error::NotTrusted(_))
    ));
    assert!(load(&mut settings, &fits).is_ok());
    assert!(
        load(&mut settings, &empty.join("none"))
            .unwrap_err()
            .is_missing()
    );
    assert!(matches!(
        load(&mut settings, &bad),
        Err(Error::BadConfig { path, reason }) if path == bad && reason.contains("line 2")
    ));
    assert!(matches!(
        load(&mut settings, &not_text),
        Err(Error::BadConfig { .. })
    ));
    assert_eq!(settings, Settings::default());
}

/// The example that is installed with the program states the defaults.
#[test]
fn the_example_file_holds_the_defaults() {
    let example = include_str!("../../asus-ux8406-tcc-guard.toml.example");

    assert_eq!(accepted(example), Settings::default());
    // And its commented-out part is acceptable once the comment signs go.
    let with_packages = example
        .replace("\n#[[package]]", "\n[[package]]")
        .replace("\n#id = ", "\nid = ")
        .replace("\n#offset = ", "\noffset = ");
    assert_eq!(accepted(&with_packages).controls.len(), 2);
}

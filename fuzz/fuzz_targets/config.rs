//! The configuration file. Only the administrator can change it, but what it
//! holds is either refused or gives settings within their rules; it never
//! crashes the guard.

#![no_main]

use std::fmt::Write;

use asus_ux8406_tcc_guard::config;
use asus_ux8406_tcc_guard::input::{MAX_PACKAGES, Number, Settings};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let mut settings = Settings::default();

    if let Err(rejected) = config::apply(&mut settings, text) {
        assert!(!rejected.0.is_empty(), "a refusal says why");
        return;
    }

    for number in Number::ALL {
        let (min, max) = number.range();
        assert!(
            (min..=max).contains(&number.get(&settings)),
            "{}",
            number.name()
        );
    }
    for value in [&settings.event_text, &settings.model] {
        assert!(!value.is_empty() && value.bytes().all(|b| (b' '..=b'~').contains(&b)));
    }
    assert!(settings.controls.len() <= MAX_PACKAGES);
    for (n, control) in settings.controls.iter().enumerate() {
        assert!(control.path.starts_with('/') && !control.path.contains(".."));
        assert!(
            settings.controls[..n]
                .iter()
                .all(|other| other.package != control.package)
        );
    }

    // The settings, written as a file, read back as the same settings.
    let mut again = Settings::default();
    config::apply(&mut again, &file(&settings)).expect("the written file is taken");
    assert_eq!(again, settings);
});

/// A configuration file that names every setting.
fn file(settings: &Settings) -> String {
    let mut text = String::new();

    for number in Number::ALL {
        writeln!(text, "{} = {}", number.name(), number.get(settings)).unwrap();
    }
    writeln!(text, "event_text = {}", quoted(&settings.event_text)).unwrap();
    writeln!(text, "model = {}", quoted(&settings.model)).unwrap();
    for control in &settings.controls {
        let offset = quoted(&format!("/sys{}", control.path));
        writeln!(
            text,
            "[[package]]\nid = {}\noffset = {offset}",
            control.package
        )
        .unwrap();
    }
    text
}

/// A TOML string of printable ASCII.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

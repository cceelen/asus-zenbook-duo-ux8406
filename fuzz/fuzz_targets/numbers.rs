//! The numbers from sysfs: temperatures, limits, offsets and the labels of
//! the package sensors. A driver, or a file system laid over /sys, decides
//! what they hold; a number the guard takes is one it can trust.

#![no_main]

use asus_ux8406_tcc_guard::input;
use libfuzzer_sys::fuzz_target;

/// Ranges as the parsers check them, and some around their edges.
const RANGES: [(i32, i32); 6] = [
    (-40_000, 150_000),
    (70_000, 130_000),
    (0, 63),
    (0, 1023),
    (-5, 5),
    (i32::MIN, 0),
];

fuzz_target!(|text: &str| {
    // The rule of parse_int: an optional minus sign and one to nine digits,
    // and nothing else. Such a text means what Rust reads in it.
    let digits = text.strip_prefix('-').unwrap_or(text);
    let plain = (1..=9).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit());
    let any = input::parse_int(text, i32::MIN, i32::MAX);
    assert_eq!(
        any,
        plain.then(|| text.parse::<i32>().expect("a plain number"))
    );

    // A range takes exactly the numbers within it.
    for (min, max) in RANGES {
        let within = any.filter(|value| (min..=max).contains(value));
        assert_eq!(input::parse_int(text, min, max), within);
    }

    // What the sysfs parsers give is within their ranges (degrees).
    let checks = [
        (input::parse_temp(text), -40, 150),
        (input::parse_tjmax(text), 70, 130),
        (input::parse_offset(text), 0, input::OFFSET_MAX),
        (input::parse_package_label(text), 0, 1023),
    ];
    for (value, min, max) in checks {
        assert!(value.is_none_or(|value| (min..=max).contains(&value)));
    }
});

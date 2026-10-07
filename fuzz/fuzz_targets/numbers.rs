//! The numbers from sysfs and from the configuration.

#![no_main]

use asus_ux8406_tcc_guard::input;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The first eight bytes give the range, the rest is the text.
    let Some((range, text)) = data.split_first_chunk::<8>() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(text) else {
        return;
    };
    let a = i32::from_le_bytes([range[0], range[1], range[2], range[3]]);
    let b = i32::from_le_bytes([range[4], range[5], range[6], range[7]]);
    let (min, max) = (a.min(b), a.max(b));

    // A number that is taken is in the range and is what Rust reads.
    if let Some(value) = input::parse_int(text, min, max) {
        assert!((min..=max).contains(&value));
        assert_eq!(text.parse::<i32>().ok(), Some(value));
    }

    let _ = input::parse_temp(text);
    let _ = input::parse_tjmax(text);
    let _ = input::parse_offset(text);
    let _ = input::parse_package_label(text);
});

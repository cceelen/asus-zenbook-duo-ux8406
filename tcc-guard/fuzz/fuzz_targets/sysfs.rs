//! sysfs values: temperatures, limits, offsets and sensor labels.
#![no_main]

use asus_ux8406_tcc_guard::input::{
    parse_int, parse_offset, parse_package_label, parse_temp, parse_tjmax,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let _ = parse_int(text, i32::MIN, i32::MAX);
    let _ = parse_temp(text);
    let _ = parse_tjmax(text);
    let _ = parse_offset(text);
    let _ = parse_package_label(text);
});

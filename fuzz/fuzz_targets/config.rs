//! The text of the configuration file.

#![no_main]

use asus_ux8406_tcc_guard::config;
use asus_ux8406_tcc_guard::input::Settings;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = config::apply(&mut Settings::default(), text);
    }
});

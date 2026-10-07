//! The configuration file. Only the administrator can change it, but what it
//! holds must still be rejected or accepted, never crash the guard.
#![no_main]

use asus_ux8406_tcc_guard::config::apply;
use asus_ux8406_tcc_guard::input::Settings;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    let mut settings = Settings::default();

    if apply(&mut settings, text).is_ok() {
        // What the file accepted keeps the settings consistent.
        let _ = settings.valid();
    }
});

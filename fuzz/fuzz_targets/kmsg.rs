//! Kernel log records. Any process that may write to /dev/kmsg puts text in
//! front of the guard, which runs as root: no record may crash it.

#![no_main]

use std::io::Cursor;

use asus_ux8406_tcc_guard::input::{self, Settings};
use asus_ux8406_tcc_guard::kmsg;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // As the guard reads the log: one read gives one record, which is_event
    // sees through String::from_utf8_lossy.
    let event_text = Settings::default().event_text;
    kmsg::scan(&mut Cursor::new(data), &event_text).expect("a slice reads");

    // An event text of the configuration's, up to the first NUL byte, and a
    // record after it.
    if let Some(at) = data.iter().position(|&byte| byte == 0) {
        let event_text = String::from_utf8_lossy(&data[..at]);
        let record = String::from_utf8_lossy(&data[at + 1..]);

        if input::is_event(&event_text, &record) {
            assert!(!event_text.is_empty() && record.contains(&*event_text));
        }
    }
});

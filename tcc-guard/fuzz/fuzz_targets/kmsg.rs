//! Kernel log records: what any process that can write to /dev/kmsg, and the
//! kernel, put in front of the guard.
#![no_main]

use std::io::Cursor;

use asus_ux8406_tcc_guard::input::{Settings, is_event};
use asus_ux8406_tcc_guard::kmsg::scan;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let event_text = Settings::default().event_text;

    let _ = is_event(&event_text, &String::from_utf8_lossy(data));
    // A reader gives as much as fits in one read, as /dev/kmsg gives one
    // record per read.
    let _ = scan(&mut Cursor::new(data), &event_text);
});

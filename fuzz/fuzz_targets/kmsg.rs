//! Records of the kernel log, as /dev/kmsg delivers them.

#![no_main]

use std::io::Cursor;

use asus_ux8406_tcc_guard::input::{self, Settings};
use asus_ux8406_tcc_guard::kmsg;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The log with the usual event text.
    let event_text = Settings::default().event_text;
    kmsg::scan(&mut Cursor::new(data), &event_text).expect("a slice reads");

    // One record with an event text of its own: the first byte says where
    // the text ends.
    let Some((&split, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let split = usize::from(split).min(text.len());
    if let (Some(event_text), Some(record)) = (text.get(..split), text.get(split..)) {
        let _ = input::is_event(event_text, record);
    }
});

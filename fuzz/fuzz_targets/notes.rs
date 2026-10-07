//! The notes of the offsets found (store.rs). They are in a directory that
//! only the administrator can change, but the guard reads them as root after
//! a crash: what a note holds is either an offset or refused, never a crash.

#![no_main]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::LazyLock;

use asus_ux8406_tcc_guard::error::Error;
use asus_ux8406_tcc_guard::input;
use asus_ux8406_tcc_guard::store::Store;
use asus_ux8406_tcc_guard::sysfs::VALUE_MAX;
use libfuzzer_sys::fuzz_target;

/// One directory of notes for the whole run, emptied for each input.
static DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = std::env::temp_dir().join(format!("asus-ux8406-fuzz-notes-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    Store::new(&dir)
        .prepare()
        .expect("a directory for the notes");
    dir
});

fuzz_target!(|data: &[u8]| {
    // The name of the note up to the first NUL byte, what it holds after it.
    let Some(at) = data.iter().position(|&byte| byte == 0) else {
        return;
    };
    let (name, text) = (&data[..at], &data[at + 1..]);
    if name.is_empty() || name.len() > 255 || name.contains(&b'/') || name == b"." || name == b".."
    {
        return;
    }

    for entry in fs::read_dir(&*DIR).expect("the directory of the notes") {
        fs::remove_file(entry.expect("a note").path()).expect("a note removed");
    }
    fs::write(DIR.join(OsStr::from_bytes(name)), text).expect("a note written");

    // Only the names the guard gives count: "package-" and the number.
    let package = std::str::from_utf8(name)
        .ok()
        .and_then(|name| name.strip_prefix("package-"))
        .and_then(|number| input::parse_int(number, 0, 1023))
        .filter(|package| name == format!("package-{package}").as_bytes());
    let offset = std::str::from_utf8(text).ok().and_then(input::parse_offset);

    match (package, Store::new(&DIR).saved()) {
        (None, Ok(saved)) => assert_eq!(saved, []),
        (Some(_), Err(Error::NotTrusted(_))) => assert!(text.len() > VALUE_MAX),
        (Some(package), Ok(saved)) => {
            assert!(text.len() <= VALUE_MAX);
            assert_eq!(saved, [(package, offset.expect("an offset"))]);
        }
        (Some(_), Err(Error::BadValue(_))) => assert!(text.len() <= VALUE_MAX && offset.is_none()),
        (_, result) => panic!("{result:?}"),
    }
});

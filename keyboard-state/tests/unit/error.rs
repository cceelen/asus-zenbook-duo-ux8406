//! What the program says when it cannot do its work.

use std::io;
use std::path::PathBuf;

use asus_ux8406_keyboard_state::Error;
use rstest::rstest;

#[rstest]
#[case::not_the_keyboard(
    Error::NotTheKeyboard("0003:056A:0357.0007".to_owned()),
    "0003:056A:0357.0007: not a HID device of the keyboard"
)]
#[case::bad_state(
    Error::BadState(PathBuf::from("/run/state")),
    "/run/state: not a keyboard state"
)]
#[case::map(
    Error::Map { path: PathBuf::from("/sys/fs/bpf/hid/x/y/map"), reason: "no such map".to_owned() },
    "/sys/fs/bpf/hid/x/y/map: no such map"
)]
#[case::io(
    Error::Io { path: PathBuf::from("/run/state"), source: io::Error::from(io::ErrorKind::PermissionDenied) },
    "/run/state: permission denied"
)]
fn the_message_names_what_it_is_about(#[case] error: Error, #[case] message: &str) {
    assert_eq!(error.to_string(), message);
}

//! The guard against a run on an installed system: the tool installs
//! packages and loads kernel programs, and runs only in the live image.

use std::path::Path;

/// The root file system as `/proc/mounts` lists it: the device and the type.
fn root_mount(mounts: &str) -> Option<(&str, &str)> {
    mounts
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();

            match (fields.next(), fields.next(), fields.next()) {
                (Some(device), Some("/"), Some(kind)) => Some((device, kind)),
                _ => None,
            }
        })
        .next_back()
}

/// Whether the kernel command line is that of a live image.
fn live_command_line(command_line: &str) -> bool {
    command_line
        .split_whitespace()
        .any(|word| word == "rd.live.image" || word.starts_with("root=live:"))
}

/// Whether the root file system is the overlay of the live image: an
/// overlay, or the snapshot device of dracut's live module.
fn live_root(device: &str, kind: &str) -> bool {
    kind == "overlay" || device.starts_with("/dev/mapper/live-") || device.contains("LiveOS")
}

/// Check that the tool runs in a live image.
///
/// # Errors
///
/// What is not as in a live image.
pub fn check(root: &Path) -> Result<(), String> {
    let command_line = std::fs::read_to_string(root.join("proc/cmdline")).unwrap_or_default();
    let mounts = std::fs::read_to_string(root.join("proc/mounts")).unwrap_or_default();

    if !live_command_line(&command_line) {
        return Err("the kernel command line has no rd.live.image".to_owned());
    }
    match root_mount(&mounts) {
        Some((device, kind)) if live_root(device, kind) => Ok(()),
        Some((device, kind)) => Err(format!(
            "the root file system is {device} ({kind}), not the overlay of the live image"
        )),
        None => Err("no root file system in /proc/mounts".to_owned()),
    }
}

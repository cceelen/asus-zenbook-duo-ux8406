//! The checks that need no action of the tester.

use std::collections::BTreeMap;
use std::time::Duration;

use asus_ux8406_kde_common::geometry::is_valid;
use asus_ux8406_kde_common::output::{Policy, Session};

use crate::activate::{self, DAEMONS, Daemon};
use crate::context::Ctx;
use crate::dbus;
use crate::outputs::{self, until};
use crate::report::{Kind, Status};

/// The connector of the upper panel.
pub const UPPER: &str = "eDP-1";
/// The connector of the lower panel.
pub const LOWER: &str = "eDP-2";
/// The vendor of the ELAN touch controllers.
const ELAN: u32 = 0x04f3;

/// When a check that runs twice runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Before the tool changes anything: the state as the image booted.
    Booted,
    /// After the tool activated the packages. `booted` says that the
    /// packages were in the image at the start.
    Activated {
        /// Whether the packages were in the image at the start.
        booted: bool,
    },
}

impl Phase {
    /// The name of a row in this phase: the later one has a suffix, so that
    /// no two rows have the same name.
    #[must_use]
    pub fn row(self, check: &str) -> String {
        match self {
            Self::Booted => check.to_owned(),
            Self::Activated { .. } => format!("{check} (after activation)"),
        }
    }
}

/// Remove the colour codes of a terminal from a text.
#[must_use]
pub fn strip_ansi(text: &str) -> String {
    let mut clean = String::new();
    let mut characters = text.chars();

    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            // A colour code ends with a letter.
            for next in characters.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            clean.push(character);
        }
    }
    clean
}

/// The UUID of each output in the text of `kscreen-doctor -o`: the lines
/// `Output: 1 eDP-1 <uuid>`.
#[must_use]
pub fn uuids(text: &str) -> BTreeMap<String, String> {
    strip_ansi(text)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Output:"))
        .filter_map(|rest| {
            let words: Vec<&str> = rest.split_whitespace().collect();

            match words.as_slice() {
                [_, name, uuid, ..] => Some(((*name).to_owned(), (*uuid).to_owned())),
                _ => None,
            }
        })
        .collect()
}

/// Both panels are there, built-in, with their own UUIDs, in a valid layout.
pub fn panels(ctx: &mut Ctx<'_>) {
    let Some(session) = outputs::open(ctx, "built-in panels", Kind::Auto) else {
        return;
    };
    let state = session.outputs.state();
    let doctor = ctx.run("kscreen-doctor", &["-o"]);
    let uuids = uuids(&doctor.stdout);
    let missing: Vec<&str> = [UPPER, LOWER]
        .into_iter()
        .filter(|name| !state.builtin.iter().any(|builtin| builtin == name))
        .collect();
    let layout = outputs::describe(&state);

    let result = if !missing.is_empty() {
        Err(format!(
            "not present as built-in outputs: {}; layout {layout}",
            missing.join(", ")
        ))
    } else if !uuids.contains_key(UPPER) || uuids.get(UPPER) == uuids.get(LOWER) {
        Err(format!(
            "the UUIDs of {UPPER} and {LOWER} are not two different ones in kscreen-doctor -o: {:?} and {:?}",
            uuids.get(UPPER),
            uuids.get(LOWER)
        ))
    } else if !is_valid(&state, &state.logical) {
        Err(format!("the layout has a gap or an overlap: {layout}"))
    } else {
        Ok(format!(
            "{UPPER} and {LOWER} are built-in with different UUIDs; layout {layout}"
        ))
    };

    ctx.auto("eDP-1 and eDP-2 present, built-in, distinct UUIDs", result);
}

/// Whether the program said "ready" in the journal of the user.
fn is_ready(ctx: &Ctx<'_>, daemon: &Daemon) -> bool {
    let journal = ctx.run(
        "journalctl",
        &[
            "--user",
            "-b",
            "--no-pager",
            "-o",
            "cat",
            "-t",
            daemon.ident,
        ],
    );

    journal.stdout.lines().any(|line| line.trim() == "ready")
}

/// Both programs said "ready".
pub fn ready(ctx: &mut Ctx<'_>) {
    for daemon in &DAEMONS {
        let timing = ctx.timing;
        let found = until(&timing, timing.settle, || is_ready(ctx, daemon));

        ctx.auto(
            &format!("{} says ready", daemon.entry),
            if found {
                Ok("the journal has the line \"ready\"".to_owned())
            } else {
                Err("no \"ready\" line in the journal; look at raw/".to_owned())
            },
        );
    }
}

/// The names of the actions that a component of kglobalacceld has, by the
/// object path of the component.
fn components(ctx: &Ctx<'_>) -> Result<BTreeMap<String, Vec<String>>, String> {
    let bus = ctx.session_bus()?;
    let paths: Vec<zbus::zvariant::OwnedObjectPath> = dbus::call(
        &bus,
        "org.kde.kglobalaccel",
        "/kglobalaccel",
        "org.kde.KGlobalAccel",
        "allComponents",
        &(),
    )?;

    paths
        .into_iter()
        .map(|path| {
            // Some versions take the context as an argument.
            dbus::call::<_, Vec<String>>(
                &bus,
                "org.kde.kglobalaccel",
                path.as_str(),
                "org.kde.kglobalaccel.Component",
                "shortcutNames",
                &(),
            )
            .or_else(|_| {
                dbus::call::<_, Vec<String>>(
                    &bus,
                    "org.kde.kglobalaccel",
                    path.as_str(),
                    "org.kde.kglobalaccel.Component",
                    "shortcutNames",
                    &("default",),
                )
            })
            .map(|names| (path.as_str().to_owned(), names))
        })
        .collect()
}

/// The section and the keys of a line of `kglobalshortcutsrc` text: the
/// first field of a value is the active key.
#[must_use]
pub fn active_keys(text: &str) -> BTreeMap<(String, String), String> {
    let mut section = String::new();
    let mut keys = BTreeMap::new();

    for line in text.lines() {
        if line.starts_with('[') {
            line.clone_into(&mut section);
        } else if let Some((name, value)) = line.split_once('=') {
            let active = value.split(',').next().unwrap_or_default();

            keys.insert((section.clone(), name.to_owned()), active.to_owned());
        }
    }
    keys
}

/// One shortcut: registered with kglobalacceld, and with this key.
struct Shortcut {
    check: &'static str,
    /// Part of the object path of the component.
    component: &'static str,
    action: &'static str,
    section: &'static str,
    key: &'static str,
    /// Whether kglobalacceld reads it from a file at its start.
    file: bool,
}

const SHORTCUTS: [Shortcut; 4] = [
    Shortcut {
        check: "shortcut: lower screen key",
        component: "kwin",
        action: "ASUS UX8406 Toggle Lower Screen",
        section: "[kwin]",
        key: "Launch (9)",
        file: false,
    },
    Shortcut {
        check: "shortcut: swap windows",
        component: "kwin",
        action: "ASUS UX8406 Swap Windows",
        section: "[kwin]",
        key: "F19",
        file: false,
    },
    Shortcut {
        check: "shortcut: emoji selector",
        component: "asus_zenbook_duo_ux8406_shortcuts",
        action: "EmojiSelector",
        section: "[services][asus-zenbook-duo-ux8406-shortcuts.desktop]",
        key: "Launch (1)",
        file: true,
    },
    Shortcut {
        check: "shortcut: toggle auto-rotate",
        component: "plasma_builtin_screen_rotation",
        action: "ToggleAutoRotate",
        section: "[plasma-builtin-screen-rotation]",
        key: "none",
        file: false,
    },
];

/// The four shortcuts are registered with their keys. kglobalacceld reads
/// the folder of the shortcut files only at its start, which is at the start
/// of the session: where the package was installed at run time, the file
/// of the emoji shortcut is not read yet, and its row is a skip.
pub fn shortcuts(ctx: &mut Ctx<'_>, phase: Phase) {
    let timing = ctx.timing;
    let home = ctx.var("HOME").unwrap_or_default().to_owned();

    // KWin and kglobalacceld write the registrations a moment after the
    // script has run.
    let mut seen = Err("not asked".to_owned());
    let mut keys = BTreeMap::new();

    until(&timing, timing.settle, || {
        seen = components(ctx);
        keys = active_keys(
            &std::fs::read_to_string(format!("{home}/.config/kglobalshortcutsrc"))
                .unwrap_or_default(),
        );
        seen.as_ref().is_ok_and(|names| {
            SHORTCUTS.iter().all(|shortcut| {
                registered(names, shortcut) && key_state(&keys, shortcut) != Some(false)
            })
        })
    });

    for shortcut in &SHORTCUTS {
        if shortcut.file && phase == (Phase::Activated { booted: false }) {
            ctx.record(
                &phase.row(shortcut.check),
                Kind::Auto,
                Status::Skip,
                "the package was installed at run time, and kglobalacceld reads the shortcut files only when the session starts: the row of the section \"As booted\" is the real check",
            );
            continue;
        }

        let result = match &seen {
            Err(reason) => Err(reason.clone()),
            Ok(names) if !registered(names, shortcut) => Err(format!(
                "kglobalacceld has no action {:?} in a component {:?}; components: {}",
                shortcut.action,
                shortcut.component,
                names.keys().cloned().collect::<Vec<_>>().join(" ")
            )),
            Ok(_) => match key_state(&keys, shortcut) {
                Some(false) => Err(format!(
                    "registered, but the key in kglobalshortcutsrc is {:?}, not {:?}",
                    keys.get(&(shortcut.section.to_owned(), shortcut.action.to_owned())),
                    shortcut.key
                )),
                Some(true) => Ok(format!(
                    "{:?} is registered with the key {}",
                    shortcut.action, shortcut.key
                )),
                None => Ok(format!(
                    "{:?} is registered; kglobalshortcutsrc has no entry, so the key is the default of the package ({})",
                    shortcut.action, shortcut.key
                )),
            },
        };

        ctx.auto(&phase.row(shortcut.check), result);
    }
    ctx.raw(
        "kglobalaccel-components.txt",
        &format!("{seen:#?}\n{keys:#?}\n"),
    );
}

fn registered(names: &BTreeMap<String, Vec<String>>, shortcut: &Shortcut) -> bool {
    names.iter().any(|(path, actions)| {
        path.contains(shortcut.component) && actions.iter().any(|action| action == shortcut.action)
    })
}

/// Whether the file has the key of the shortcut; none if it has no entry.
fn key_state(keys: &BTreeMap<(String, String), String>, shortcut: &Shortcut) -> Option<bool> {
    keys.get(&(shortcut.section.to_owned(), shortcut.action.to_owned()))
        .map(|key| key == shortcut.key)
}

/// The ELAN touchscreens and pens that KWin lists.
///
/// # Errors
///
/// KWin does not list its input devices.
pub fn elan_devices(ctx: &Ctx<'_>) -> Result<Vec<asus_ux8406_kde_keys::input::Device>, String> {
    let bus = ctx.session_bus()?;

    asus_ux8406_kde_keys::input::devices(&bus)
        .map(|devices| {
            devices
                .into_iter()
                .filter(|device| device.vendor == ELAN && device.touch)
                .collect()
        })
        .map_err(|error| format!("KWin's input devices: {error}"))
}

/// Whether the devices are mapped as the packages mean: each to a built-in
/// panel, the two controllers to the two panels.
fn mapping(devices: &[asus_ux8406_kde_keys::input::Device]) -> Result<String, String> {
    let mut by_controller: BTreeMap<u32, Vec<&asus_ux8406_kde_keys::input::Device>> =
        BTreeMap::new();

    for device in devices {
        by_controller
            .entry(device.product)
            .or_default()
            .push(device);
    }

    let text = devices
        .iter()
        .map(|device| {
            format!(
                "{} ({}) on {:?}",
                device.sys_name,
                device.id(),
                device.output_name
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let outputs: Vec<&str> = by_controller
        .values()
        .map(|group| group[0].output_name.as_str())
        .collect();
    let split = by_controller.values().any(|group| {
        group
            .iter()
            .any(|device| device.output_name != group[0].output_name)
    });

    if devices.is_empty() {
        Err("KWin lists no ELAN touchscreen or pen (vendor 04f3)".to_owned())
    } else if split {
        Err(format!(
            "the devices of one controller act on different panels: {text}"
        ))
    } else if !outputs.contains(&UPPER) || !outputs.contains(&LOWER) {
        Err(format!(
            "the controllers do not act on {UPPER} and {LOWER}, one each: {text}"
        ))
    } else {
        Ok(text)
    }
}

/// Each ELAN device acts on a panel of its own.
pub fn touch(ctx: &mut Ctx<'_>) {
    let timing = ctx.timing;
    let mut result = Err("not asked".to_owned());

    until(&timing, timing.settle, || {
        result = elan_devices(ctx).and_then(|devices| mapping(&devices));
        result.is_ok()
    });
    ctx.auto("ELAN touch and pen devices act on their panels", result);
}

fn policies_of(session: &Session) -> BTreeMap<String, Policy> {
    session.outputs.policies().into_iter().collect()
}

fn all_are(policies: &BTreeMap<String, Policy>, wanted: impl Fn(Policy) -> bool) -> bool {
    [UPPER, LOWER]
        .iter()
        .all(|name| policies.get(*name).is_some_and(|policy| wanted(*policy)))
}

fn describe_policies(policies: &BTreeMap<String, Policy>) -> String {
    policies
        .iter()
        .map(|(name, policy)| format!("{name}: {}", policy.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Wait until the policies of both panels fit, at most `patience`, and say
/// what they are.
fn settle_policies(
    ctx: &Ctx<'_>,
    session: &mut Session,
    patience: Duration,
    wanted: impl Fn(Policy) -> bool,
) -> BTreeMap<String, Policy> {
    let timing = ctx.timing;

    until(&timing, patience, || {
        let _ = session.dispatch();
        all_are(&policies_of(session), &wanted)
    });
    policies_of(session)
}

/// Report the policy of both panels under `check`; whether both are `never`.
fn never_row(ctx: &mut Ctx<'_>, session: &mut Session, check: &str) -> bool {
    let policies = settle_policies(ctx, session, ctx.timing.settle, |policy| {
        policy == Policy::Never
    });
    let never = all_are(&policies, |policy| policy == Policy::Never);

    ctx.auto(
        check,
        if never {
            Ok(describe_policies(&policies))
        } else {
            Err(format!("not never: {}", describe_policies(&policies)))
        },
    );
    never
}

/// The policy is `never` on both panels, without a change (for the state as booted).
pub fn policy_booted(ctx: &mut Ctx<'_>) {
    if let Some(mut session) =
        outputs::open(ctx, "auto-rotate policy never (as booted)", Kind::Auto)
    {
        never_row(
            ctx,
            &mut session,
            "auto-rotate policy never on both panels (as booted)",
        );
    }
}

/// KWin's auto-rotate policy is `never` on both panels, and a change to `always` is reverted.
pub fn policy_never(ctx: &mut Ctx<'_>) {
    let Some(mut session) = outputs::open(ctx, "auto-rotate policy never", Kind::Auto) else {
        return;
    };

    if !never_row(ctx, &mut session, "auto-rotate policy never on both panels") {
        return;
    }

    let both = [
        (UPPER.to_owned(), Policy::Always),
        (LOWER.to_owned(), Policy::Always),
    ];
    let asked = session
        .set_policies(&both)
        .ok_or("KWin's output management is too old to set the policy".to_owned())
        .and_then(|number| {
            session
                .flush()
                .and_then(|()| session.wait(number))
                .map_err(|error| error.to_string())
                .and_then(|answer| answer.map_err(|reason| format!("KWin refused: {reason}")))
        });

    let result = asked.and_then(|()| {
        let after = settle_policies(ctx, &mut session, ctx.timing.settle, |policy| {
            policy == Policy::Never
        });

        if all_are(&after, |policy| policy == Policy::Never) {
            Ok("set to always over the protocol; the program set both back to never".to_owned())
        } else {
            Err(format!(
                "the program did not revert it: {}",
                describe_policies(&after)
            ))
        }
    });

    ctx.auto("a change to always is reverted to never", result);
}

fn config_text(ctx: &Ctx<'_>) -> String {
    let base = ctx.var("XDG_CONFIG_HOME").map_or_else(
        || format!("{}/.config", ctx.var("HOME").unwrap_or_default()),
        str::to_owned,
    );

    std::fs::read_to_string(format!("{base}/plasma-builtin-screen-rotation.toml"))
        .unwrap_or_default()
}

/// How many settle times the program gets to set the policies after a
/// restart.
const RESTART_PATIENCE: u32 = 3;

/// `restore` gives the policies back, and the program starts again.
pub fn restore(ctx: &mut Ctx<'_>) {
    let rotation = &DAEMONS[1];
    let command = match activate::entry_command(ctx, rotation.entry) {
        Ok(command) => command,
        Err(reason) => {
            ctx.auto("restore command", Err(reason));
            return;
        }
    };
    let Some(mut session) = outputs::open(ctx, "restore command", Kind::Auto) else {
        return;
    };
    let before = config_text(ctx);
    let output = ctx.run(&command[0], &["restore"]);
    let after = settle_policies(ctx, &mut session, ctx.timing.settle, |policy| {
        policy != Policy::Never
    });
    let config = config_text(ctx);

    ctx.raw(
        "restore.txt",
        &format!(
            "config before:\n{before}\nrestore output:\n{}\nconfig after:\n{config}",
            output.text()
        ),
    );

    let result = if !output.success() {
        Err(format!(
            "restore ended with {:?}: {}",
            output.code,
            output.text().trim()
        ))
    } else if !all_are(&after, |policy| policy != Policy::Never) {
        Err(format!(
            "the policies are still never: {}",
            describe_policies(&after)
        ))
    } else if config.contains("eDP-") {
        Err("the configuration file of the program still lists policies".to_owned())
    } else {
        Ok(format!(
            "policies back to KWin's own: {}; the file has no policies",
            describe_policies(&after)
        ))
    };

    ctx.auto("restore gives KWin's own policy back", result);

    // The rotation checks that follow need the program again.
    let again = activate::autostart_unit(ctx, rotation.entry).and_then(|unit| {
        let restarted = ctx.run("systemctl", &["--user", "restart", &unit]);

        if restarted.success() {
            Ok(())
        } else {
            Err(format!(
                "systemctl --user restart {unit}: {}",
                restarted.text().trim()
            ))
        }
    });
    // The program claims the accelerometer before it flushes its first
    // request: a slow sensor service delays the policies.
    let patience = ctx.timing.settle * RESTART_PATIENCE;
    let policies = settle_policies(ctx, &mut session, patience, |policy| {
        policy == Policy::Never
    });

    ctx.auto(
        "program started again after restore",
        again.and_then(|()| {
            if all_are(&policies, |policy| policy == Policy::Never) {
                Ok(format!("restarted; {}", describe_policies(&policies)))
            } else {
                Err(format!(
                    "running, but the policies are {}",
                    describe_policies(&policies)
                ))
            }
        }),
    );
}

/// The devices with a switch for the tablet mode, from the text of
/// `/proc/bus/input/devices`: the bit 1 of the `SW` bitmap is
/// `SW_TABLET_MODE`.
#[must_use]
pub fn tablet_switches(text: &str) -> Vec<String> {
    text.split("\n\n")
        .filter(|block| {
            block
                .lines()
                .filter_map(|line| line.strip_prefix("B: SW="))
                .filter_map(|bits| u64::from_str_radix(bits.trim(), 16).ok())
                .any(|bits| bits & 0b10 != 0)
        })
        .filter_map(|block| {
            block
                .lines()
                .find_map(|line| line.strip_prefix("N: Name="))
                .map(|name| name.trim_matches('"').to_owned())
        })
        .collect()
}

/// Whether the machine has a switch for the tablet mode (information).
pub fn tablet(ctx: &mut Ctx<'_>) {
    let switches = tablet_switches(&ctx.read("/proc/bus/input/devices").unwrap_or_default());
    let libinput = ctx.sudo("libinput", &["list-devices"]);

    ctx.raw("libinput-list-devices.txt", &libinput.text());
    ctx.record(
        "tablet-mode switch",
        Kind::Auto,
        Status::Ok,
        &if switches.is_empty() {
            "none: KWin's own rotation does not run on this model (information, not a fault)"
                .to_owned()
        } else {
            format!("present: {}", switches.join(", "))
        },
    );
}

/// The keyboard is connected and its program is attached.
pub fn keyboard(ctx: &mut Ctx<'_>, phase: Phase) {
    let devices = activate::keyboard_hid(ctx);
    let listing = ctx.sudo("ls", &["-R", "/sys/fs/bpf/hid"]);
    let journal = ctx.run("journalctl", &["-b", "-k", "--no-pager", "-g", "hid"]);

    ctx.raw("bpf-pins.txt", &listing.text());
    ctx.raw("kernel-hid.txt", &journal.stdout);

    let attached = listing
        .stdout
        .to_ascii_uppercase()
        .lines()
        .any(|line| line.contains("0B05"));

    ctx.auto(
        &phase.row("keyboard connected"),
        if devices.is_empty() {
            Err("no HID device of vendor 0b05".to_owned())
        } else {
            Ok(devices.join(", "))
        },
    );
    ctx.auto(
        &phase.row("keyboard program attached"),
        if attached {
            Ok("udev-hid-bpf pinned a program for the keyboard in /sys/fs/bpf/hid".to_owned())
        } else {
            Err(format!(
                "no pinned program for 0b05 in /sys/fs/bpf/hid ({}); the guided key checks will show whether the keys work",
                listing.text().trim()
            ))
        },
    );
}

/// All automatic checks.
pub fn run(ctx: &mut Ctx<'_>, booted: bool) {
    panels(ctx);
    ready(ctx);
    shortcuts(ctx, Phase::Activated { booted });
    touch(ctx);
    policy_never(ctx);
    restore(ctx);
    crate::hand::automatic(ctx);
    tablet(ctx);
    keyboard(ctx, Phase::Activated { booted });
}

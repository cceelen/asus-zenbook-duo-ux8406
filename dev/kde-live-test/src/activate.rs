//! Make the installed pieces active without a reboot or a logout: the
//! system pieces (udev, hwdb) first, then the session pieces (the KWin
//! script, and the two programs through the autostart units of systemd).

use crate::context::Ctx;
use crate::dbus;
use crate::outputs::until;
use crate::report::{Kind, Status};

/// The script of the keys package, as KWin knows it.
pub const SCRIPT_ID: &str = "asus-zenbook-duo-ux8406-keys";

/// A program that Plasma starts by an autostart entry.
pub struct Daemon {
    /// Its autostart entry, without `.desktop`.
    pub entry: &'static str,
    /// The name it takes on the session bus.
    pub service: &'static str,
    /// Its name in the journal.
    pub ident: &'static str,
}

/// The two programs of the packages.
pub const DAEMONS: [Daemon; 2] = [
    Daemon {
        entry: "asus-ux8406-kde-keys",
        service: "io.github.cceelen.AsusUX8406.Keys",
        ident: "asus-ux8406-kde-keys",
    },
    Daemon {
        entry: "plasma-builtin-screen-rotation",
        service: "io.github.cceelen.BuiltinScreenRotation",
        ident: "plasma-builtin-screen-rotation",
    },
];

/// The command line of an `Exec=` line: the words, without the field codes.
#[must_use]
pub fn exec_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut started = false;

    for character in line.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            ' ' | '\t' if !quoted => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            other => {
                word.push(other);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
        .into_iter()
        .filter(|word| !(word.len() == 2 && word.starts_with('%')))
        .collect()
}

/// The `Exec` words of an autostart entry.
///
/// # Errors
///
/// The entry is not installed or has no `Exec`.
pub fn entry_command(ctx: &Ctx<'_>, entry: &str) -> Result<Vec<String>, String> {
    let path = format!("/etc/xdg/autostart/{entry}.desktop");
    let text = ctx
        .read(&path)
        .ok_or_else(|| format!("{path} is not installed"))?;
    let words = text
        .lines()
        .find_map(|line| line.strip_prefix("Exec="))
        .map(exec_words)
        .unwrap_or_default();

    if words.is_empty() {
        Err(format!("{path} has no Exec line"))
    } else {
        Ok(words)
    }
}

/// Run a privileged command and report it as one row.
fn privileged(ctx: &mut Ctx<'_>, check: &str, program: &str, args: &[&str]) {
    let output = ctx.sudo(program, args);

    ctx.auto(
        check,
        if output.success() {
            Ok(format!("sudo {program} {}", args.join(" ")))
        } else {
            Err(format!(
                "sudo {program} {} ended with {:?}: {}",
                args.join(" "),
                output.code,
                output.text().trim()
            ))
        },
    );
}

/// The names of the HID devices of the keyboard (vendor 0b05).
#[must_use]
pub fn keyboard_hid(ctx: &Ctx<'_>) -> Vec<String> {
    ctx.list("/sys/bus/hid/devices")
        .into_iter()
        .filter(|name| name.to_ascii_uppercase().contains(":0B05:"))
        .collect()
}

fn system(ctx: &mut Ctx<'_>) {
    privileged(
        ctx,
        "udev rules reloaded",
        "udevadm",
        &["control", "--reload"],
    );
    privileged(ctx, "hwdb updated", "systemd-hwdb", &["update"]);

    let devices = keyboard_hid(ctx);

    if devices.is_empty() {
        ctx.record(
            "keyboard HID devices re-triggered",
            Kind::Auto,
            Status::Skip,
            "no HID device of vendor 0b05: is the keyboard on the cable or on the screen?",
        );
    }
    for device in &devices {
        let path = format!("/sys/bus/hid/devices/{device}");

        privileged(
            ctx,
            &format!("HID device {device} re-triggered"),
            "udevadm",
            &["trigger", "--action=add", &path],
        );
    }
    privileged(
        ctx,
        "dock port re-triggered",
        "udevadm",
        &[
            "trigger",
            "--action=add",
            "--subsystem-match=usb",
            "--attr-match=idVendor=0b05",
        ],
    );
    privileged(
        ctx,
        "panel connectors re-triggered",
        "udevadm",
        &["trigger", "--action=change", "--subsystem-match=drm"],
    );
    privileged(
        ctx,
        "backlight re-triggered",
        "udevadm",
        &["trigger", "--action=change", "--subsystem-match=backlight"],
    );
    privileged(ctx, "udev settled", "udevadm", &["settle", "--timeout=20"]);
}

fn script(ctx: &Ctx<'_>) -> Result<String, String> {
    let file = format!("/usr/share/kwin/scripts/{SCRIPT_ID}/contents/code/main.js");
    let path = ctx.path(&file);
    let bus = ctx.session_bus()?;
    let scripting = |method: &str, body: &(&str,)| -> Result<bool, String> {
        dbus::call(
            &bus,
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting",
            method,
            body,
        )
    };

    if !path.is_file() {
        return Err(format!("{file} is not installed"));
    }
    // KWin loaded it already if it started after the install.
    if scripting("isScriptLoaded", &(SCRIPT_ID,))? {
        // If this fails, loadScript says why.
        let _ = scripting("unloadScript", &(SCRIPT_ID,));
    }

    let id: i32 = dbus::call(
        &bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "loadScript",
        &(path.to_string_lossy().as_ref(), SCRIPT_ID),
    )?;

    if id < 0 {
        return Err(format!("KWin did not load {file} (loadScript gave {id})"));
    }
    dbus::call::<_, ()>(
        &bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "start",
        &(),
    )?;
    Ok(format!("loadScript gave id {id}, then start"))
}

/// The unit names that systemd's autostart generator can give to an entry:
/// with the dashes of the desktop id escaped, and as they are.
fn unit_candidates(entry: &str) -> [String; 2] {
    [
        format!("app-{}@autostart.service", entry.replace('-', "\\x2d")),
        format!("app-{entry}@autostart.service"),
    ]
}

/// The unit that the generator made for an autostart entry.
///
/// # Errors
///
/// No unit exists for the entry.
pub fn autostart_unit(ctx: &Ctx<'_>, entry: &str) -> Result<String, String> {
    let candidates = unit_candidates(entry);

    candidates
        .iter()
        .find(|unit| ctx.run("systemctl", &["--user", "cat", unit]).success())
        .cloned()
        .ok_or_else(|| {
            format!(
                "systemctl --user cat finds none of {}: is {entry}.desktop in /etc/xdg/autostart and does the generator run?",
                candidates.join(", ")
            )
        })
}

/// Start the units as Plasma does after the login: the generator reads the
/// new entries again, then `xdg-desktop-autostart.target` starts them. A
/// target that is active already does not start the new units, so the units
/// start by name.
fn autostart(ctx: &mut Ctx<'_>) {
    let reloaded = ctx.run("systemctl", &["--user", "daemon-reload"]);

    ctx.auto(
        "autostart units made again from the entries",
        if reloaded.success() {
            Ok("systemctl --user daemon-reload".to_owned())
        } else {
            Err(reloaded.text().trim().to_owned())
        },
    );

    let units: Vec<(&Daemon, Result<String, String>)> = DAEMONS
        .iter()
        .map(|daemon| (daemon, autostart_unit(ctx, daemon.entry)))
        .collect();

    for (daemon, unit) in &units {
        ctx.auto(
            &format!("{} has an autostart unit", daemon.entry),
            unit.clone(),
        );
    }

    let names: Vec<&str> = units
        .iter()
        .filter_map(|(_, unit)| unit.as_deref().ok())
        .collect();
    let target = "xdg-desktop-autostart.target";
    let active = ctx
        .run("systemctl", &["--user", "is-active", "--quiet", target])
        .success();
    let mut args = vec!["--user", "start"];

    if active {
        args.extend(&names);
    } else {
        args.push(target);
    }

    let started = if active && names.is_empty() {
        Err("no autostart unit to start".to_owned())
    } else {
        let output = ctx.run("systemctl", &args);

        if output.success() {
            Ok(if active {
                format!("{target} was active already: started {}", names.join(", "))
            } else {
                format!("started {target}")
            })
        } else {
            Err(format!(
                "systemctl {} ended with {:?}: {}",
                args.join(" "),
                output.code,
                output.text().trim()
            ))
        }
    };

    ctx.auto("autostart units started", started);
    for (daemon, unit) in &units {
        if let Ok(unit) = unit {
            running(ctx, daemon, unit);
        }
    }
}

/// The unit is active and the program has its name on the session bus.
fn running(ctx: &mut Ctx<'_>, daemon: &Daemon, unit: &str) {
    let timing = ctx.timing;
    let active = until(&timing, timing.settle, || {
        ctx.run("systemctl", &["--user", "is-active", "--quiet", unit])
            .success()
    });
    let owned = ctx.session_bus().and_then(|bus| {
        if dbus::wait_owner(&bus, daemon.service, timing.settle, timing.poll) {
            Ok(())
        } else {
            Err(format!(
                "{} has no owner on the session bus",
                daemon.service
            ))
        }
    });

    ctx.auto(
        &format!("{} running", daemon.entry),
        if active {
            owned.map(|()| format!("{unit} is active and {} has an owner", daemon.service))
        } else {
            Err(format!("{unit} is not active"))
        },
    );
}

fn session(ctx: &mut Ctx<'_>, booted: bool) {
    let loaded = script(ctx);

    ctx.auto("KWin script loaded over D-Bus", loaded);
    if !booted {
        ctx.record(
            "KWin loads the script by itself",
            Kind::Auto,
            Status::Skip,
            "KWin reads its scripts when it starts: this needs a new session, and the live session has no login",
        );
    }
    autostart(ctx);
}

/// Activate everything. `booted` says that the packages were in the image
/// at the start, so that KWin has loaded the script by itself already.
pub fn run(ctx: &mut Ctx<'_>, booted: bool) {
    system(ctx);
    session(ctx, booted);
}

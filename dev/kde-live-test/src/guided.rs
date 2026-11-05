//! The checks where the tester does one physical action and the tool
//! decides from the events what happened. Each action has a time limit,
//! and the tester can skip it.

use asus_ux8406_kde_common::geometry::{Monitor, State, find, is_valid};
use asus_ux8406_kde_common::output::Session;

use crate::activate::DAEMONS;
use crate::auto::{LOWER, UPPER};
use crate::context::{Ctx, Timing};
use crate::dbus;
use crate::outputs::{self, Wait, describe, is_on, poll, until, watch};
use crate::report::{Kind, Status};
use crate::windows::{self, Windows};

/// What the lower screen is, for each prompt that names it. The upper screen
/// is the one with the camera.
const LOWER_IS: &str =
    "(The lower screen is the one next to the keyboard, the upper one has the camera.)";

/// Ask the tester for one action.
pub fn ask(ctx: &Ctx<'_>, action: &str) {
    ctx.say("");
    ctx.say(&format!(">>> {action}"));
    ctx.say(&ctx.timing.guided.map_or_else(
        || "    (type s and Enter to skip)".to_owned(),
        |limit| {
            format!(
                "    (type s and Enter to skip; the tool gives up after {} s)",
                limit.as_secs()
            )
        },
    ));
}

/// How a guided check ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// As expected.
    Good,
    /// Not as expected; the next action can still be asked.
    Bad,
    /// Skipped, late or lost: no point in asking for more.
    Gone,
}

/// The row for a wait that ended without the thing happening.
fn missed<T>(ctx: &mut Ctx<'_>, check: &str, wait: &Wait<T>, timeout: &str) -> Outcome {
    match wait {
        Wait::Skipped => ctx.record(check, Kind::Guided, Status::Skip, "skipped by the tester"),
        Wait::Lost(reason) => ctx.record(
            check,
            Kind::Guided,
            Status::Fail,
            &format!("lost the connection: {reason}"),
        ),
        _ => ctx.record(check, Kind::Guided, Status::Fail, timeout),
    }
    Outcome::Gone
}

/// The row for the end of a wait.
pub fn conclude<T>(
    ctx: &mut Ctx<'_>,
    check: &str,
    wait: Wait<T>,
    ok: impl FnOnce(T) -> Result<String, String>,
    timeout: &str,
) -> Outcome {
    match wait {
        Wait::Done(found) => {
            let result = ok(found);
            let good = result.is_ok();

            ctx.check(check, Kind::Guided, result);
            if good { Outcome::Good } else { Outcome::Bad }
        }
        other => missed(ctx, check, &other, timeout),
    }
}

/// The place, turn and scale of a monitor, to compare layouts.
fn place(monitor: &Monitor) -> (i32, i32, u32, String) {
    (
        monitor.x,
        monitor.y,
        monitor.transform,
        format!("{}", monitor.scale),
    )
}

fn same_places(one: &State, other: &State) -> bool {
    [UPPER, LOWER]
        .iter()
        .all(|name| find(&one.logical, name).map(place) == find(&other.logical, name).map(place))
}

/// The key right of F12, three times: off, on, off.
pub fn lower_key(ctx: &mut Ctx<'_>, session: &mut Session) {
    let start = session.outputs.state();
    let patience = ctx.timing.guided;
    let timing = ctx.timing;

    for (number, wanted) in [(1, false), (2, true), (3, false)] {
        let check = format!("key right of F12, press {number} of 3");
        let verb = if wanted { "on" } else { "off" };

        ask(
            ctx,
            &format!("Press the key right of F12. The lower screen should go {verb}. {LOWER_IS}"),
        );

        let wait = watch(session, ctx.console, &timing, patience, |state| {
            (is_on(state, LOWER) == wanted).then(|| state.clone())
        });
        let first = number == 1;
        let outcome = conclude(
            ctx,
            &check,
            wait,
            |state| {
                if wanted && !same_places(&state, &start) {
                    Err(format!(
                        "on again, but not in the same place: {} before, {} now",
                        describe(&start),
                        describe(&state)
                    ))
                } else if !first && !wanted && !is_on(&state, UPPER) {
                    Err("the upper screen went off too".to_owned())
                } else {
                    Ok(format!("lower screen {verb}; {}", describe(&state)))
                }
            },
            &format!("the lower screen did not go {verb} within the time limit"),
        );

        if outcome == Outcome::Gone {
            break;
        }
    }
    restore_lower(ctx, session);
}

/// Leave the lower screen on for the checks that follow.
fn restore_lower(ctx: &mut Ctx<'_>, session: &mut Session) {
    let timing = ctx.timing;

    if is_on(&session.outputs.state(), LOWER) {
        return;
    }

    let toggled = ctx.session_bus().and_then(|bus| {
        dbus::call::<_, ()>(
            &bus,
            DAEMONS[0].service,
            "/io/github/cceelen/AsusUX8406/Keys",
            DAEMONS[0].service,
            "ToggleLowerScreen",
            &(),
        )
    });
    let back = until(&timing, timing.settle, || {
        let _ = session.dispatch();
        is_on(&session.outputs.state(), LOWER)
    });

    ctx.say(&format!(
        "The tool switched the lower screen on again: {}",
        if toggled.is_ok() && back {
            "done"
        } else {
            "it did not work, press the key"
        }
    ));
}

/// Wait until a window that the tool opened is closed; else say that the
/// tester has to close it.
fn closed(ctx: &mut Ctx<'_>, what: &str, gone: impl Fn(&Ctx<'_>) -> bool) {
    let timing = ctx.timing;

    if !until(&timing, timing.settle, || gone(ctx)) {
        ctx.record(
            &format!("{what} closed"),
            Kind::Guided,
            Status::Ok,
            &format!("{what} is still open: close it by hand (information, not a fault)"),
        );
    }
}

/// Two terminal windows, one on each screen, and F8.
pub fn swap(ctx: &mut Ctx<'_>) {
    let check = "F8 swaps the windows of the two screens";
    let windows = match Windows::start(ctx) {
        Ok(windows) => windows,
        Err(reason) => {
            ctx.check(check, Kind::Guided, Err(reason));
            return;
        }
    };
    let mut pids = Vec::new();
    let mut failure = None;

    for title in ["UX8406-TEST-A", "UX8406-TEST-B"] {
        let log = ctx.report.dir().join("raw").join(format!("{title}.log"));
        let tab = format!("tabtitle={title}");

        match ctx.runner.start(
            "konsole",
            &["--nofork", "-p", &tab, "-e", "sleep", "3600"],
            &log,
        ) {
            Ok(pid) => pids.push(pid),
            Err(reason) => {
                failure = Some(reason);
                break;
            }
        }
    }
    if let Some(reason) = failure {
        ctx.check(check, Kind::Guided, Err(reason));
    } else {
        arrange_and_swap(ctx, &windows, check, (pids[0], pids[1]));
    }
    windows.stop();
    for pid in pids {
        let number = pid.to_string();

        let _ = ctx.run("kill", &[&number]);
        closed(
            ctx,
            &format!("the test window of process {number}"),
            |ctx| !ctx.run("kill", &["-0", &number]).success(),
        );
    }
}

/// Place the two windows, ask for F8 and watch them swap.
fn arrange_and_swap(ctx: &mut Ctx<'_>, windows: &Windows, check: &str, (a, b): (u32, u32)) {
    let timing = ctx.timing;
    let seen = until(&timing, timing.settle * 3, || {
        windows.output_of(a).is_some() && windows.output_of(b).is_some()
    });
    let placed = if seen {
        windows
            .place(a, UPPER)
            .and_then(|()| windows.place(b, LOWER))
    } else {
        Err("KWin did not report the two test windows".to_owned())
    };
    let arranged = placed.is_ok()
        && until(&timing, timing.settle, || {
            windows.output_of(a).as_deref() == Some(UPPER)
                && windows.output_of(b).as_deref() == Some(LOWER)
        });

    if arranged {
        ask(ctx, "Press F8. The two test windows should swap screens.");

        let wait = poll(ctx.console, &timing, timing.guided, || {
            Ok((windows.output_of(a).as_deref() == Some(LOWER)
                && windows.output_of(b).as_deref() == Some(UPPER))
            .then_some(()))
        });

        conclude(
            ctx,
            check,
            wait,
            |()| {
                Ok("window A went from eDP-1 to eDP-2 and window B from eDP-2 to eDP-1".to_owned())
            },
            "the windows did not swap within the time limit",
        );
    } else {
        ctx.check(
            check,
            Kind::Guided,
            Err(placed
                .err()
                .unwrap_or_else(|| "the test windows did not go to their screens".to_owned())),
        );
    }
}

/// The keyboard on the lower screen, and off it.
pub fn keyboard_on_screen(ctx: &mut Ctx<'_>, session: &mut Session) {
    let timing = ctx.timing;
    let patience = timing.guided;
    let before = session.outputs.state();

    ask(
        ctx,
        "Put the keyboard on the lower screen (the one that is next to it now).",
    );

    let wait = watch(session, ctx.console, &timing, patience, |state| {
        (!state.connected.contains_key(LOWER)).then_some(())
    });

    if conclude(
        ctx,
        "keyboard on the lower screen: the screen goes away",
        wait,
        |()| Ok("eDP-2 is gone from KWin's outputs".to_owned()),
        "eDP-2 stayed in KWin's outputs",
    ) != Outcome::Good
    {
        return;
    }

    ask(
        ctx,
        "Press the key right of F12 once. Nothing should happen.",
    );

    let wait = watch(session, ctx.console, &timing, timing.quiet, |state| {
        state.connected.contains_key(LOWER).then_some(())
    });

    ctx.check(
        "the key does nothing with the keyboard on the screen",
        Kind::Guided,
        match wait {
            Wait::Timeout | Wait::Skipped => {
                Ok("the outputs did not change while the tool watched".to_owned())
            }
            Wait::Done(()) => Err("eDP-2 came back".to_owned()),
            Wait::Lost(reason) => Err(reason),
        },
    );
    ask(
        ctx,
        "Take the keyboard off the lower screen, so that it lies next to it again.",
    );

    let wait = watch(session, ctx.console, &timing, patience, |state| {
        is_on(state, LOWER).then(|| state.clone())
    });

    conclude(
        ctx,
        "keyboard off the lower screen: screen and layout come back",
        wait,
        |state| {
            if same_places(&state, &before) {
                Ok(format!("same layout as before: {}", describe(&state)))
            } else {
                Err(format!(
                    "layout before {}, now {}",
                    describe(&before),
                    describe(&state)
                ))
            }
        },
        "eDP-2 did not come back",
    );
}

/// The events of `libinput debug-events` that the tool has not read yet.
struct Events {
    log: std::path::PathBuf,
    read: usize,
}

impl Events {
    /// The lines since the last call, as (node, event).
    fn fresh(&mut self) -> Vec<(String, String)> {
        let text = std::fs::read_to_string(&self.log).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let fresh = lines
            .iter()
            .skip(self.read)
            .filter_map(|line| {
                let mut words = line.split_whitespace();

                words.next().zip(words.next()).map(|(node, event)| {
                    (node.trim_start_matches('-').to_owned(), event.to_owned())
                })
            })
            .collect();

        self.read = lines.len();
        fresh
    }
}

/// One touch on one panel with one tool, as the tester is asked for it.
fn touch_case(
    ctx: &mut Ctx<'_>,
    events: &mut Events,
    panel_of: &dyn Fn(&str) -> Option<String>,
    panel: &str,
    (tool, prefix): (&str, &str),
) {
    let timing = ctx.timing;
    let name = if panel == LOWER { "lower" } else { "upper" };
    let check = format!("{tool} on the {name} screen");
    let how = if tool == "pen" {
        "with the pen (near or on the glass)"
    } else {
        "with a finger"
    };
    let (shout, place) = if panel == LOWER {
        ("LOWER", "the one next to the keyboard")
    } else {
        ("UPPER", "the one with the camera")
    };
    let (other, other_name) = if panel == LOWER {
        (UPPER, "upper")
    } else {
        (LOWER, "lower")
    };
    let mut touched = || {
        poll(ctx.console, &timing, timing.guided, || {
            Ok(events
                .fresh()
                .into_iter()
                .find(|(_, event)| event.starts_with(prefix))
                .map(|touch| vec![touch]))
        })
    };

    ask(ctx, &format!("Touch the {shout} screen ({place}) {how}."));

    let wait = match touched() {
        Wait::Done(one) if panel_of(&one[0].0).as_deref() == Some(other) => {
            ctx.say(&format!(
                ">>> That was the {other_name} screen. Touch the {shout} screen, {place}."
            ));
            match touched() {
                Wait::Done(two) => Wait::Done([one, two].concat()),
                ended => ended,
            }
        }
        ended => ended,
    };

    conclude(
        ctx,
        &check,
        wait,
        |touches| {
            let told = touches
                .iter()
                .map(|(node, event)| format!("{event} from {node}"))
                .collect::<Vec<_>>()
                .join("; then ");
            let (node, _) = &touches[touches.len() - 1];

            match panel_of(node) {
                Some(mapped) if mapped == panel => Ok(format!(
                    "{told}, which KWin maps to {panel}{}",
                    if touches.len() == 2 {
                        " (the first touch was on the other screen)"
                    } else {
                        ""
                    }
                )),
                Some(mapped) => Err(format!(
                    "{told}, which KWin maps to {mapped:?}, not {panel}"
                )),
                None => Err(format!("{told}, which KWin does not list")),
            }
        },
        "no event from the ELAN controllers",
    );
}

/// A finger, and with `--pen` the pen, on each panel.
pub fn touch(ctx: &mut Ctx<'_>) {
    let devices = match crate::auto::elan_devices(ctx) {
        Ok(devices) if !devices.is_empty() => devices,
        Ok(_) => {
            ctx.record(
                "touch and pen",
                Kind::Guided,
                Status::Skip,
                "KWin lists no ELAN device",
            );
            return;
        }
        Err(reason) => {
            ctx.check("touch and pen", Kind::Guided, Err(reason));
            return;
        }
    };
    let nodes: Vec<String> = devices
        .iter()
        .map(|device| format!("/dev/input/{}", device.sys_name))
        .collect();
    let log = ctx
        .report
        .dir()
        .join("raw")
        .join("libinput-debug-events.log");
    let mut args = vec!["-n", "stdbuf", "-oL", "libinput", "debug-events"];

    for node in &nodes {
        args.extend(["--device", node]);
    }
    if let Err(reason) = ctx.runner.start("sudo", &args, &log) {
        ctx.check("touch and pen", Kind::Guided, Err(reason));
        return;
    }

    let timing = ctx.timing;
    let mut events = Events { log, read: 0 };
    let panel_of = |node: &str| {
        devices
            .iter()
            .find(|device| device.sys_name == node)
            .map(|device| device.output_name.clone())
    };

    let tools: &[(&str, &str)] = if ctx.pen {
        &[("finger", "TOUCH_DOWN"), ("pen", "TABLET_TOOL_")]
    } else {
        &[("finger", "TOUCH_DOWN")]
    };

    if !ctx.pen {
        ctx.record(
            "pen on each screen",
            Kind::Guided,
            Status::Skip,
            "no pen: run the tool with --pen to test it",
        );
    }
    // Let the program start, and drop its first lines.
    let _ = until(&timing, timing.settle, || !events.fresh().is_empty());
    for panel in [LOWER, UPPER] {
        for &tool in tools {
            touch_case(ctx, &mut events, &panel_of, panel, tool);
        }
    }
    let _ = ctx.sudo("pkill", &["-f", "libinput debug-events"]);
}

/// The packaged program that reads the keyboard's state map.
const KEYBOARD_STATE: &str = "/usr/libexec/asus-ux8406-keyboard-state";
/// A HID device of the keyboard that is not there: `load` then reads the
/// maps of the connections that are, and writes nothing.
const NO_DEVICE: &str = "0003:0B05:1BF2.0000";
/// A state file that does not exist, so that `load` has no state of its own
/// to give.
const NO_STATE_FILE: &str = "/nonexistent/asus-ux8406-keyboard-state";

/// The backlight level in the state map of the keyboard's program, with
/// root; none if there is no map to read.
fn keyboard_level(ctx: &Ctx<'_>) -> Option<u8> {
    if !ctx.path(KEYBOARD_STATE).is_file() {
        return None;
    }

    let output = ctx.sudo(KEYBOARD_STATE, &["-s", NO_STATE_FILE, "load", NO_DEVICE]);

    output
        .success()
        .then(|| {
            output
                .stdout
                .lines()
                .find_map(|line| line.strip_prefix("ASUS_UX8406_KBD_BACKLIGHT="))
                .and_then(|level| level.trim().parse().ok())
        })
        .flatten()
}

/// The key of the keyboard backlight, twice. The keyboard has no LED device:
/// its program keeps the level in a map, which the tool reads before and
/// after each press. Without a map the tester says whether the light changed.
pub fn backlight(ctx: &mut Ctx<'_>) {
    // Each look runs sudo: not ten times a second.
    let timing = Timing {
        poll: ctx.timing.poll * 10,
        ..ctx.timing
    };
    let mut level = keyboard_level(ctx);

    for number in 1..=2 {
        let check = format!("keyboard backlight key, press {number} of 2");

        ask(
            ctx,
            "Press the keyboard backlight key (the one with the light symbol). Each press must change the light.",
        );

        let outcome = if let Some(before) = level {
            let wait = poll(ctx.console, &timing, timing.guided, || {
                Ok(keyboard_level(ctx).filter(|now| *now != before))
            });

            let outcome = conclude(
                ctx,
                &check,
                wait,
                |now| {
                    Ok(format!(
                        "the level in the state map went from {before} to {now}"
                    ))
                },
                "the level in the state map did not change",
            );

            level = keyboard_level(ctx).or(level);
            outcome
        } else {
            let changed = ctx.console.confirm("Did the keyboard light change?");

            ctx.check(
                &check,
                Kind::Guided,
                if changed {
                    Ok("the tester says the light changed (no state map to read)".to_owned())
                } else {
                    Err("the tester says the light did not change".to_owned())
                },
            );
            if changed {
                Outcome::Good
            } else {
                Outcome::Gone
            }
        };

        if outcome == Outcome::Gone {
            break;
        }
    }
}

/// The orientations, as the sensor says them, with the transform that the
/// panels that are on get.
const TURNS: [(&str, &str, u32); 4] = [
    ("left-up", "Turn the laptop so that its LEFT side is up.", 1),
    (
        "right-up",
        "Turn the laptop so that its RIGHT side is up.",
        3,
    ),
    (
        "normal",
        "Put the laptop upright again, as in normal use.",
        0,
    ),
    ("bottom-up", "Turn the laptop upside down.", 2),
];

fn orientation(bus: &zbus::blocking::Connection) -> Result<String, String> {
    dbus::property(
        bus,
        "net.hadess.SensorProxy",
        "/net/hadess/SensorProxy",
        "net.hadess.SensorProxy",
        "AccelerometerOrientation",
    )
}

/// The sensor reports it: wait until the laptop is upright.
fn upright(ctx: &mut Ctx<'_>, bus: &zbus::blocking::Connection, text: &str) -> bool {
    let timing = ctx.timing;

    ask(ctx, text);
    matches!(
        poll(ctx.console, &timing, timing.guided, || {
            orientation(bus).map(|text| (text == "normal").then_some(()))
        }),
        Wait::Done(())
    )
}

/// Ask for the key right of F12 until the lower screen is on or off, as
/// wanted. Whether it is.
fn lower_is(ctx: &mut Ctx<'_>, session: &mut Session, label: &str, on: bool) -> bool {
    let timing = ctx.timing;
    let verb = if on { "on" } else { "off" };

    if is_on(&session.outputs.state(), LOWER) == on {
        return true;
    }
    ask(
        ctx,
        &format!("Press the key right of F12. The lower screen should go {verb}. {LOWER_IS}"),
    );

    let wait = watch(session, ctx.console, &timing, timing.guided, |state| {
        (is_on(state, LOWER) == on).then_some(())
    });

    match wait {
        Wait::Done(()) => true,
        other => {
            missed(
                ctx,
                &format!("rotation, {label}"),
                &other,
                &format!("the lower screen did not go {verb} within the time limit"),
            );
            false
        }
    }
}

/// Whether the lower panel lies above the upper one, or one of them is off.
fn lower_on_top(state: &State) -> bool {
    find(&state.logical, LOWER)
        .zip(find(&state.logical, UPPER))
        .is_none_or(|(lower, upper)| lower.y < upper.y)
}

/// Turn the laptop four ways, with the panels that are on now. Each turn
/// is judged on those panels.
fn turns(ctx: &mut Ctx<'_>, session: &mut Session, bus: &zbus::blocking::Connection, label: &str) {
    let timing = ctx.timing;

    // The program puts the layout from before back a moment after the
    // laptop is upright.
    let _ = until(&timing, timing.settle, || {
        let _ = session.dispatch();

        let state = session.outputs.state();

        state.logical.iter().all(|monitor| monitor.transform == 0)
    });

    let base = session.outputs.state();
    let active: Vec<&str> = [UPPER, LOWER]
        .into_iter()
        .filter(|panel| is_on(&base, panel))
        .collect();

    for (name, text, transform) in TURNS {
        let check = format!("rotation, {label}: {name}");

        ask(ctx, text);

        let turned = poll(ctx.console, &timing, timing.guided, || {
            orientation(bus).map(|now| (now == name).then_some(()))
        });

        if !matches!(turned, Wait::Done(())) {
            missed(
                ctx,
                &check,
                &turned,
                "the sensor did not report this orientation",
            );
            continue;
        }

        let wait = watch(session, ctx.console, &timing, timing.settle, |state| {
            active
                .iter()
                .all(|panel| {
                    find(&state.logical, panel)
                        .is_some_and(|monitor| monitor.transform == transform)
                })
                .then(|| state.clone())
        });
        let state = session.outputs.state();

        conclude(
            ctx,
            &check,
            wait,
            |state| {
                if !is_valid(&state, &state.logical) {
                    Err(format!("gap or overlap: {}", describe(&state)))
                } else if name == "bottom-up" && !lower_on_top(&state) {
                    Err(format!(
                        "the lower panel is not on top: {}",
                        describe(&state)
                    ))
                } else if name == "normal" && !same_places(&state, &base) {
                    Err(format!(
                        "not the layout from before: {} now, {} before",
                        describe(&state),
                        describe(&base)
                    ))
                } else {
                    Ok(format!(
                        "{} turned as wanted, no gap, no overlap: {}",
                        active.join(" and "),
                        describe(&state)
                    ))
                }
            },
            &format!("the panels did not turn to {name}: {}", describe(&state)),
        );
    }
}

/// The program of the rotation turns the screens only while it is on: a tester
/// can have switched it off with the applet. Switch it on, and say so.
fn rotation_on(ctx: &mut Ctx<'_>) {
    let timing = ctx.timing;
    let Ok(bus) = ctx.session_bus() else {
        return;
    };
    let service = DAEMONS[1].service;
    let path = "/io/github/cceelen/BuiltinScreenRotation";
    let enabled = || dbus::property::<bool>(&bus, service, path, service, "Enabled");

    if enabled() != Ok(false) {
        return;
    }

    let switched = dbus::call::<_, ()>(&bus, service, path, service, "SetEnabled", &(true,))
        .and_then(|()| {
            if until(&timing, timing.settle, || enabled() == Ok(true)) {
                Ok(())
            } else {
                Err("the Enabled property stayed false".to_owned())
            }
        });

    match switched {
        Ok(()) => ctx.record(
            "rotation was off",
            Kind::Guided,
            Status::Ok,
            "rotation was off, switched on for the test (information, not a fault)",
        ),
        Err(reason) => ctx.check(
            "rotation was off",
            Kind::Guided,
            Err(format!("could not switch the rotation on: {reason}")),
        ),
    }
}

/// Turn the laptop four ways, once with both panels on and once with the
/// upper panel only.
pub fn rotation(ctx: &mut Ctx<'_>, session: &mut Session) {
    let bus = match ctx.system_bus() {
        Ok(bus) => bus,
        Err(reason) => {
            ctx.check("rotation", Kind::Guided, Err(reason));
            return;
        }
    };
    // The sensor reports only to a client that claims it.
    let _ = dbus::call::<_, ()>(
        &bus,
        "net.hadess.SensorProxy",
        "/net/hadess/SensorProxy",
        "net.hadess.SensorProxy",
        "ClaimAccelerometer",
        &(),
    );

    rotation_on(ctx);
    for (label, lower_on) in [("both panels", true), ("upper panel only", false)] {
        let text = if lower_on {
            "Put the keyboard on the cable and the laptop upright."
        } else {
            "Hold the laptop upright for the run with the upper panel only."
        };

        if !upright(ctx, &bus, text) {
            ctx.record(
                &format!("rotation, {label}"),
                Kind::Guided,
                Status::Skip,
                "the laptop was not upright at the start",
            );
            break;
        }
        if lower_is(ctx, session, label, lower_on) {
            turns(ctx, session, &bus, label);
        }
    }
    restore_lower(ctx, session);
}

/// The tray applet.
pub fn applet(ctx: &mut Ctx<'_>) {
    let timing = ctx.timing;
    let bus = match ctx.session_bus() {
        Ok(bus) => bus,
        Err(reason) => {
            ctx.check("tray applet", Kind::Guided, Err(reason));
            return;
        }
    };
    let enabled = || {
        dbus::property::<bool>(
            &bus,
            DAEMONS[1].service,
            "/io/github/cceelen/BuiltinScreenRotation",
            DAEMONS[1].service,
            "Enabled",
        )
    };

    for (wanted, text) in [
        (
            false,
            "Middle-click the Auto-rotate icon in the system tray once (a left click would open the popup). The icon may be behind the arrow.",
        ),
        (true, "Middle-click the Auto-rotate icon again."),
    ] {
        ask(ctx, text);

        let wait = poll(ctx.console, &timing, timing.guided, || {
            enabled().map(|now| (now == wanted).then_some(()))
        });

        conclude(
            ctx,
            if wanted {
                "tray applet: click switches rotation on"
            } else {
                "tray applet: click switches rotation off"
            },
            wait,
            |()| {
                Ok(format!(
                    "the Enabled property of the program became {wanted}"
                ))
            },
            "the Enabled property did not change",
        );
    }
}

/// A key that starts a program.
pub fn key_starts(ctx: &mut Ctx<'_>, check: &str, key: &str, process: &str) {
    let timing = ctx.timing;

    ask(ctx, &format!("Press {key}."));

    let wait = poll(ctx.console, &timing, timing.guided, || {
        Ok(ctx.run("pgrep", &["-x", process]).success().then_some(()))
    });

    conclude(
        ctx,
        check,
        wait,
        |()| Ok(format!("the process {process} started")),
        &format!("no process {process} within the time limit"),
    );
    let _ = ctx.run("pkill", &["-x", process]);
    closed(ctx, process, |ctx| {
        !ctx.run("pgrep", &["-x", process]).success()
    });
}

/// All guided checks.
pub fn run(ctx: &mut Ctx<'_>) {
    ctx.say("");
    ctx.say("Now the checks with your help. For each, the tool says what to do.");
    ctx.say("Keep the keyboard on the cable unless the tool says otherwise.");
    ctx.say("The lower screen is the one next to the keyboard, the upper one has the camera.");

    // The instructions must stay in view while the tool opens windows.
    let above = windows::keep_above(ctx, true);

    if let Err(reason) = &above {
        ctx.record(
            "terminal window above the others",
            Kind::Guided,
            Status::Ok,
            &format!("not done: {reason} (information, not a fault)"),
        );
    }

    if let Some(mut session) = outputs::open(ctx, "guided checks", Kind::Guided) {
        lower_key(ctx, &mut session);
        swap(ctx);
        keyboard_on_screen(ctx, &mut session);
        touch(ctx);
        backlight(ctx);
        rotation(ctx, &mut session);
        crate::hand::guided(ctx, &mut session);
        applet(ctx);
    }
    key_starts(
        ctx,
        "F11 opens the emoji selector",
        "F11 (the emoji key)",
        "plasma-emojier",
    );
    key_starts(
        ctx,
        "F12 opens System Settings",
        "F12 (MyASUS)",
        "systemsettings",
    );
    if above.is_ok() {
        let _ = windows::keep_above(ctx, false);
    }
}

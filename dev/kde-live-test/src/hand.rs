//! A screen turned by hand: the program of the rotation holds it, and
//! Auto-rotate shows off until it is switched on again. The saved setting does
//! not change.

use asus_ux8406_kde_common::geometry::{State, find, is_valid};
use asus_ux8406_kde_common::output::Session;
use asus_ux8406_kde_common::rotation::Orientation;

use crate::activate::DAEMONS;
use crate::auto::{LOWER, UPPER};
use crate::context::Ctx;
use crate::dbus;
use crate::guided::{Outcome, ask, conclude};
use crate::outputs::{self, Wait, describe, is_on, poll, until, watch};
use crate::report::{Kind, Status};

const PATH: &str = "/io/github/cceelen/BuiltinScreenRotation";

/// The transform of the screen on a connector.
fn transform_of(state: &State, connector: &str) -> Option<u32> {
    find(&state.logical, connector).map(|monitor| monitor.transform)
}

/// Whether the program of the rotation says that the screens follow the
/// sensor.
fn enabled(ctx: &Ctx<'_>) -> Result<bool, String> {
    ctx.session_bus().and_then(|bus| {
        dbus::property::<bool>(
            &bus,
            DAEMONS[1].service,
            PATH,
            DAEMONS[1].service,
            "Enabled",
        )
    })
}

/// A method of the program of the rotation.
fn call<B>(ctx: &Ctx<'_>, method: &str, body: &B) -> Result<(), String>
where
    B: zbus::export::serde::Serialize + zbus::zvariant::DynamicType,
{
    ctx.session_bus().and_then(|bus| {
        dbus::call::<_, ()>(
            &bus,
            DAEMONS[1].service,
            PATH,
            DAEMONS[1].service,
            method,
            body,
        )
    })
}

/// The transform that the sensor asks for now; none if it gives no
/// orientation.
fn sensor_transform(ctx: &Ctx<'_>) -> Result<Option<u32>, String> {
    ctx.system_bus().and_then(|bus| {
        dbus::property::<String>(
            &bus,
            "net.hadess.SensorProxy",
            "/net/hadess/SensorProxy",
            "net.hadess.SensorProxy",
            "AccelerometerOrientation",
        )
        .map(|text| Orientation::from_sensor(&text).transform())
    })
}

/// Both screens under the sensor: each has the transform of the laptop's
/// orientation, or the sensor gives none.
fn under_the_sensor(wanted: Result<Option<u32>, String>, state: &State) -> Result<String, String> {
    match wanted? {
        None => Ok(format!(
            "the sensor gives no orientation; the screens are {}",
            describe(state)
        )),
        Some(transform)
            if [UPPER, LOWER]
                .iter()
                .all(|panel| transform_of(state, panel) == Some(transform)) =>
        {
            Ok(format!(
                "both screens have transform {transform}: {}",
                describe(state)
            ))
        }
        Some(transform) => Err(format!(
            "the laptop's orientation asks for transform {transform}, the screens are {}",
            describe(state)
        )),
    }
}

/// The lower screen turned upside down by the program, and then handed back
/// to the sensor. Nothing is asked of the tester.
pub fn automatic(ctx: &mut Ctx<'_>) {
    let check = "the lower screen turned by hand (TurnScreen)";
    let timing = ctx.timing;
    let Some(mut session) = outputs::open(ctx, check, Kind::Auto) else {
        return;
    };
    let before = session.outputs.state();

    if !(is_on(&before, UPPER) && is_on(&before, LOWER)) {
        ctx.record(
            check,
            Kind::Auto,
            Status::Skip,
            "a built-in screen is off: the check needs both",
        );
        return;
    }

    let turned = call(ctx, "TurnScreen", &(LOWER, "bottom-up")).and_then(|()| {
        if until(&timing, timing.settle, || {
            let _ = session.dispatch();
            transform_of(&session.outputs.state(), LOWER) == Some(2)
        }) {
            Ok(())
        } else {
            Err(format!(
                "{LOWER} did not turn to transform 2: {}",
                describe(&session.outputs.state())
            ))
        }
    });
    let held = until(&timing, timing.settle, || enabled(ctx) == Ok(false));
    let now = session.outputs.state();

    ctx.check(
        check,
        Kind::Auto,
        turned.and_then(|()| {
            if transform_of(&now, UPPER) != transform_of(&before, UPPER) {
                Err(format!("{UPPER} changed: {}", describe(&now)))
            } else if !is_valid(&now, &now.logical) {
                Err(format!("gap or overlap: {}", describe(&now)))
            } else if !held {
                Err("Enabled stayed true after a turn by hand".to_owned())
            } else {
                Ok(format!(
                    "{LOWER} has transform 2, {UPPER} is as it was, the layout is valid, Enabled is false: {}",
                    describe(&now)
                ))
            }
        }),
    );

    let check = "switching on hands the screens back to the sensor";
    let resumed = call(ctx, "SetEnabled", &(true,)).and_then(|()| {
        if until(&timing, timing.settle, || enabled(ctx) == Ok(true)) {
            Ok(())
        } else {
            Err("Enabled stayed false".to_owned())
        }
    });
    let mut result = Err("not asked".to_owned());

    until(&timing, timing.settle * 3, || {
        let _ = session.dispatch();
        result = under_the_sensor(sensor_transform(ctx), &session.outputs.state());
        result.is_ok()
    });
    ctx.check(check, Kind::Auto, resumed.and(result));
}

/// The lower screen turned upside down with the applet, kept so across the
/// keyboard going on and off the screen, and then handed back to the sensor.
pub fn guided(ctx: &mut Ctx<'_>, session: &mut Session) {
    let timing = ctx.timing;
    let lower_turned = |state: &State| transform_of(state, LOWER) == Some(2);

    ask(
        ctx,
        "Open the Auto-rotate applet (click its icon in the system tray) and turn the LOWER screen (the one next to the keyboard) upside down.",
    );

    let wait = watch(session, ctx.console, &timing, timing.guided, |state| {
        lower_turned(state).then(|| state.clone())
    });
    let turned = conclude(
        ctx,
        "applet: turn the lower screen upside down",
        wait,
        |state| {
            if transform_of(&state, UPPER) == Some(0) && is_valid(&state, &state.logical) {
                Ok(format!(
                    "only the lower screen turned: {}",
                    describe(&state)
                ))
            } else {
                Err(format!(
                    "the upper screen changed or the layout is not valid: {}",
                    describe(&state)
                ))
            }
        },
        "the lower screen did not turn upside down",
    ) == Outcome::Good;

    if turned {
        keyboard_across(ctx, session);
    } else {
        ctx.record(
            "applet: the keyboard goes on and off a held screen",
            Kind::Guided,
            Status::Skip,
            "the lower screen was not turned upside down by hand",
        );
    }
    switch_on(ctx, session, turned);
}

/// The keyboard on the lower screen and off it: the screen comes back upside
/// down.
fn keyboard_across(ctx: &mut Ctx<'_>, session: &mut Session) {
    let timing = ctx.timing;

    ask(ctx, "Put the keyboard on the lower screen.");

    let gone = watch(session, ctx.console, &timing, timing.guided, |state| {
        (!state.connected.contains_key(LOWER)).then_some(())
    });

    if !matches!(gone, Wait::Done(())) {
        ctx.record(
            "applet: the keyboard goes on and off a held screen",
            Kind::Guided,
            Status::Skip,
            "the lower screen did not go away, or the tester skipped",
        );
        return;
    }
    ask(
        ctx,
        "Take the keyboard off the lower screen, so that it lies next to it again.",
    );

    let back = poll(ctx.console, &timing, timing.guided, || {
        let _ = session.dispatch();

        let state = session.outputs.state();

        Ok((is_on(&state, LOWER) && transform_of(&state, LOWER) == Some(2)).then_some(state))
    });

    conclude(
        ctx,
        "applet: the keyboard goes on and off a held screen",
        back,
        |state| {
            Ok(format!(
                "the lower screen came back upside down: {}",
                describe(&state)
            ))
        },
        "the lower screen did not come back upside down",
    );
}

/// Auto-rotate on again, with the applet: both screens under the sensor. If
/// the tester did not, or the lower screen was not held, the tool does it.
fn switch_on(ctx: &mut Ctx<'_>, session: &mut Session, turned: bool) {
    let timing = ctx.timing;
    let check = "applet: Auto-rotate on hands the screens back to the sensor";

    if !turned {
        // Nothing was held by the tester; the screens are as they were.
        let _ = call(ctx, "SetEnabled", &(true,));
        ctx.record(
            check,
            Kind::Guided,
            Status::Skip,
            "no screen was turned by hand",
        );
        return;
    }
    ask(ctx, "Switch Auto-rotate on (the switch in the applet).");

    let wait = poll(ctx.console, &timing, timing.guided, || {
        let _ = session.dispatch();

        let state = session.outputs.state();

        Ok(
            (enabled(ctx) == Ok(true) && under_the_sensor(sensor_transform(ctx), &state).is_ok())
                .then_some(state),
        )
    });

    let wanted = sensor_transform(ctx);

    conclude(
        ctx,
        check,
        wait,
        |state| under_the_sensor(wanted, &state),
        "Auto-rotate did not come on, or the screens did not go back to the sensor",
    );
    // A tester who skipped leaves the rotation on for the checks that follow.
    let _ = call(ctx, "SetEnabled", &(true,));
}

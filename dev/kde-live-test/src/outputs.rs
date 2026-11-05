//! KWin's outputs as the tool follows them: the globals of the compositor,
//! and the waits for a change that the tester causes.

use std::time::{Duration, Instant};

use asus_ux8406_kde_common::geometry::{State, find};
use asus_ux8406_kde_common::output::Session;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::console::Console;
use crate::context::{Ctx, Timing};

/// The globals that the registry announced.
#[derive(Default)]
struct Probe {
    globals: Vec<(String, u32)>,
}

impl Dispatch<WlRegistry, ()> for Probe {
    fn event(
        state: &mut Self,
        _: &WlRegistry,
        event: wl_registry::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.globals.extend(announced(event));
    }
}

/// The interface and version of a global that an event announces; none for
/// the event that takes a global away.
#[must_use]
pub fn announced(event: wl_registry::Event) -> Option<(String, u32)> {
    match event {
        wl_registry::Event::Global {
            interface, version, ..
        } => Some((interface, version)),
        _ => None,
    }
}

/// The globals of the compositor, as interface and version.
///
/// # Errors
///
/// The compositor does not answer.
pub fn globals(connection: &Connection) -> Result<Vec<(String, u32)>, String> {
    let mut queue = connection.new_event_queue::<Probe>();
    let mut probe = Probe::default();
    let _registry = connection.display().get_registry(&queue.handle(), ());

    queue
        .roundtrip(&mut probe)
        .map(|_| probe.globals)
        .map_err(|error| format!("Wayland registry: {error}"))
}

/// How a wait ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Wait<T> {
    /// The thing happened.
    Done(T),
    /// It did not happen in time.
    Timeout,
    /// The tester asked to skip.
    Skipped,
    /// The connection to KWin failed.
    Lost(String),
}

/// Look until `probe` has an answer. `probe` is called at once and after
/// each pause. Without `patience` the look goes on until the answer comes or
/// the tester skips.
pub fn poll<T>(
    console: &dyn Console,
    timing: &Timing,
    patience: impl Into<Option<Duration>>,
    mut probe: impl FnMut() -> Result<Option<T>, String>,
) -> Wait<T> {
    let end = patience.into().map(|patience| Instant::now() + patience);

    loop {
        match probe() {
            Ok(Some(found)) => return Wait::Done(found),
            Err(reason) => return Wait::Lost(reason),
            Ok(None) => {}
        }
        if console.skip_requested() {
            return Wait::Skipped;
        }
        if end.is_some_and(|end| Instant::now() >= end) {
            return Wait::Timeout;
        }
        std::thread::sleep(timing.poll);
    }
}

/// Look at the outputs until `probe` has an answer.
pub fn watch<T>(
    session: &mut Session,
    console: &dyn Console,
    timing: &Timing,
    patience: impl Into<Option<Duration>>,
    mut probe: impl FnMut(&State) -> Option<T>,
) -> Wait<T> {
    poll(console, timing, patience, || {
        session
            .dispatch()
            .map(|()| probe(&session.outputs.state()))
            .map_err(|error| error.to_string())
    })
}

/// Whether the connector is on: it has a place in the layout.
#[must_use]
pub fn is_on(state: &State, connector: &str) -> bool {
    find(&state.logical, connector).is_some()
}

/// A short text for the layout, for the report.
#[must_use]
pub fn describe(state: &State) -> String {
    let monitors: Vec<String> = state
        .logical
        .iter()
        .map(|monitor| {
            format!(
                "{} at {},{} transform {} scale {}",
                monitor.connectors.join("+"),
                monitor.x,
                monitor.y,
                monitor.transform,
                monitor.scale
            )
        })
        .collect();

    format!("[{}]", monitors.join("; "))
}

/// Connect to KWin's outputs for a check, or give the reason as a failed row.
pub fn open(ctx: &mut Ctx<'_>, check: &str, kind: crate::report::Kind) -> Option<Session> {
    match ctx.outputs() {
        Ok(session) => Some(session),
        Err(reason) => {
            ctx.check(check, kind, Err(reason));
            None
        }
    }
}

/// Wait until `probe` is true, at most `patience`; whether it became true.
pub fn until(timing: &Timing, patience: Duration, mut probe: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + patience;

    loop {
        if probe() {
            return true;
        }
        if Instant::now() >= end {
            return false;
        }
        std::thread::sleep(timing.poll);
    }
}

//! The global shortcut "Toggle auto-rotate": a component of kglobalacceld
//! that the program registers itself, over D-Bus, with no default key. The user
//! sets the key in System Settings, Shortcuts. kglobalacceld sends
//! `globalShortcutPressed` for the component when the key is pressed.
//!
//! The interfaces are `org.kde.KGlobalAccel` (`doRegister`, `setShortcutKeys`,
//! `getComponent`) and `org.kde.kglobalaccel.Component` of the `KGlobalAccel`
//! framework (`src/org.kde.KGlobalAccel.xml`,
//! `src/org.kde.kglobalaccel.Component.xml`), and the program does what the
//! framework's `KGlobalAccel::setGlobalShortcut` does for an action.

use std::thread;

use zbus::MatchRule;
use zbus::blocking::{Connection, MessageIterator};
use zbus::message::Type;

/// The name of the service of kglobalacceld.
const SERVICE: &str = "org.kde.kglobalaccel";
const PATH: &str = "/kglobalaccel";
const INTERFACE: &str = "org.kde.KGlobalAccel";
const COMPONENT_INTERFACE: &str = "org.kde.kglobalaccel.Component";

/// The unique name of the component.
pub const COMPONENT: &str = "plasma-builtin-screen-rotation";
/// The unique name of the action.
pub const ACTION: &str = "ToggleAutoRotate";

/// `SetPresent` of kglobalacceld: the action exists in a running program.
const SET_PRESENT: u32 = 2;
/// `IsDefault`: the keys are the default ones.
const IS_DEFAULT: u32 = 8;

/// The action id: the unique and the friendly name of the component, and of
/// the action.
fn action_id() -> Vec<String> {
    [
        COMPONENT,
        ACTION,
        "Auto-rotate screens",
        "Toggle auto-rotate",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// Register the action, with no default key. kglobalacceld gives back the key
/// that the user has set for it.
fn register(bus: &Connection) -> zbus::Result<()> {
    let id = action_id();
    let none: Vec<(Vec<i32>,)> = Vec::new();

    bus.call_method(Some(SERVICE), PATH, Some(INTERFACE), "doRegister", &(&id,))
        .and_then(|_| {
            bus.call_method(
                Some(SERVICE),
                PATH,
                Some(INTERFACE),
                "setShortcutKeys",
                &(&id, &none, SET_PRESENT),
            )
        })
        .and_then(|_| {
            bus.call_method(
                Some(SERVICE),
                PATH,
                Some(INTERFACE),
                "setShortcutKeys",
                &(&id, &none, IS_DEFAULT),
            )
        })
        .map(drop)
}

/// Register the action, and say in the journal why it did not work: a
/// kglobalacceld that is not there yet registers nothing, and the program
/// tries again when it is.
fn try_register(bus: &Connection) {
    let _ =
        register(bus).inspect_err(|error| log::info!("global shortcut: not registered: {error}"));
}

/// The messages of a rule on a connection.
fn messages(
    bus: &Connection,
    rule: zbus::Result<MatchRule<'static>>,
) -> zbus::Result<MessageIterator> {
    rule.and_then(|rule| MessageIterator::for_match_rule(rule, bus, None))
}

/// Register the action now and each time kglobalacceld starts again, and call
/// `pressed` each time the user presses its key. Both go on in threads of
/// their own: kglobalacceld need not be there when the program starts. The
/// program works without the shortcut: an error to follow the signals leaves
/// it out.
///
/// # Errors
///
/// The signals cannot be followed.
pub fn watch(bus: &Connection, pressed: impl Fn() + Send + 'static) -> zbus::Result<()> {
    let owners = messages(
        bus,
        MatchRule::builder()
            .msg_type(Type::Signal)
            .sender("org.freedesktop.DBus")
            .and_then(|rule| rule.member("NameOwnerChanged"))
            .and_then(|rule| rule.add_arg(SERVICE))
            .map(zbus::match_rule::Builder::build),
    );
    let keys = messages(
        bus,
        MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(SERVICE)
            .and_then(|rule| rule.interface(COMPONENT_INTERFACE))
            .and_then(|rule| rule.member("globalShortcutPressed"))
            .map(zbus::match_rule::Builder::build),
    );
    let registrar = bus.clone();

    owners
        .and_then(|owners| keys.map(|keys| (owners, keys)))
        .map(|(owners, keys)| {
            thread::spawn(move || {
                try_register(&registrar);
                // A message that cannot be read is skipped; the stream ends with
                // the connection.
                for message in owners.flatten() {
                    // A signal of another shape names no owner: no new owner.
                    let owner = message
                        .body()
                        .deserialize::<(String, String, String)>()
                        .map(|(_, _, owner)| owner)
                        .unwrap_or_default();

                    if !owner.is_empty() {
                        try_register(&registrar);
                    }
                }
            });
            thread::spawn(move || {
                for message in keys.flatten() {
                    // A signal of another shape is no key of this program.
                    if message
                        .body()
                        .deserialize::<(String, String, i64)>()
                        .is_ok_and(|(component, action, _)| {
                            component == COMPONENT && action == ACTION
                        })
                    {
                        pressed();
                    }
                }
            });
        })
}

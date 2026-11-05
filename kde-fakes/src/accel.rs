//! A fake of kglobalacceld (`org.kde.kglobalaccel` on the session bus): the
//! actions that a program registers, and the press of a shortcut.

use std::sync::{Arc, Mutex, PoisonError};

use zbus::blocking::Connection;
use zbus::interface;
use zbus::zvariant::OwnedObjectPath;

/// The object of the daemon.
const PATH: &str = "/kglobalaccel";

/// What the programs asked for.
#[derive(Debug, Default)]
struct Asked {
    registered: Vec<Vec<String>>,
    components: Vec<String>,
    keys: Vec<(Vec<String>, u32, usize)>,
}

struct Daemon(Arc<Mutex<Asked>>);

fn lock(asked: &Mutex<Asked>) -> std::sync::MutexGuard<'_, Asked> {
    asked.lock().unwrap_or_else(PoisonError::into_inner)
}

#[interface(name = "org.kde.KGlobalAccel")]
impl Daemon {
    #[zbus(name = "doRegister")]
    fn do_register(&self, action: Vec<String>) {
        lock(&self.0).registered.push(action);
    }

    #[zbus(name = "setShortcutKeys")]
    fn set_shortcut_keys(
        &self,
        action: Vec<String>,
        keys: Vec<(Vec<i32>,)>,
        flags: u32,
    ) -> Vec<(Vec<i32>,)> {
        lock(&self.0).keys.push((action, flags, keys.len()));
        keys
    }

    #[zbus(name = "getComponent")]
    fn get_component(&self, unique: &str) -> OwnedObjectPath {
        lock(&self.0).components.push(unique.to_owned());
        OwnedObjectPath::try_from(component_path(unique)).expect("a path")
    }
}

/// The object of a component, as kglobalacceld names it.
#[must_use]
pub fn component_path(unique: &str) -> String {
    format!("/component/{}", unique.replace('-', "_"))
}

/// The fake daemon.
pub struct Accel {
    address: String,
    connection: Option<Connection>,
    asked: Arc<Mutex<Asked>>,
}

impl Accel {
    /// A daemon on the bus at this address.
    #[must_use]
    pub fn new(address: &str) -> Self {
        let mut accel = Self::absent(address);

        accel.start();
        accel
    }

    /// A daemon that is not there yet; refer to [`Accel::start`].
    #[must_use]
    pub fn absent(address: &str) -> Self {
        Self {
            address: address.to_owned(),
            connection: None,
            asked: Arc::new(Mutex::new(Asked::default())),
        }
    }

    /// Start the daemon (again): it knows nothing of the earlier actions.
    ///
    /// # Panics
    ///
    /// If the bus does not take the service.
    pub fn start(&mut self) {
        self.connection = None;
        *lock(&self.asked) = Asked::default();
        self.connection = Some(
            zbus::blocking::connection::Builder::address(self.address.as_str())
                .and_then(|builder| builder.serve_at(PATH, Daemon(Arc::clone(&self.asked))))
                .and_then(|builder| builder.name("org.kde.kglobalaccel"))
                .and_then(zbus::blocking::connection::Builder::build)
                .expect("kglobalacceld service"),
        );
    }

    /// Stop the daemon.
    pub fn stop(&mut self) {
        self.connection = None;
    }

    /// The action ids (component, action, friendly names) registered so far.
    #[must_use]
    pub fn registered(&self) -> Vec<Vec<String>> {
        lock(&self.asked).registered.clone()
    }

    /// The components that were asked for.
    #[must_use]
    pub fn components(&self) -> Vec<String> {
        lock(&self.asked).components.clone()
    }

    /// The calls that set keys: the action id, the flags and how many keys.
    #[must_use]
    pub fn keys(&self) -> Vec<(Vec<String>, u32, usize)> {
        lock(&self.asked).keys.clone()
    }

    /// The user presses the shortcut of an action.
    ///
    /// # Panics
    ///
    /// If the daemon is not running or the signal cannot be sent.
    pub fn press(&self, component: &str, action: &str) {
        self.connection
            .as_ref()
            .expect("a running daemon")
            .emit_signal(
                None::<&str>,
                component_path(component),
                "org.kde.kglobalaccel.Component",
                "globalShortcutPressed",
                &(component, action, 0_i64),
            )
            .expect("signal");
    }
}

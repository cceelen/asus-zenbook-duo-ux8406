//! A private D-Bus bus for one test: a `dbus-daemon` of its own, in a
//! temporary directory, stopped when the value is dropped. The programs
//! under test get its address as `DBUS_SESSION_BUS_ADDRESS` and
//! `DBUS_SYSTEM_BUS_ADDRESS`.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use tempfile::TempDir;

/// The configuration of the bus: anyone may own any name and send to
/// anyone.
const CONFIGURATION: &str = r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path=@PATH@</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#;

/// A running bus.
pub struct Bus {
    _dir: TempDir,
    daemon: Child,
    address: String,
}

impl Bus {
    /// Start a bus.
    ///
    /// # Panics
    ///
    /// If `dbus-daemon` cannot be started; the tests need it.
    #[must_use]
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("temporary directory");
        let socket = dir.path().join("bus");
        let configuration = dir.path().join("bus.conf");

        std::fs::write(
            &configuration,
            CONFIGURATION.replace("@PATH@", &socket.to_string_lossy()),
        )
        .expect("bus configuration");

        let mut daemon = Command::new("dbus-daemon")
            .arg(format!("--config-file={}", configuration.display()))
            .args(["--nofork", "--print-address"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("dbus-daemon (package dbus-daemon or dbus) for the tests");
        let mut address = String::new();

        BufReader::new(daemon.stdout.take().expect("stdout"))
            .read_line(&mut address)
            .expect("bus address");
        Self {
            _dir: dir,
            daemon,
            address: address.trim().to_owned(),
        }
    }

    /// The address to connect to.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// A new connection to the bus.
    ///
    /// # Panics
    ///
    /// If the bus does not take it.
    pub fn connect(&self) -> zbus::blocking::Connection {
        zbus::blocking::connection::Builder::address(self.address())
            .and_then(zbus::blocking::connection::Builder::build)
            .expect("connection to the private bus")
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

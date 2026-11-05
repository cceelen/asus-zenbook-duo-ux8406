//! The program as a whole, against a fake KWin and a fake iio-sensor-proxy
//! on a private bus: one module per topic. Each test runs the program in a
//! session of its own, with a configuration directory of its own.

mod applet;
mod errors;
mod policy;
mod process;
mod restore;
mod screens;
mod sensor;
mod shortcut;
mod turns;
mod worker;

use std::path::PathBuf;
use std::process::Command;

use asus_ux8406_kde_fakes::{Bus, Compositor, Output, Program, Sensor, Style, program};
use tempfile::TempDir;
use zbus::zvariant::OwnedValue;

/// The program.
const PROGRAM: &str = env!("CARGO_BIN_EXE_plasma-builtin-screen-rotation");
/// The name, object and interface of its service.
const SERVICE: &str = "io.github.cceelen.BuiltinScreenRotation";
const PATH: &str = "/io/github/cceelen/BuiltinScreenRotation";

const UPPER: &str = "eDP-1";
const LOWER: &str = "eDP-2";

/// The two panels as the UX8406CA has them: the lower one below the upper
/// one, and the primary.
fn panels() -> Vec<Output> {
    vec![
        Output::panel(UPPER, 0, 0),
        Output {
            priority: 1,
            ..Output::panel(LOWER, 0, 1440)
        },
    ]
}

/// A session: KWin with these outputs, the bus, the sensor service if it
/// is given an orientation, and a configuration directory.
struct Session {
    compositor: Compositor,
    bus: Bus,
    sensor: Option<Sensor>,
    config: TempDir,
}

impl Session {
    fn new(outputs: Vec<Output>, orientation: Option<&str>) -> Self {
        Self::of_style(Style::Registry, outputs, orientation)
    }

    fn of_style(style: Style, outputs: Vec<Output>, orientation: Option<&str>) -> Self {
        Self::on(Bus::new(), style, outputs, orientation)
    }

    fn on(bus: Bus, style: Style, outputs: Vec<Output>, orientation: Option<&str>) -> Self {
        let compositor = Compositor::new(style, outputs);
        let sensor = orientation.map(|orientation| Sensor::new(bus.address(), orientation));

        Self {
            compositor,
            bus,
            sensor,
            config: TempDir::new().expect("directory"),
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = program::command(PROGRAM, Some(&self.compositor), Some(&self.bus));

        command
            .env("XDG_CONFIG_HOME", self.config.path())
            .args(args);
        command
    }

    /// The program, once it is ready.
    fn start(&self) -> Program {
        let program = Program::start(self.command(&[]));

        program.wait_log("ready");
        program
    }

    /// The configuration file of the program.
    fn file(&self) -> PathBuf {
        self.config
            .path()
            .join("plasma-builtin-screen-rotation.toml")
    }

    fn config(&self) -> String {
        std::fs::read_to_string(self.file()).unwrap_or_default()
    }

    fn sensor(&self) -> &Sensor {
        self.sensor.as_ref().expect("a sensor")
    }

    /// Call a method of the program's service.
    fn call<B>(&self, method: &str, body: &B) -> zbus::Result<()>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        self.bus
            .connect()
            .call_method(Some(SERVICE), PATH, Some(SERVICE), method, body)
            .map(drop)
    }

    /// A property of the program's service.
    fn property(&self, name: &str) -> OwnedValue {
        self.bus
            .connect()
            .call_method(
                Some(SERVICE),
                PATH,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(SERVICE, name),
            )
            .and_then(|reply| reply.body().deserialize::<OwnedValue>())
            .expect("property")
    }

    fn control(&self) -> String {
        String::try_from(self.property("Control")).expect("text")
    }

    /// Where an output is and how it is turned.
    fn place(&self, name: &str) -> (i32, i32, u32) {
        let output = self.compositor.output(name);

        (output.x, output.y, output.transform)
    }

    /// Wait until both panels are at these places.
    fn wait_places(&self, program: &Program, upper: (i32, i32, u32), lower: (i32, i32, u32)) {
        program.wait_for(&format!("{UPPER} {upper:?}, {LOWER} {lower:?}"), || {
            self.place(UPPER) == upper && self.place(LOWER) == lower
        });
    }
}

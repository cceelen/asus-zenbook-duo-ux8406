//! The program as a whole, against a fake KWin (outputs on Wayland, input
//! devices on D-Bus) on a private bus: one module per topic. Each test runs
//! the program in a session of its own.

mod errors;
mod lower_screen;
mod touch;

use asus_ux8406_kde_fakes::input::Device;
use asus_ux8406_kde_fakes::{Bus, Compositor, Input, Output, Program, Style, program};

/// The program.
const PROGRAM: &str = env!("CARGO_BIN_EXE_asus-ux8406-kde-keys");
/// The sysfs tree of a UX8406CA.
const UX8406CA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ux8406ca");
/// The name and object of the program's service.
const SERVICE: &str = "io.github.cceelen.AsusUX8406.Keys";
const PATH: &str = "/io/github/cceelen/AsusUX8406/Keys";

const UPPER: &str = "eDP-1";
const LOWER: &str = "eDP-2";

/// The two panels, one below the other, the upper one the primary.
fn panels() -> Vec<Output> {
    vec![
        Output {
            priority: 1,
            ..Output::panel(UPPER, 0, 0)
        },
        Output::panel(LOWER, 0, 1440),
    ]
}

/// The touchscreens and pens of the two panels, on no output yet.
fn touch() -> Vec<Device> {
    vec![
        Device::touchscreen("event10", 0x4447),
        Device::pen("event11", 0x4447),
        Device::touchscreen("event12", 0x4448),
        Device::pen("event13", 0x4448),
    ]
}

/// A session: KWin with these outputs and input devices, and the bus.
struct Session {
    compositor: Compositor,
    bus: Bus,
    input: Option<Input>,
}

impl Session {
    fn new(outputs: Vec<Output>, devices: Option<Vec<Device>>) -> Self {
        let compositor = Compositor::new(Style::Registry, outputs);
        let bus = Bus::new();
        let input = devices.map(|devices| Input::new(bus.address(), devices));

        Self {
            compositor,
            bus,
            input,
        }
    }

    /// The program on the UX8406CA, once it is ready.
    fn start(&self) -> Program {
        let mut command = program::command(PROGRAM, Some(&self.compositor), Some(&self.bus));

        command.args(["--root", UX8406CA]);

        let program = Program::start(command);

        program.wait_log("ready");
        program
    }

    /// Press the key right of F12, as the KWin script does.
    fn press(&self) {
        self.bus
            .connect()
            .call_method(Some(SERVICE), PATH, Some(SERVICE), "ToggleLowerScreen", &())
            .expect("the program's service");
    }

    fn input(&self) -> &Input {
        self.input.as_ref().expect("input devices")
    }
}

//! Fakes for the tests of the KDE parts: KWin's outputs on Wayland, and the
//! D-Bus services that the programs use, on a private bus. Not installed;
//! only the tests use this crate.

pub mod accel;
pub mod bus;
pub mod compositor;
pub mod input;
pub mod program;
pub mod protocol;
pub mod sensor;

pub use accel::Accel;
pub use bus::Bus;
pub use compositor::{Compositor, Output, Style};
pub use input::Input;
pub use program::Program;
pub use sensor::Sensor;

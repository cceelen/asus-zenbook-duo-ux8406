//! The command line of plasma-builtin-screen-rotation.

use clap::{Parser, Subcommand};

/// What the command line asks for.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "plasma-builtin-screen-rotation",
    version,
    about = "Turns the built-in screens with the laptop in KDE Plasma"
)]
pub struct Cli {
    /// What to do; without one, follow the orientation of the laptop.
    #[command(subcommand)]
    pub action: Option<Action>,
}

/// What to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Subcommand)]
pub enum Action {
    /// Follow the orientation of the laptop until the session ends.
    Run,
    /// Stop the program of this session and give KWin's own auto-rotate
    /// setting back to the built-in screens. Run it before the package is
    /// removed.
    Restore,
}

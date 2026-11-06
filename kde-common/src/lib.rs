//! What the KDE parts share: the display layouts of the built-in panels, and
//! the client of KWin's output management.
//!
//! [`geometry`], [`layout`], [`rotation`], [`hold`] and [`touch`] are the logic of the
//! GNOME Shell extensions (`gnome-common/state.js`, `gnome-keys/extension/`,
//! `gnome-rotation/extension/` of cceelen/gnome-extension) in Rust, on plain data. Both are checked
//! against the same cases, in `tests/vectors/`.

pub mod daemon;
pub mod geometry;
pub mod hold;
pub mod layout;
pub mod output;
pub mod protocol;
pub mod rotation;
pub mod touch;

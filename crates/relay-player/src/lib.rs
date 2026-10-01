//! The standalone player: an exported macro is this program with the macro
//! appended (see `relay_core::format::bundle`). This library is the logic
//! that needs no window, so it's tested anywhere; `main.rs` is the Win32 shell.

pub mod args;
pub mod exit;
pub mod place;
pub mod status;

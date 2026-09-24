//! Relay core: the macro model, step grouping, edit operations, timeline math,
//! the session state machine, schedules, trigger edge detectors and the `.rly`
//! file format. This crate has no OS or Tauri dependencies.

pub mod keys;
pub mod model;
pub mod steps;

//! Playing a macro, shared by the app and the exported player: the
//! [`engine`] and its thread, and [`finder`], which looks for images on the
//! real screen.

pub mod engine;
pub mod finder;
pub mod plan;

pub use engine::{
    Engine, EngineCmd, EngineHandle, ImageFinder, PixelReader, PlayPlan, PlaybackSink, RunReport, Tick, TimingStats,
    spawn,
};

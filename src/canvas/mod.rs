pub mod core;
pub mod actions;
pub mod conditions;
pub mod helpers;
pub mod physics;
pub(crate) mod gravity;
pub mod events;
pub mod location;
pub mod crystalline_bridge;

// Flatten the public surface: callers use `crate::canvas::Canvas` etc.
pub use core::{Canvas, CanvasMode, CanvasLayout};
// physics helper needed by object update path
pub(crate) use physics::rotation_adjusted_offset;
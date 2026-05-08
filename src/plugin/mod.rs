//! Quartz plugin system.
//!
//! Each plugin is a self-contained directory. No cross-plugin imports.
//!
//! ## Plugins
//! - [`background`]         — `BackgroundPlugin`, `LayeredBackground`, `BackgroundLayer`
//! - [`terrain_collision`]  — `TerrainCollisionPlugin` — pixel-outline SAT, groups, caching
//! - [`save_game`]          — `SaveGamePlugin` — versioned JSON save/load
//! - [`grapple`]            — `GrapplePlugin`, `GrappleConstraint`, rope/spring types
//!
//! ## Lifecycle
//! - `on_init`        — called once immediately when the plugin is registered.
//! - `on_update`      — called every frame after game-object update events (before physics).
//! - `on_post_update` — called every frame after the physics step.
//!
//! ## Borrow Safety
//! The engine uses `std::mem::take` to dispatch to plugins, so plugins
//! may freely call any `canvas` method during their hooks.
//!
//! ---
//!
//! ## Plugin Authoring Guide
//!
//! ### Minimum requirements for a Quartz plugin
//!
//! 1. A struct that implements `QuartzPlugin`.
//! 2. `fn name(&self) -> &str` — returns a unique, stable string identifier.
//! 3. All other hooks are optional (default = no-op or `false`).
//!
//! ```rust
//! pub struct MyPlugin;
//!
//! impl QuartzPlugin for MyPlugin {
//!     fn name(&self) -> &str { "my_plugin" }
//! }
//! ```
//!
//! ### Standard file layout
//!
//! ```
//! src/plugin/my_plugin/     ← plugin directory (separate git repo, gitignored)
//!   mod.rs                  ← pub struct MyPlugin, impl QuartzPlugin, pub enum MyCommand
//! ```
//!
//! Add to this file:
//! ```rust
//! // ── Sub-modules ──────────────
//! pub mod my_plugin;
//! ```
//! And add a line to the `## Plugins` list in the module doc above.
//!
//! If you need the plugin's public types available via `use quartz::prelude::*`,
//! re-export them in `quartz/src/lib.rs` under the prelude section.
//!
//! ### Standard command pattern
//!
//! Define a typed command enum inside your plugin's `mod.rs`:
//!
//! ```rust
//! #[derive(Clone, Debug)]
//! pub enum MyCommand {
//!     DoThing { target: Target },
//!     SetValue { target: Target, value: f32 },
//! }
//! ```
//!
//! Handle it in `on_call`:
//!
//! ```rust
//! fn on_call(&mut self, canvas: &mut Canvas, payload: &dyn std::any::Any) -> bool {
//!     if let Some(cmd) = payload.downcast_ref::<MyCommand>() {
//!         match cmd {
//!             MyCommand::DoThing { target } => { /* … */ true }
//!             MyCommand::SetValue { target, value } => { /* … */ true }
//!         }
//!     } else {
//!         false
//!     }
//! }
//! ```
//!
//! Game code dispatches via:
//! ```rust
//! use quartz::plugin::my_plugin::MyCommand;
//! use std::sync::Arc;
//!
//! canvas.run(Action::PluginCall {
//!     name:    "my_plugin".into(),
//!     payload: Arc::new(MyCommand::SetValue {
//!         target: Target::ByName("player".into()),
//!         value:  42.0,
//!     }),
//! });
//! ```
//!
//! ### Notes on the plugin/ directory
//!
//! - Each plugin directory is a **separately maintained git repository**.
//!   They are cloned into `src/plugin/<name>/` and are gitignored by the
//!   quartz repo. Do **not** modify `.gitignore` — the existing pattern
//!   `/src/plugin` covers all present and future plugin directories.
//! - No cross-plugin imports. Plugins must not depend on each other.
//! - Plugin types that need to be in the prelude must be explicitly
//!   re-exported in `src/lib.rs`. The `QuartzPlugin` trait itself is
//!   available via `use quartz::prelude::*` already.

// ── Sub-modules ──────────────────────────────────────────────────────────────

/// Background generation and layer compositing.
pub mod background;
/// Pixel-outline SAT collision — terrain, groups, dynamic objects, outline cache.
pub mod terrain_collision;
/// Versioned JSON save / load.
pub mod save_game;
/// Grapple, rope, and spring constraint types.
pub mod grapple;

use super::canvas::Canvas;

// ── Public trait ────────────────────────────────────────────────────────────

/// A plugin that hooks into the Quartz engine tick loop.
///
/// Implement this trait and register an instance with
/// [`Canvas::add_plugin`] to receive per-frame callbacks.
pub trait QuartzPlugin {
    /// Unique string identifier for this plugin.
    /// Used for `Action::RunPlugin` dispatch and debug output.
    fn name(&self) -> &str;

    /// Called every frame **after** game-object update events but
    /// **before** the physics step.
    ///
    /// The default implementation is a no-op.
    #[allow(unused_variables)]
    fn on_update(&mut self, canvas: &mut Canvas, dt: f32) {}

    /// Called every frame **after** the physics step (Crystalline or legacy).
    ///
    /// The default implementation is a no-op.
    #[allow(unused_variables)]
    fn on_post_update(&mut self, canvas: &mut Canvas, dt: f32) {}

    /// Called once immediately after the plugin is registered with
    /// [`Canvas::add_plugin`]. Use this for one-time setup that requires
    /// access to the canvas (e.g. scanning existing objects).
    ///
    /// The default implementation is a no-op.
    #[allow(unused_variables)]
    fn on_init(&mut self, canvas: &mut Canvas) {}

    /// Called when `Action::RunPlugin { name, data }` targets this plugin.
    /// Return `true` if the action was handled.
    ///
    /// The default implementation ignores the action and returns `false`.
    #[allow(unused_variables)]
    fn on_action(&mut self, canvas: &mut Canvas, data: &str) -> bool { false }

    /// Called once per frame **from inside** the Crystalline physics step,
    /// after `apply_physics_result()` and grapple constraint enforcement,
    /// but before the step returns to the main tick loop.
    ///
    /// This is the correct place for any plugin that needs to apply
    /// position-level corrections (custom constraints, extra restitution,
    /// etc.) after the solver has written results back to game objects.
    ///
    /// The collision pairs detected by Crystalline this frame are available
    /// via [`Canvas::last_collision_pairs`] as `(object_name_a, object_name_b)`.
    ///
    /// Only called when Crystalline physics is enabled. Not called during the
    /// legacy `handle_collisions()` path.
    ///
    /// The default implementation is a no-op.
    #[allow(unused_variables)]
    fn on_post_solve(&mut self, canvas: &mut Canvas, dt: f32) {}

    /// Called to evaluate `Condition::Plugin { name, arg }` for this plugin.
    /// Return the boolean result of the condition.
    ///
    /// The default implementation always returns `false`.
    #[allow(unused_variables)]
    fn on_condition(&self, canvas: &Canvas, arg: Option<&str>) -> bool { false }

    /// Called when `Action::PluginCall { name, payload }` targets this plugin.
    /// Plugins may downcast `payload` using `std::any::Any` to retrieve
    /// typed command structures. Return `true` if the call was handled.
    ///
    /// The default implementation always returns `false`.
    #[allow(unused_variables)]
    fn on_call(&mut self, canvas: &mut Canvas, payload: &dyn std::any::Any) -> bool { false }
}

// ── Internal registry ────────────────────────────────────────────────────────

/// Holds the registered plugins for a Canvas.
///
/// Implements `Clone` by returning an empty registry — plugin state is
/// intentionally not cloned since plugins register themselves at startup.
/// This allows `Canvas` to keep `#[derive(Clone)]` without restriction.
pub struct PluginRegistry {
    pub(crate) plugins: Vec<Box<dyn QuartzPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

/// Cloning a canvas does not clone plugin state.
/// Plugins are re-registered by the game's initialization logic when needed.
impl Clone for PluginRegistry {
    fn clone(&self) -> Self {
        Self { plugins: Vec::new() }
    }
}

//! Expandable module contract.
//!
//! Any future subsystem (climatology, AI, race director, ...) becomes a `SimModule`
//! registered on the facade. The core orchestration loop (`CoreFacade::step`) is
//! agnostic to module internals: it just ticks every module and collects its
//! snapshot contribution. Adding a module NEVER changes the core loop or the core
//! C ABI — it only adds namespaced `f90_core_module_<name>_*` symbols.

use crate::frame::CoreFrame;

/// A deterministic module failure. The orchestrator logs it and keeps going.
#[derive(Debug)]
pub struct ModuleError(pub String);

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "module error: {}", self.0)
    }
}

impl std::error::Error for ModuleError {}

/// Signals a module can emit into the orchestrator this tick.
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleSignal {
    /// Reserved for future cross-module effects (weather -> tire grip, AI -> inputs).
    None,
}

/// Read-only orchestration context handed to each `SimModule::tick`.
pub struct ModuleCtx<'a> {
    /// Fixed timestep of this tick (seconds).
    pub fixed_dt: f64,
    /// Core clock in milliseconds. DETERMINISM: this is the only clock a module may
    /// use — reading the wall clock in a module is forbidden (CI-checks `std::time`).
    pub clock_ms: i64,
    /// Immutable view of this tick's orchestrated outcome (physics+audio) BEFORE the
    /// module contributions are appended. Modules read `latest` to react to state.
    pub latest: &'a CoreFrame,
    /// Sink for emitting signals back into the orchestrator.
    pub emit: &'a mut dyn FnMut(ModuleSignal),
}

/// A simulable subsystem owned by the facade.
pub trait SimModule: Send {
    /// Stable module id, used as the `f90_core_module_<name>_*` prefix and the
    /// snapshot key. ASCII, no spaces.
    fn name(&self) -> &'static str;
    /// Advance the module one fixed step. Called once per `CoreFacade::step`.
    fn tick(&mut self, ctx: &mut ModuleCtx) -> Result<(), ModuleError>;
    /// Deterministic, serializable contribution merged into the facade snapshot.
    fn snapshot(&self) -> Vec<u8>;
    /// Restore the module to its pristine post-construction state.
    fn reset(&mut self);
}

/// Ordered set of modules owned by the facade.
pub struct ModuleRegistry {
    modules: Vec<Box<dyn SimModule>>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self { modules: Vec::new() }
    }

    pub fn register(&mut self, module: Box<dyn SimModule>) {
        self.modules.push(module);
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.name()).collect()
    }

    /// Tick every module. A failing module is logged and skipped (orchestrator
    /// resilience) — it never aborts the loop.
    pub fn tick_all(&mut self, ctx: &mut ModuleCtx) {
        for m in self.modules.iter_mut() {
            if let Err(e) = m.tick(ctx) {
                eprintln!("[formula90_core] module '{}' tick failed: {e}", m.name());
            }
        }
    }

    pub fn snapshots(&self) -> Vec<(String, Vec<u8>)> {
        self.modules
            .iter()
            .map(|m| (m.name().to_string(), m.snapshot()))
            .collect()
    }

    pub fn reset_all(&mut self) {
        for m in self.modules.iter_mut() {
            m.reset();
        }
    }
}

//! AI/race-director module — EXAMPLE STUB proving the extension path.
//!
//! Real implementation will be its own crate (`game/ai/engine`) registered here.
//! For now it is a deterministic no-op observing the latest orchestrated frame and
//! emitting a tiny bincode payload (planned: directive outputs the C++/GDScript loop
//! applies on top of driver input).

use bincode;
use serde::{Deserialize, Serialize};

use crate::module::{ModuleCtx, ModuleError, SimModule};

/// Planned AI directive this stub will one day emit (e.g. rubber-band balance,
/// rival aggression, rubber-banding assist). Kept serializable for snapshot parity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiDirective {
    pub clock_ms: i64,
    pub target_speed_kmh: Option<f64>,
    pub throttle_override: Option<f64>,
}

pub struct AiStub {
    state: AiDirective,
}

impl AiStub {
    pub fn new() -> Self {
        Self {
            state: AiDirective {
                clock_ms: 0,
                target_speed_kmh: None,
                throttle_override: None,
            },
        }
    }
}

impl Default for AiStub {
    fn default() -> Self {
        Self::new()
    }
}

impl SimModule for AiStub {
    fn name(&self) -> &'static str {
        "ai"
    }

    fn tick(&mut self, ctx: &mut ModuleCtx) -> Result<(), ModuleError> {
        self.state.clock_ms = ctx.clock_ms;
        // The stub can observe the orchestrated frame (physics + audio) for free:
        // `ctx.latest.rpm`, `ctx.latest.speed_kmh`, ... — deterministic read of what
        // the orchestrator already computed this tick.
        let _ = ctx.latest.rpm;
        Ok(())
    }

    fn snapshot(&self) -> Vec<u8> {
        bincode::serialize(&self.state).unwrap_or_default()
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

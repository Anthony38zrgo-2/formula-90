//! Climatology module — EXAMPLE STUB proving the extension path.
//!
//! Real implementation will be its own crate (`game/weather/engine`) registered here.
//! For now it is a deterministic no-op: it records the core clock and emits a tiny
//! postcard payload. It must never read the wall clock (determinism contract).

use serde::{Deserialize, Serialize};

use crate::module::{ModuleCtx, ModuleError, SimModule};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeatherState {
    /// Core clock (ms) of the last tick the module observed.
    pub clock_ms: i64,
    /// Placeholder conditions — replaced by the real climatology module.
    pub ambient_temp_c: f32,
    pub wind_ms: f32,
    pub raining: bool,
}

pub struct WeatherStub {
    state: WeatherState,
}

impl WeatherStub {
    pub fn new() -> Self {
        Self {
            state: WeatherState {
                clock_ms: 0,
                ambient_temp_c: 22.0,
                wind_ms: 0.0,
                raining: false,
            },
        }
    }
}

impl Default for WeatherStub {
    fn default() -> Self {
        Self::new()
    }
}

impl SimModule for WeatherStub {
    fn name(&self) -> &'static str {
        "weather"
    }

    fn tick(&mut self, ctx: &mut ModuleCtx) -> Result<(), ModuleError> {
        // Deterministic: the module may only use the core clock.
        self.state.clock_ms = ctx.clock_ms;
        // Intentionally no wall-clock / RNG / IO here.
        Ok(())
    }

    fn snapshot(&self) -> Vec<u8> {
        postcard::to_allocvec(&self.state).unwrap_or_default()
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

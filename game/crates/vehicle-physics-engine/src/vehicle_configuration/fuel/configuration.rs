use super::*;

/// Onboard fuel model. `vehicle_mass` stays the DRY mass; the total mass and the
/// effective weight distribution are derived from the fuel remaining and the
/// tank's local position so combustion never rewrites the profile's dry specs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FuelConfig {
    pub capacity_kg: f64,
    pub initial_kg: f64,
    /// Tank centre of mass in vehicle-local geometric space (-Z forward, +Y up).
    pub tank_position_local_m: Vec3,
    pub brake_specific_consumption_kg_per_kwh: f64,
    pub idle_consumption_kg_per_hour: f64,
    /// Plan reference (kg/lap) used by the HUD delta: comparing the measured
    /// average against this value flags a stint that is burning more or less
    /// fuel than the initial load assumed.
    pub estimated_lap_consumption_kg: f64,
    /// Plan reference lap duration (s) used to project the instantaneous burn
    /// rate into kilograms per lap before the first lap time is known.
    pub reference_lap_time_s: f64,
    /// Runtime state: kilograms currently in the tank.
    pub current_kg: f64,
    /// Runtime state: kilograms burned since the last refill.
    pub consumed_kg: f64,
}

impl Default for FuelConfig {
    fn default() -> Self {
        Self {
            capacity_kg: 0.0,
            initial_kg: 0.0,
            tank_position_local_m: Vec3::ZERO,
            brake_specific_consumption_kg_per_kwh: 0.0,
            idle_consumption_kg_per_hour: 0.0,
            estimated_lap_consumption_kg: 0.0,
            reference_lap_time_s: 0.0,
            current_kg: 0.0,
            consumed_kg: 0.0,
        }
    }
}

impl FuelConfig {
    /// A zero-capacity tank disables the whole model (legacy profiles).
    pub fn is_enabled(&self) -> bool {
        self.capacity_kg > 0.0
    }

    pub fn effective_current_kg(&self) -> f64 {
        if self.is_enabled() {
            self.current_kg.clamp(0.0, self.capacity_kg)
        } else {
            0.0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_enabled() && self.effective_current_kg() <= 0.0
    }

    pub fn refill_to_initial(&mut self) {
        self.current_kg = if self.is_enabled() {
            self.initial_kg.clamp(0.0, self.capacity_kg)
        } else {
            0.0
        };
        self.consumed_kg = 0.0;
    }

    /// Pit-service target: leaves exactly `target_kg` in the tank, clamped to
    /// the profile limits. A non-finite request keeps the current load. The
    /// consumed counter restarts so the HUD plan tracks the new stint.
    pub fn set_current_kg(&mut self, target_kg: f64) {
        if !self.is_enabled() {
            self.current_kg = 0.0;
            self.consumed_kg = 0.0;
            return;
        }
        if target_kg.is_finite() {
            self.current_kg = target_kg.clamp(0.0, self.capacity_kg);
            self.consumed_kg = 0.0;
        }
    }
}

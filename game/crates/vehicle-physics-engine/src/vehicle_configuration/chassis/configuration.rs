use serde::{Deserialize, Serialize};

use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChassisConfiguration {
    pub vehicle_name: String,
    pub vehicle_mass: f64,
    pub vehicle_mass_excludes_wheel_assemblies: bool,
    pub front_weight_distribution: f64,
    pub center_of_gravity_height_offset: f64,
    pub inertia_multipliers: Vec3,
    pub wheelbase: f64,
    pub front_track: f64,
    pub rear_track: f64,
    #[serde(default)]
    pub physical_mass_inventory: Option<crate::vehicle_mass_inventory::VehicleMassInventory>,
}

impl ChassisConfiguration {
    /// Dry mass plus the fuel currently in the tank.
    pub fn total_vehicle_mass(&self, tires: &TiresConfiguration, fuel: &FuelConfig) -> f64 {
        self.complete_dry_vehicle_mass(tires) + fuel.effective_current_kg()
    }

    pub fn complete_dry_vehicle_mass(&self, tires: &TiresConfiguration) -> f64 {
        self.vehicle_mass + self.additional_wheel_assembly_mass(tires)
    }

    pub fn additional_wheel_assembly_mass(&self, tires: &TiresConfiguration) -> f64 {
        if self.vehicle_mass_excludes_wheel_assemblies {
            2.0 * (tires.front_wheel_mass + tires.rear_wheel_mass)
        } else {
            0.0
        }
    }

    /// Front axle share of the total mass with the tank load present. The combined
    /// centre of mass blends the dry centre of mass with the tank position, so the
    /// distribution migrates toward the tank as the tank drains or fills.
    pub fn effective_front_weight_distribution(
        &self,
        tires: &TiresConfiguration,
        fuel: &FuelConfig,
    ) -> f64 {
        let fuel_kg = fuel.effective_current_kg();
        if !self.vehicle_mass_excludes_wheel_assemblies && fuel_kg <= 0.0 {
            return self.front_weight_distribution;
        }
        let total_mass = self.total_vehicle_mass(tires, fuel);
        if total_mass <= 1e-9 {
            return self.front_weight_distribution;
        }
        let fuel_front_distribution = if self.wheelbase.abs() > 1e-9 {
            0.5 - fuel.tank_position_local_m.z / self.wheelbase
        } else {
            self.front_weight_distribution
        };
        let additional_front_wheel_mass = if self.vehicle_mass_excludes_wheel_assemblies {
            2.0 * tires.front_wheel_mass
        } else {
            0.0
        };
        ((self.vehicle_mass * self.front_weight_distribution
            + additional_front_wheel_mass
            + fuel_kg * fuel_front_distribution)
            / total_mass)
            .clamp(0.0, 1.0)
    }

    /// Helper to get mass supported by a single wheel at static rest.
    pub fn mass_over_wheel(
        &self,
        tires: &TiresConfiguration,
        fuel: &FuelConfig,
        wheel: WheelIndex,
    ) -> f64 {
        let total_mass = self.total_vehicle_mass(tires, fuel);
        let front_distribution = self.effective_front_weight_distribution(tires, fuel);
        if wheel.is_front() {
            total_mass * front_distribution * 0.5
        } else {
            total_mass * (1.0 - front_distribution) * 0.5
        }
    }
}

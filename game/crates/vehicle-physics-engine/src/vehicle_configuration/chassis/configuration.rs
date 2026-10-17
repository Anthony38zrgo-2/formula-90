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
    #[serde(skip)]
    pub authored_wheel_centers_local_metres: Option<[Vec3; 4]>,
}

impl ChassisConfiguration {
    /// Binds the authored hub centers from the physical suspension geometry so
    /// mass, load split and inertia share one longitudinal frame. Profiles that
    /// do not opt into authored wheel centers keep the legacy symmetric frame.
    pub fn bind_authored_wheel_centers(&mut self, suspension: &SuspensionConfiguration) {
        self.authored_wheel_centers_local_metres =
            if suspension.use_authored_wheel_centers {
                suspension
                    .geometric_suspension
                    .as_ref()
                    .map(|geometry| WheelIndex::ALL.map(|wheel| geometry.corners.get(wheel).hub_center))
            } else {
                None
            };
    }

    pub fn uses_authored_wheel_frame(&self) -> bool {
        self.authored_wheel_centers_local_metres.is_some()
    }

    pub fn wheel_center_local(&self, wheel: WheelIndex) -> Vec3 {
        if let Some(centers) = self.authored_wheel_centers_local_metres {
            return centers[wheel as usize];
        }
        Vec3::new(
            self.wheel_lateral_offset(wheel),
            self.center_of_gravity_height_offset,
            self.axle_center_z(wheel),
        )
    }

    pub fn axle_center_z(&self, wheel: WheelIndex) -> f64 {
        if let Some(centers) = self.authored_wheel_centers_local_metres {
            let (left, right) = if wheel.is_front() {
                (WheelIndex::FrontLeft, WheelIndex::FrontRight)
            } else {
                (WheelIndex::RearLeft, WheelIndex::RearRight)
            };
            return 0.5
                * (centers[left as usize].z + centers[right as usize].z);
        }
        if wheel.is_front() {
            -self.wheelbase * 0.5
        } else {
            self.wheelbase * 0.5
        }
    }

    fn wheel_lateral_offset(&self, wheel: WheelIndex) -> f64 {
        let track = if wheel.is_front() {
            self.front_track
        } else {
            self.rear_track
        };
        if wheel.is_right() {
            track * 0.5
        } else {
            -track * 0.5
        }
    }

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

    fn wheel_assembly_mass(&self, tires: &TiresConfiguration, wheel: WheelIndex) -> f64 {
        if self.vehicle_mass_excludes_wheel_assemblies {
            if wheel.is_front() {
                tires.front_wheel_mass
            } else {
                tires.rear_wheel_mass
            }
        } else {
            0.0
        }
    }

    /// Base-body center of mass in the model frame. The declared front share is
    /// measured from the front axle, so the frame origin is not assumed to be the
    /// axle midpoint.
    pub fn base_center_of_mass_local(&self) -> Vec3 {
        let center_z =
            self.axle_center_z(WheelIndex::RearLeft) - self.front_weight_distribution * self.wheelbase;
        Vec3::new(0.0, self.center_of_gravity_height_offset, center_z)
    }

    pub fn center_of_mass_local(&self, tires: &TiresConfiguration, fuel: &FuelConfig) -> Vec3 {
        let total_mass = self.total_vehicle_mass(tires, fuel);
        if total_mass <= 1e-9 {
            return self.base_center_of_mass_local();
        }
        let mut weighted_center = self.base_center_of_mass_local() * self.vehicle_mass;
        if self.vehicle_mass_excludes_wheel_assemblies {
            for wheel in WheelIndex::ALL {
                weighted_center +=
                    self.wheel_center_local(wheel) * self.wheel_assembly_mass(tires, wheel);
            }
        }
        weighted_center += fuel.tank_position_local_m * fuel.effective_current_kg();
        let center = weighted_center / total_mass;
        Vec3::new(0.0, self.center_of_gravity_height_offset, center.z)
    }

    /// Front axle share of the total mass with the tank load present, derived
    /// from the combined center of mass in the same frame as the wheel centers.
    pub fn effective_front_weight_distribution(
        &self,
        tires: &TiresConfiguration,
        fuel: &FuelConfig,
    ) -> f64 {
        if self.wheelbase.abs() <= 1e-9 {
            return self.front_weight_distribution;
        }
        let center_z = self.center_of_mass_local(tires, fuel).z;
        ((self.axle_center_z(WheelIndex::RearLeft) - center_z) / self.wheelbase).clamp(0.0, 1.0)
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

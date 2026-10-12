use super::*;
use super::legacy_profile::JsonVehicleSpec;

/// Formula-90 vehicle configuration with GEVP-compatible suspension, tire and drivetrain semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
    pub schema_version: u32,
    pub chassis: ChassisConfiguration,
    pub fuel: FuelConfig,

    // Steering
    pub steering: SteeringConfiguration,

    // Powertrain & Gearing
    pub powertrain: PowertrainConfiguration,

    // Suspension Parameters
    pub suspension: SuspensionConfiguration,

    // Tires & Contact
    pub front_tire_radius: f64,
    pub rear_tire_radius: f64,
    pub front_tire_width: f64, // meters
    pub rear_tire_width: f64,  // meters
    pub front_wheel_mass: f64,
    pub rear_wheel_mass: f64,
    // Global fallback (used when a per-axle value is omitted in JSON).
    pub contact_patch: f64,
    // Per-axle contact patch (front/rear).
    pub front_contact_patch: f64,
    pub rear_contact_patch: f64,
    pub front_airborne_decay: f64,
    pub rear_airborne_decay: f64,
    pub brakes: BrakesConfiguration,

    // Tire pressure + thermal (pressure-aware carcass + 5-node thermal model)
    pub tire_pressure: TirePressureConfig,
    pub tire_thermal: TireThermalAxleConfig,

    #[serde(default)]
    pub tire_wear: TireWearAxleConfig,

    // Schema v3: per-axle steady tire force envelope (consumed by tire.rs).
    #[serde(default)]
    pub front_tire_force: TireForceProfile,
    #[serde(default)]
    pub rear_tire_force: TireForceProfile,
    #[serde(default)]
    pub pressure_mechanics: PressureMechanicsSensitivity,

    // Surface properties
    pub surface_friction: HashMap<SurfaceType, f64>,
    pub surface_stiffness: HashMap<SurfaceType, f64>,
    pub surface_rolling_resistance: HashMap<SurfaceType, f64>,
    pub surface_longitudinal_grip_ratio: HashMap<SurfaceType, f64>,
    /// GRIP-06: first-order filter tau (s) for the per-wheel effective surface
    /// friction/stiffness/rolling values, so kerb and grass crossings are
    /// gradual. `0.0` keeps instantaneous surface steps (legacy).
    #[serde(default)]
    pub surface_transition_tau_s: f64,

    // Driving aids (canonical model + policy). Runtime enablement is held
    // separately in the simulator's aids_enabled_mask, not here.
    pub aids: AidsConfig,

    // Aerodynamics
    pub aerodynamics: AerodynamicsConfiguration,

    /// Opaque passthrough of the profile `audio` section. Not consumed by the
    /// physics solver; parsed by the audio-contract consumer (e.g.
    /// vehicle-audio-engine `powertrain` module).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<serde_json::Value>,
    #[serde(default)]
    pub coupled_world: Option<crate::coupled_vehicle_world::CoupledVehicleWorldConfiguration>,
    #[serde(default)]
    pub underfloor_contact: Option<crate::underfloor_contact_configuration::UnderfloorContactConfig>,
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self::f1_94_canonical()
    }
}

impl VehicleConfig {
    /// Dry mass plus the fuel currently in the tank.
    pub fn total_vehicle_mass(&self) -> f64 {
        self.complete_dry_vehicle_mass() + self.fuel.effective_current_kg()
    }

    pub fn complete_dry_vehicle_mass(&self) -> f64 {
        self.chassis.vehicle_mass + self.additional_wheel_assembly_mass()
    }

    pub fn additional_wheel_assembly_mass(&self) -> f64 {
        if self.chassis.vehicle_mass_excludes_wheel_assemblies {
            2.0 * (self.front_wheel_mass + self.rear_wheel_mass)
        } else {
            0.0
        }
    }

    /// Front axle share of the total mass with the tank load present. The combined
    /// centre of mass blends the dry centre of mass with the tank position, so the
    /// distribution migrates toward the tank as the tank drains or fills.
    pub fn effective_front_weight_distribution(&self) -> f64 {
        let fuel_kg = self.fuel.effective_current_kg();
        if !self.chassis.vehicle_mass_excludes_wheel_assemblies && fuel_kg <= 0.0 {
            return self.chassis.front_weight_distribution;
        }
        let total_mass = self.total_vehicle_mass();
        if total_mass <= 1e-9 {
            return self.chassis.front_weight_distribution;
        }
        let fuel_front_distribution = if self.chassis.wheelbase.abs() > 1e-9 {
            0.5 - self.fuel.tank_position_local_m.z / self.chassis.wheelbase
        } else {
            self.chassis.front_weight_distribution
        };
        let additional_front_wheel_mass = if self.chassis.vehicle_mass_excludes_wheel_assemblies {
            2.0 * self.front_wheel_mass
        } else {
            0.0
        };
        ((self.chassis.vehicle_mass * self.chassis.front_weight_distribution
            + additional_front_wheel_mass
            + fuel_kg * fuel_front_distribution)
            / total_mass).clamp(0.0, 1.0)
    }

    /// Helper to get mass supported by a single wheel at static rest.
    pub fn mass_over_wheel(&self, wheel: WheelIndex) -> f64 {
        let total_mass = self.total_vehicle_mass();
        let front_distribution = self.effective_front_weight_distribution();
        if wheel.is_front() {
            total_mass * front_distribution * 0.5
        } else {
            total_mass * (1.0 - front_distribution) * 0.5
        }
    }

    /// Helper to get suspension spring rate in N/m for a wheel based on rest target.
    pub fn calculate_spring_rate(&self, wheel: WheelIndex) -> f64 {
        let mass = self.mass_over_wheel(wheel);
        let g = 9.80665;
        let spring_length = if wheel.is_front() {
            self.suspension.front_spring_length
        } else {
            self.suspension.rear_spring_length
        };
        let resting_ratio = if wheel.is_front() {
            self.suspension.front_resting_ratio
        } else {
            self.suspension.rear_resting_ratio
        };
        let rest_travel = spring_length * resting_ratio;
        if rest_travel > 1e-4 {
            (mass * g) / rest_travel
        } else {
            50000.0
        }
    }

    /// Nominal suspension-ray origin in vehicle-local geometric space (-Z forward, +Y up, +X right),
    /// where origin (0, 0, 0) is centered between axles at mean wheel-center height.
    pub fn wheel_anchor_local(&self, wheel: WheelIndex) -> Vec3 {
        let z = if wheel.is_front() {
            -self.chassis.wheelbase * 0.5
        } else {
            self.chassis.wheelbase * 0.5
        };
        let x = if wheel.is_front() {
            if wheel.is_right() {
                self.chassis.front_track * 0.5
            } else {
                -self.chassis.front_track * 0.5
            }
        } else if wheel.is_right() {
            self.chassis.rear_track * 0.5
        } else {
            -self.chassis.rear_track * 0.5
        };

        let mean_radius = (self.front_tire_radius + self.rear_tire_radius) * 0.5;
        let hub_y = if wheel.is_front() {
            self.front_tire_radius - mean_radius
        } else {
            self.rear_tire_radius - mean_radius
        };
        let spring_length = if wheel.is_front() {
            self.suspension.front_spring_length
        } else {
            self.suspension.rear_spring_length
        };
        let resting_ratio = if wheel.is_front() {
            self.suspension.front_resting_ratio
        } else {
            self.suspension.rear_resting_ratio
        };
        let anchor_y = hub_y + spring_length * (1.0 - resting_ratio);

        Vec3::new(x, anchor_y, z)
    }

    /// Load vehicle configuration from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let spec: JsonVehicleSpec =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
        Self::from_json_spec(spec)
    }

    /// Load vehicle configuration from a JSON file path.
    pub fn from_json_path(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        Self::from_json_str(&json)
    }

    /// Serialize this config to a serde_json::Value matching the JSON schema.
    pub fn to_json_value(&self) -> serde_json::Value {
        let spec = JsonVehicleSpec::from_config(self);
        serde_json::to_value(spec).unwrap_or_default()
    }

    fn from_json_spec(spec: JsonVehicleSpec) -> Result<Self, String> {
        spec.validate()?;
        let config = spec.to_config();
        crate::aero::validate_aero_config(&config)?;
        if let Some(inventory) = &config.chassis.physical_mass_inventory {
            inventory.validate(&config)?;
        }
        if let Some(world) = &config.coupled_world {
            world.validate()?;
            if config.chassis.physical_mass_inventory.is_none() { return Err("Coupled world requires a physical mass inventory".into()); }
        }
        Ok(config)
    }
}

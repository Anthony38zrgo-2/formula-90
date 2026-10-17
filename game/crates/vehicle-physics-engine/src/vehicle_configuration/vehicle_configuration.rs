use super::*;
use super::legacy_profile::JsonVehicleSpec;

/// Formula-90 vehicle configuration with GEVP-compatible suspension, tire and drivetrain semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfiguration {
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
    pub tires: TiresConfiguration,
    pub brakes: BrakesConfiguration,

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

pub type VehicleConfig = VehicleConfiguration;

impl Default for VehicleConfiguration {
    fn default() -> Self {
        Self::f1_94_canonical()
    }
}

impl VehicleConfiguration {
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

    pub fn to_resolved_configuration_value(&self) -> serde_json::Value {
        let mut configuration = self.to_json_value();
        configuration["powertrain"]["gear_inertia"] = serde_json::json!(self.powertrain.gear_inertia);
        configuration["powertrain"]["automatic_shift"] =
            serde_json::to_value(&self.powertrain.automatic_shift).expect("automatic shift parameters serialize");
        configuration
    }

    fn from_json_spec(spec: JsonVehicleSpec) -> Result<Self, String> {
        spec.validate()?;
        let mut config = spec.to_config();
        config
            .chassis
            .bind_authored_wheel_centers(&config.suspension);
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

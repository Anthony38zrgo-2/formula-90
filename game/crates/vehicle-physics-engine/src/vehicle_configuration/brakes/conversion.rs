use super::*;
use super::document::*;

impl JsonBrakeThermal {
    pub(crate) fn to_config(&self) -> BrakeThermalConfig {
        let mut c = BrakeThermalConfig::default();
        if let Some(v) = self.initial_temperature_c {
            c.initial_temperature_c = v;
        }
        if let Some(v) = self.optimal_min_temperature_c {
            c.optimal_min_temperature_c = v;
        }
        if let Some(v) = self.optimal_max_temperature_c {
            c.optimal_max_temperature_c = v;
        }
        if let Some(v) = self.fade_start_temperature_c {
            c.fade_start_temperature_c = v;
        }
        if let Some(v) = self.critical_temperature_c {
            c.critical_temperature_c = v;
        }
        if let Some(v) = self.cold_efficiency {
            c.cold_efficiency = v;
        }
        if let Some(v) = self.minimum_fade_efficiency {
            c.minimum_fade_efficiency = v;
        }
        if let Some(v) = self.braking_heat_fraction {
            c.braking_heat_fraction = v;
        }
        // direct_caliper_heat_fraction is v2 parse-only.
        if let Some(v) = &self.front {
            overlay_brake_axle(&mut c.front, v);
        }
        if let Some(v) = &self.rear {
            overlay_brake_axle(&mut c.rear, v);
        }
        if let Some(v) = &self.front_duct {
            overlay_brake_duct(&mut c.front_duct, v);
        }
        if let Some(v) = &self.rear_duct {
            overlay_brake_duct(&mut c.rear_duct, v);
        }
        c
    }

    pub(crate) fn from_config(c: &BrakeThermalConfig) -> Self {
        Self {
            initial_temperature_c: Some(c.initial_temperature_c),
            optimal_min_temperature_c: Some(c.optimal_min_temperature_c),
            optimal_max_temperature_c: Some(c.optimal_max_temperature_c),
            fade_start_temperature_c: Some(c.fade_start_temperature_c),
            critical_temperature_c: Some(c.critical_temperature_c),
            cold_efficiency: Some(c.cold_efficiency),
            minimum_fade_efficiency: Some(c.minimum_fade_efficiency),
            braking_heat_fraction: Some(c.braking_heat_fraction),
            // v2-only key; never emitted for the compact v3 payload.
            direct_caliper_heat_fraction: None,
            front: Some(JsonBrakeAxleThermal::from_config(&c.front)),
            rear: Some(JsonBrakeAxleThermal::from_config(&c.rear)),
            front_duct: Some(JsonBrakeDuctAxle::from_config(&c.front_duct)),
            rear_duct: Some(JsonBrakeDuctAxle::from_config(&c.rear_duct)),
        }
    }
}

impl JsonBrakeAxleThermal {
    pub(crate) fn from_config(c: &BrakeAxleThermalConfig) -> Self {
        Self {
            model: Some(c.model),
            rotor_material: Some(c.rotor_material),
            rotor_mass_kg: Some(c.rotor_mass_kg),
            rotor_outer_diameter_m: Some(c.rotor_outer_diameter_m),
            rotor_inner_diameter_m: Some(c.rotor_inner_diameter_m),
            rotor_radiation_emissivity: Some(c.rotor_radiation_emissivity),
            rotor_radiation_ambient_view_factor: Some(c.rotor_radiation_ambient_view_factor),
            rotor_ventilation: Some(c.rotor_ventilation),
            cooling_profile: Some(c.cooling_profile),
            installation_airflow_scale: Some(c.installation_airflow_scale),
            thermal_mass_scale: Some(c.thermal_mass_scale),
            rotor_to_rim_w_k: Some(c.rotor_to_rim_w_k),
            rim_heat_capacity_j_k: Some(c.rim_heat_capacity_j_k),
            rim_base_air_w_k: Some(c.rim_base_air_w_k),
            rim_to_tire_carcass_w_k: Some(c.rim_to_tire_carcass_w_k),
            rim_to_tire_gas_w_k: Some(c.rim_to_tire_gas_w_k),
            // v2-only keys; never emitted for the compact v3 payload.
            surface_bulk_response_scale: None,
            disc_heat_capacity_j_k: None,
            disc_surface_heat_capacity_j_k: None,
            disc_bulk_heat_capacity_j_k: None,
            disc_surface_to_bulk_w_k: None,
            disc_surface_base_air_w_k: None,
            caliper_heat_capacity_j_k: None,
            hub_heat_capacity_j_k: None,
            disc_to_caliper_w_k: None,
            disc_to_hub_w_k: None,
            disc_to_rim_radiation_w_k: None,
            hub_to_rim_w_k: None,
            disc_base_air_w_k: None,
            caliper_base_air_w_k: None,
            hub_base_air_w_k: None,
            disc_flow_cooling_gain_w_k: None,
            caliper_flow_cooling_gain_w_k: None,
            hub_flow_cooling_gain_w_k: None,
            rim_flow_cooling_gain_w_k: None,
        }
    }
}

impl JsonBrakeDuctAxle {
    pub(crate) fn from_config(c: &BrakeDuctAxleConfig) -> Self {
        Self {
            opening: Some(c.opening),
            max_inlet_area_m2_per_wheel: Some(c.max_inlet_area_m2_per_wheel),
            discharge_coefficient: Some(c.discharge_coefficient),
            pressure_recovery: Some(c.pressure_recovery),
            cooling_reference_speed_ms: Some(c.cooling_reference_speed_ms),
            drag_coefficient: Some(c.drag_coefficient),
            area_response_exponent: Some(c.area_response_exponent),
            cooling_flow_exponent: Some(c.cooling_flow_exponent),
            minimum_cooling_flow_ratio: Some(c.minimum_cooling_flow_ratio),
            air_specific_heat_j_kg_k: Some(c.air_specific_heat_j_kg_k),
            heat_exchanger_ua_w_k: Some(c.heat_exchanger_ua_w_k),
            rotor_cooling_fraction: Some(c.rotor_cooling_fraction),
            // v2-only keys; never emitted for the compact v3 payload.
            disc_cooling_weight: None,
            caliper_cooling_weight: None,
            hub_cooling_weight: None,
            rim_cooling_weight: None,
        }
    }
}

fn overlay_brake_axle(c: &mut BrakeAxleThermalConfig, j: &JsonBrakeAxleThermal) {
    if let Some(v) = j.model {
        c.model = v;
    }
    if let Some(v) = j.rotor_material {
        c.rotor_material = v;
    }
    if let Some(v) = j.rotor_mass_kg {
        c.rotor_mass_kg = v;
    }
    if let Some(v) = j.rotor_outer_diameter_m {
        c.rotor_outer_diameter_m = v;
    }
    if let Some(v) = j.rotor_inner_diameter_m {
        c.rotor_inner_diameter_m = v;
    }
    if let Some(emissivity) = j.rotor_radiation_emissivity {
        c.rotor_radiation_emissivity = emissivity;
    }
    if let Some(ambient_view_factor) = j.rotor_radiation_ambient_view_factor {
        c.rotor_radiation_ambient_view_factor = ambient_view_factor;
    }
    if let Some(v) = j.rotor_ventilation {
        c.rotor_ventilation = v;
    }
    if let Some(v) = j.cooling_profile {
        c.cooling_profile = v;
    }
    if let Some(v) = j.installation_airflow_scale {
        c.installation_airflow_scale = v;
    }
    if let Some(v) = j.thermal_mass_scale {
        c.thermal_mass_scale = v;
    }
    if let Some(v) = j.rotor_to_rim_w_k {
        c.rotor_to_rim_w_k = v;
    } else if j.disc_to_hub_w_k.is_some()
        || j.hub_to_rim_w_k.is_some()
        || j.disc_to_rim_radiation_w_k.is_some()
    {
        // BRAKE-1000: physical lumping of the v2 rotor->hub->rim conduction
        // path plus radiation into one rotor->rim conductance. Missing v2
        // keys use the legacy defaults; a zero key yields a zero series term.
        let disc_to_hub = j.disc_to_hub_w_k.unwrap_or(LEGACY_DISC_TO_HUB_DEFAULT_W_K);
        let hub_to_rim = j.hub_to_rim_w_k.unwrap_or(LEGACY_HUB_TO_RIM_DEFAULT_W_K);
        let radiation = j
            .disc_to_rim_radiation_w_k
            .unwrap_or(LEGACY_DISC_TO_RIM_RADIATION_DEFAULT_W_K);
        let series = if disc_to_hub > 0.0 && hub_to_rim > 0.0 {
            1.0 / (1.0 / disc_to_hub + 1.0 / hub_to_rim)
        } else {
            0.0
        };
        c.rotor_to_rim_w_k = series + radiation;
    }
    if let Some(v) = j.rim_heat_capacity_j_k {
        c.rim_heat_capacity_j_k = v;
    }
    if let Some(v) = j.rim_to_tire_carcass_w_k {
        c.rim_to_tire_carcass_w_k = v;
    }
    if let Some(v) = j.rim_to_tire_gas_w_k {
        c.rim_to_tire_gas_w_k = v;
    }
    if let Some(v) = j.rim_base_air_w_k {
        c.rim_base_air_w_k = v;
    }
    // Deprecated v2 keys (surface/bulk split, caliper/hub nodes, explicit
    // capacities, direct caliper heating, flow-cooling gains) are parse-only:
    // accepted for schema-v2 compatibility and ignored. The compact rotor
    // capacity derives from rotor_material/rotor_mass_kg/thermal_mass_scale.
}

fn overlay_brake_duct(c: &mut BrakeDuctAxleConfig, j: &JsonBrakeDuctAxle) {
    if let Some(v) = j.opening {
        c.opening = v;
    }
    if let Some(v) = j.max_inlet_area_m2_per_wheel {
        c.max_inlet_area_m2_per_wheel = v;
    }
    if let Some(v) = j.discharge_coefficient {
        c.discharge_coefficient = v;
    }
    if let Some(v) = j.pressure_recovery {
        c.pressure_recovery = v;
    }
    if let Some(v) = j.cooling_reference_speed_ms {
        c.cooling_reference_speed_ms = v;
    }
    if let Some(v) = j.drag_coefficient {
        c.drag_coefficient = v;
    }
    if let Some(v) = j.area_response_exponent {
        c.area_response_exponent = v;
    }
    if let Some(v) = j.cooling_flow_exponent {
        c.cooling_flow_exponent = v;
    }
    if let Some(v) = j.minimum_cooling_flow_ratio {
        c.minimum_cooling_flow_ratio = v;
    }
    if let Some(v) = j.air_specific_heat_j_kg_k {
        c.air_specific_heat_j_kg_k = v;
    }
    if let Some(v) = j.heat_exchanger_ua_w_k {
        c.heat_exchanger_ua_w_k = v;
    }
    if let Some(v) = j.rotor_cooling_fraction {
        c.rotor_cooling_fraction = v;
    } else if j.disc_cooling_weight.is_some()
        || j.caliper_cooling_weight.is_some()
        || j.hub_cooling_weight.is_some()
        || j.rim_cooling_weight.is_some()
    {
        // BRAKE-1000: physical lumping of the v2 per-node cooling weights into
        // a single rotor share. Missing weights use the legacy defaults so the
        // derivation is stable for partial v2 blocks.
        let disc = j.disc_cooling_weight.unwrap_or(LEGACY_DISC_COOLING_WEIGHT_DEFAULT);
        let caliper = j
            .caliper_cooling_weight
            .unwrap_or(LEGACY_CALIPER_COOLING_WEIGHT_DEFAULT);
        let hub = j.hub_cooling_weight.unwrap_or(LEGACY_HUB_COOLING_WEIGHT_DEFAULT);
        let rim = j.rim_cooling_weight.unwrap_or(LEGACY_RIM_COOLING_WEIGHT_DEFAULT);
        let total = disc + caliper + hub + rim;
        if total.is_finite() && total > 0.0 {
            c.rotor_cooling_fraction = (disc + caliper + hub) / total;
        }
    }
}

// Legacy v2 defaults used when only some of the deprecated lumping inputs are
// present in a schema-v2 brake block.
const LEGACY_DISC_TO_HUB_DEFAULT_W_K: f64 = 38.0;
const LEGACY_HUB_TO_RIM_DEFAULT_W_K: f64 = 31.0;
const LEGACY_DISC_TO_RIM_RADIATION_DEFAULT_W_K: f64 = 9.0;
const LEGACY_DISC_COOLING_WEIGHT_DEFAULT: f64 = 0.556;
const LEGACY_CALIPER_COOLING_WEIGHT_DEFAULT: f64 = 0.222;
const LEGACY_HUB_COOLING_WEIGHT_DEFAULT: f64 = 0.095;
const LEGACY_RIM_COOLING_WEIGHT_DEFAULT: f64 = 0.127;

use super::*;
use super::document::*;

impl JsonTireThermal {
    pub(crate) fn from_config(c: &TireThermalAxleConfig) -> Self {
        let mut out = Self::from_single_config(&c.front);
        out.front = Some(Box::new(Self::from_single_config(&c.front)));
        out.rear = Some(Box::new(Self::from_single_config(&c.rear)));
        out
    }

    pub(crate) fn from_single_config(c: &TireThermalConfig) -> Self {
        Self {
            initial_temperature_c: Some(c.initial_temperature_c),
            ambient_fallback_c: Some(c.ambient_fallback_c),
            track_fallback_c: Some(c.track_fallback_c),
            optimal_tread_temperature_c: Some(c.optimal_tread_temperature_c),
            optimal_tread_window_c: Some(c.optimal_tread_window_c),
            overheat_tread_temperature_c: Some(c.overheat_tread_temperature_c),
            optimal_carcass_temperature_c: Some(c.optimal_carcass_temperature_c),
            tread_zone_heat_capacity_j_k: Some(c.tread_zone_heat_capacity_j_k),
            carcass_heat_capacity_j_k: Some(c.carcass_heat_capacity_j_k),
            gas_heat_capacity_j_k: Some(c.gas_heat_capacity_j_k),
            tread_to_carcass_w_k: Some(c.tread_to_carcass_w_k),
            carcass_to_gas_w_k: Some(c.carcass_to_gas_w_k),
            tread_to_air_w_k: Some(c.tread_to_air_w_k),
            carcass_to_air_w_k: Some(c.carcass_to_air_w_k),
            road_conductance_w_k: Some(c.road_conductance_w_k),
            lateral_tread_conductance_w_k: Some(c.lateral_tread_conductance_w_k),
            slip_heat_efficiency: Some(c.slip_heat_efficiency),
            carcass_hysteresis_efficiency: Some(c.carcass_hysteresis_efficiency),
            // THERM-700: no pressure exponents are emitted; they are replaced by the
            // shared tires.pressure_mechanics sensitivity block.
            pressure_stiffness_exponent: None,
            pressure_damping_exponent: None,
            pressure_max_deflection_exponent: None,
            pressure_patch_exponent: None,
            pressure_relaxation_exponent: None,
            pressure_rr_exponent: None,
            pressure_force_stiffness_exponent: None,
            pressure_trail_exponent: None,
            volume_deflection_gain: Some(c.volume_deflection_gain),
            minimum_pressure_kpa_gauge: Some(c.minimum_pressure_kpa_gauge),
            maximum_pressure_kpa_gauge: Some(c.maximum_pressure_kpa_gauge),
            front: None,
            rear: None,
        }
    }
}


impl JsonTireWear {
    pub(crate) fn from_config(c: &TireWearAxleConfig) -> Self {
        let mut out = Self::from_single_config(&c.front);
        out.front = Some(Box::new(Self::from_single_config(&c.front)));
        out.rear = Some(Box::new(Self::from_single_config(&c.rear)));
        out
    }

    pub(crate) fn from_single_config(c: &TireWearConfig) -> Self {
        Self {
            initial_tread_depth_millimeters: Some(c.initial_tread_depth_millimeters),
            minimum_tread_depth_millimeters: Some(c.minimum_tread_depth_millimeters),
            tread_wear_per_megajoule_millimeters: Some(c.tread_wear_per_megajoule_millimeters),
            cold_tread_temperature_celsius: Some(c.cold_tread_temperature_celsius),
            optimal_tread_temperature_celsius: Some(c.optimal_tread_temperature_celsius),
            overheat_tread_temperature_celsius: Some(c.overheat_tread_temperature_celsius),
            extreme_tread_temperature_celsius: Some(c.extreme_tread_temperature_celsius),
            cold_temperature_wear_multiplier: Some(c.cold_temperature_wear_multiplier),
            overheat_temperature_wear_multiplier: Some(c.overheat_temperature_wear_multiplier),
            extreme_temperature_wear_multiplier: Some(c.extreme_temperature_wear_multiplier),
            friction_reference_coefficient: Some(c.friction_reference_coefficient),
            friction_sensitivity_exponent: Some(c.friction_sensitivity_exponent),
            inner_shoulder_wear_multiplier: Some(c.inner_shoulder_wear_multiplier),
            center_wear_multiplier: Some(c.center_wear_multiplier),
            outer_shoulder_wear_multiplier: Some(c.outer_shoulder_wear_multiplier),
            cliff_start_wear_fraction: Some(c.cliff_start_wear_fraction),
            end_of_life_grip_scale: Some(c.end_of_life_grip_scale),
            progressive_grip_loss_fraction: Some(c.progressive_grip_loss_fraction),
            front: None,
            rear: None,
        }
    }
}


pub(crate) fn tire_wear_from_json(j: &Option<JsonTireWear>) -> TireWearAxleConfig {
    let mut shared = TireWearConfig::default();
    if let Some(w) = j {
        overlay_tire_wear(&mut shared, w);
        let mut front = shared;
        let mut rear = shared;
        if let Some(front_json) = &w.front {
            overlay_tire_wear(&mut front, front_json);
        }
        if let Some(rear_json) = &w.rear {
            overlay_tire_wear(&mut rear, rear_json);
        }
        return TireWearAxleConfig { front, rear };
    }
    TireWearAxleConfig {
        front: shared,
        rear: shared,
    }
}

pub(crate) fn overlay_tire_wear(c: &mut TireWearConfig, w: &JsonTireWear) {
    if let Some(v) = w.initial_tread_depth_millimeters {
        c.initial_tread_depth_millimeters = v;
    }
    if let Some(v) = w.minimum_tread_depth_millimeters {
        c.minimum_tread_depth_millimeters = v;
    }
    if let Some(v) = w.tread_wear_per_megajoule_millimeters {
        c.tread_wear_per_megajoule_millimeters = v;
    }
    if let Some(v) = w.cold_tread_temperature_celsius {
        c.cold_tread_temperature_celsius = v;
    }
    if let Some(v) = w.optimal_tread_temperature_celsius {
        c.optimal_tread_temperature_celsius = v;
    }
    if let Some(v) = w.overheat_tread_temperature_celsius {
        c.overheat_tread_temperature_celsius = v;
    }
    if let Some(v) = w.extreme_tread_temperature_celsius {
        c.extreme_tread_temperature_celsius = v;
    }
    if let Some(v) = w.cold_temperature_wear_multiplier {
        c.cold_temperature_wear_multiplier = v;
    }
    if let Some(v) = w.overheat_temperature_wear_multiplier {
        c.overheat_temperature_wear_multiplier = v;
    }
    if let Some(v) = w.extreme_temperature_wear_multiplier {
        c.extreme_temperature_wear_multiplier = v;
    }
    if let Some(v) = w.friction_reference_coefficient {
        c.friction_reference_coefficient = v;
    }
    if let Some(v) = w.friction_sensitivity_exponent {
        c.friction_sensitivity_exponent = v;
    }
    if let Some(v) = w.inner_shoulder_wear_multiplier {
        c.inner_shoulder_wear_multiplier = v;
    }
    if let Some(v) = w.center_wear_multiplier {
        c.center_wear_multiplier = v;
    }
    if let Some(v) = w.outer_shoulder_wear_multiplier {
        c.outer_shoulder_wear_multiplier = v;
    }
    if let Some(v) = w.cliff_start_wear_fraction {
        c.cliff_start_wear_fraction = v;
    }
    if let Some(v) = w.end_of_life_grip_scale {
        c.end_of_life_grip_scale = v;
    }
    if let Some(v) = w.progressive_grip_loss_fraction {
        c.progressive_grip_loss_fraction = v;
    }
}

pub(crate) fn tire_pressure_from_json(j: &Option<JsonTirePressure>) -> TirePressureConfig {
    let mut cfg = TirePressureConfig::default();
    if let Some(p) = j {
        if let Some(map) = &p.cold_kpa_gauge {
            for (name, v) in map {
                if let Some(w) = wheel_index_from_name(name) {
                    cfg.cold_kpa_gauge[w as usize] = *v;
                }
            }
        }
        if let Some(map) = &p.reference_hot_kpa_gauge {
            for (name, v) in map {
                if let Some(w) = wheel_index_from_name(name) {
                    cfg.reference_hot_kpa_gauge[w as usize] = *v;
                }
            }
        }
        if let Some(v) = p.reference_temperature_c {
            cfg.reference_temperature_c = v;
        }
        if let Some(v) = p.atmospheric_pressure_kpa {
            cfg.atmospheric_pressure_kpa = v;
        }
    }
    cfg
}

pub(crate) fn tire_thermal_from_json(j: &Option<JsonTireThermal>) -> TireThermalAxleConfig {
    let mut shared = TireThermalConfig::default();
    if let Some(t) = j {
        overlay_tire_thermal(&mut shared, t);
        let mut front = shared;
        let mut rear = shared;
        if let Some(front_json) = &t.front {
            overlay_tire_thermal(&mut front, front_json);
        }
        if let Some(rear_json) = &t.rear {
            overlay_tire_thermal(&mut rear, rear_json);
        }
        return TireThermalAxleConfig { front, rear };
    }
    TireThermalAxleConfig {
        front: shared,
        rear: shared,
    }
}

pub(crate) fn overlay_tire_thermal(c: &mut TireThermalConfig, t: &JsonTireThermal) {
    if let Some(v) = t.initial_temperature_c {
        c.initial_temperature_c = v;
    }
    if let Some(v) = t.ambient_fallback_c {
        c.ambient_fallback_c = v;
    }
    if let Some(v) = t.track_fallback_c {
        c.track_fallback_c = v;
    }
    if let Some(v) = t.optimal_tread_temperature_c {
        c.optimal_tread_temperature_c = v;
    }
    if let Some(v) = t.optimal_tread_window_c {
        c.optimal_tread_window_c = v;
    }
    if let Some(v) = t.overheat_tread_temperature_c {
        c.overheat_tread_temperature_c = v;
    }
    if let Some(v) = t.optimal_carcass_temperature_c {
        c.optimal_carcass_temperature_c = v;
    }
    if let Some(v) = t.tread_zone_heat_capacity_j_k {
        c.tread_zone_heat_capacity_j_k = v;
    }
    if let Some(v) = t.carcass_heat_capacity_j_k {
        c.carcass_heat_capacity_j_k = v;
    }
    if let Some(v) = t.gas_heat_capacity_j_k {
        c.gas_heat_capacity_j_k = v;
    }
    if let Some(v) = t.tread_to_carcass_w_k {
        c.tread_to_carcass_w_k = v;
    }
    if let Some(v) = t.carcass_to_gas_w_k {
        c.carcass_to_gas_w_k = v;
    }
    if let Some(v) = t.tread_to_air_w_k {
        c.tread_to_air_w_k = v;
    }
    if let Some(v) = t.carcass_to_air_w_k {
        c.carcass_to_air_w_k = v;
    }
    if let Some(v) = t.road_conductance_w_k {
        c.road_conductance_w_k = v;
    }
    if let Some(v) = t.lateral_tread_conductance_w_k {
        c.lateral_tread_conductance_w_k = v;
    }
    if let Some(v) = t.slip_heat_efficiency {
        c.slip_heat_efficiency = v;
    }
    if let Some(v) = t.carcass_hysteresis_efficiency {
        c.carcass_hysteresis_efficiency = v;
    }
    // THERM-700: the eight legacy pressure_*_exponent keys are parsed above purely so
    // schema-v2 profiles load; they are intentionally NOT applied to the runtime
    // config. tires.pressure_mechanics sensitivities are the only pressure knobs.
    if let Some(v) = t.volume_deflection_gain {
        c.volume_deflection_gain = v;
    }
    if let Some(v) = t.minimum_pressure_kpa_gauge {
        c.minimum_pressure_kpa_gauge = v;
    }
    if let Some(v) = t.maximum_pressure_kpa_gauge {
        c.maximum_pressure_kpa_gauge = v;
    }
}

pub(crate) fn wheel_index_from_name(name: &str) -> Option<WheelIndex> {
    match name {
        "FL" => Some(WheelIndex::FrontLeft),
        "FR" => Some(WheelIndex::FrontRight),
        "RL" => Some(WheelIndex::RearLeft),
        "RR" => Some(WheelIndex::RearRight),
        _ => None,
    }
}

pub(crate) fn wheel_name(w: WheelIndex) -> &'static str {
    match w {
        WheelIndex::FrontLeft => "FL",
        WheelIndex::FrontRight => "FR",
        WheelIndex::RearLeft => "RL",
        WheelIndex::RearRight => "RR",
    }
}

pub(crate) fn wheel_name_map(values: [f64; 4]) -> HashMap<String, f64> {
    let mut m = HashMap::new();
    for w in WheelIndex::ALL {
        m.insert(wheel_name(w).to_string(), values[w as usize]);
    }
    m
}


pub(crate) fn build_surface_assist_map<F>(
    surfaces: &HashMap<String, JsonSurfaceEntry>,
    f: F,
) -> HashMap<SurfaceType, f64>
where
    F: Fn(&JsonSurfaceEntry) -> f64,
{
    let mut map = HashMap::new();
    for (name, entry) in surfaces {
        if let Ok(st) = parse_surface_type(name) {
            map.insert(st, f(entry));
        }
    }
    map
}

pub(crate) fn parse_surface_type(name: &str) -> Result<SurfaceType, String> {
    match name {
        "Road" => Ok(SurfaceType::Road),
        "Curb" => Ok(SurfaceType::Curb),
        "Dirt" => Ok(SurfaceType::Dirt),
        "Grass" => Ok(SurfaceType::Grass),
        "Gravel" => Ok(SurfaceType::Gravel),
        "Sand" => Ok(SurfaceType::Sand),
        "Wall" => Ok(SurfaceType::Wall),
        "Metal" => Ok(SurfaceType::Metal),
        _ => Err(format!("Unknown surface type: {name}")),
    }
}

pub(crate) fn surface_type_name(st: SurfaceType) -> String {
    match st {
        SurfaceType::Road => "Road".to_string(),
        SurfaceType::Curb => "Curb".to_string(),
        SurfaceType::Dirt => "Dirt".to_string(),
        SurfaceType::Grass => "Grass".to_string(),
        SurfaceType::Gravel => "Gravel".to_string(),
        SurfaceType::Sand => "Sand".to_string(),
        SurfaceType::Wall => "Wall".to_string(),
        SurfaceType::Metal => "Metal".to_string(),
    }
}

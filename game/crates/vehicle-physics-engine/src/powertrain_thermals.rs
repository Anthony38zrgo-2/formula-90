use serde::{Deserialize, Serialize};

const MINIMUM_THERMAL_STEP_SECONDS: f64 = 0.001;
const MAXIMUM_THERMAL_STEP_SECONDS: f64 = 0.05;
const COLD_TEMPERATURE_PENALTY_SPAN_CELSIUS: f64 = 45.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PowertrainCoolingDuctConfig {
    pub opening: f64,
    pub maximum_inlet_area_square_meters: f64,
    pub discharge_coefficient: f64,
    pub pressure_recovery: f64,
    pub drag_coefficient: f64,
    pub area_response_exponent: f64,
    pub air_specific_heat_joules_per_kilogram_kelvin: f64,
    pub heat_exchanger_conductance_watts_per_kelvin: f64,
    pub base_cooling_conductance_watts_per_kelvin: f64,
}

impl Default for PowertrainCoolingDuctConfig {
    fn default() -> Self {
        Self {
            opening: 0.0,
            maximum_inlet_area_square_meters: 0.03,
            discharge_coefficient: 0.72,
            pressure_recovery: 0.65,
            drag_coefficient: 0.70,
            area_response_exponent: 1.15,
            air_specific_heat_joules_per_kilogram_kelvin: 1005.0,
            heat_exchanger_conductance_watts_per_kelvin: 2500.0,
            base_cooling_conductance_watts_per_kelvin: 25.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PowertrainThermalConfig {
    pub initial_engine_block_temperature_celsius: f64,
    pub initial_water_temperature_celsius: f64,
    pub initial_oil_temperature_celsius: f64,
    pub engine_block_heat_capacity_joules_per_kelvin: f64,
    pub water_heat_capacity_joules_per_kelvin: f64,
    pub oil_heat_capacity_joules_per_kelvin: f64,
    pub engine_to_water_conductance_watts_per_kelvin: f64,
    pub engine_to_oil_conductance_watts_per_kelvin: f64,
    pub idle_heat_generation_watts: f64,
    pub heat_generation_per_mechanical_power: f64,
    pub exhaust_heat_fraction: f64,
    pub engine_block_heat_fraction: f64,
    pub water_heat_fraction: f64,
    pub oil_heat_fraction: f64,
    pub water_optimal_minimum_temperature_celsius: f64,
    pub water_optimal_maximum_temperature_celsius: f64,
    pub water_hot_derating_temperature_celsius: f64,
    pub water_critical_temperature_celsius: f64,
    pub water_minimum_cold_engine_torque_fraction: f64,
    pub oil_optimal_minimum_temperature_celsius: f64,
    pub oil_optimal_maximum_temperature_celsius: f64,
    pub oil_hot_derating_temperature_celsius: f64,
    pub oil_critical_temperature_celsius: f64,
    pub oil_minimum_cold_engine_torque_fraction: f64,
    pub minimum_hot_engine_torque_fraction: f64,
    pub water_cooling_duct: PowertrainCoolingDuctConfig,
    pub oil_cooling_duct: PowertrainCoolingDuctConfig,
}

impl Default for PowertrainThermalConfig {
    fn default() -> Self {
        Self {
            initial_engine_block_temperature_celsius: 100.0,
            initial_water_temperature_celsius: 85.0,
            initial_oil_temperature_celsius: 100.0,
            engine_block_heat_capacity_joules_per_kelvin: 48000.0,
            water_heat_capacity_joules_per_kelvin: 22000.0,
            oil_heat_capacity_joules_per_kelvin: 13000.0,
            engine_to_water_conductance_watts_per_kelvin: 900.0,
            engine_to_oil_conductance_watts_per_kelvin: 420.0,
            idle_heat_generation_watts: 18000.0,
            heat_generation_per_mechanical_power: 1.4,
            exhaust_heat_fraction: 0.0,
            engine_block_heat_fraction: 1.0,
            water_heat_fraction: 0.0,
            oil_heat_fraction: 0.0,
            water_optimal_minimum_temperature_celsius: 80.0,
            water_optimal_maximum_temperature_celsius: 105.0,
            water_hot_derating_temperature_celsius: 110.0,
            water_critical_temperature_celsius: 120.0,
            water_minimum_cold_engine_torque_fraction: 0.96,
            oil_optimal_minimum_temperature_celsius: 95.0,
            oil_optimal_maximum_temperature_celsius: 125.0,
            oil_hot_derating_temperature_celsius: 130.0,
            oil_critical_temperature_celsius: 145.0,
            oil_minimum_cold_engine_torque_fraction: 0.88,
            minimum_hot_engine_torque_fraction: 0.70,
            water_cooling_duct: PowertrainCoolingDuctConfig {
                opening: 0.45,
                maximum_inlet_area_square_meters: 0.030,
                discharge_coefficient: 0.72,
                pressure_recovery: 0.65,
                drag_coefficient: 0.70,
                area_response_exponent: 1.15,
                air_specific_heat_joules_per_kilogram_kelvin: 1005.0,
                heat_exchanger_conductance_watts_per_kelvin: 2500.0,
                base_cooling_conductance_watts_per_kelvin: 25.0,
            },
            oil_cooling_duct: PowertrainCoolingDuctConfig {
                opening: 0.35,
                maximum_inlet_area_square_meters: 0.015,
                discharge_coefficient: 0.72,
                pressure_recovery: 0.60,
                drag_coefficient: 0.65,
                area_response_exponent: 1.15,
                air_specific_heat_joules_per_kilogram_kelvin: 1005.0,
                heat_exchanger_conductance_watts_per_kelvin: 1100.0,
                base_cooling_conductance_watts_per_kelvin: 12.0,
            },
        }
    }
}

impl PowertrainThermalConfig {
    pub fn validate(&self) -> Result<(), String> {
        let finite_values = [
            self.initial_engine_block_temperature_celsius,
            self.initial_water_temperature_celsius,
            self.initial_oil_temperature_celsius,
            self.engine_block_heat_capacity_joules_per_kelvin,
            self.water_heat_capacity_joules_per_kelvin,
            self.oil_heat_capacity_joules_per_kelvin,
            self.engine_to_water_conductance_watts_per_kelvin,
            self.engine_to_oil_conductance_watts_per_kelvin,
            self.idle_heat_generation_watts,
            self.heat_generation_per_mechanical_power,
            self.exhaust_heat_fraction,
            self.engine_block_heat_fraction,
            self.water_heat_fraction,
            self.oil_heat_fraction,
            self.water_optimal_minimum_temperature_celsius,
            self.water_optimal_maximum_temperature_celsius,
            self.water_hot_derating_temperature_celsius,
            self.water_critical_temperature_celsius,
            self.water_minimum_cold_engine_torque_fraction,
            self.oil_optimal_minimum_temperature_celsius,
            self.oil_optimal_maximum_temperature_celsius,
            self.oil_hot_derating_temperature_celsius,
            self.oil_critical_temperature_celsius,
            self.oil_minimum_cold_engine_torque_fraction,
            self.minimum_hot_engine_torque_fraction,
        ];
        if finite_values.iter().any(|value| !value.is_finite()) {
            return Err("powertrain.thermal values must be finite".to_string());
        }
        if self.engine_block_heat_capacity_joules_per_kelvin <= 0.0
            || self.water_heat_capacity_joules_per_kelvin <= 0.0
            || self.oil_heat_capacity_joules_per_kelvin <= 0.0
        {
            return Err("powertrain.thermal heat capacities must be positive".to_string());
        }
        if self.engine_to_water_conductance_watts_per_kelvin < 0.0
            || self.engine_to_oil_conductance_watts_per_kelvin < 0.0
            || self.idle_heat_generation_watts < 0.0
            || self.heat_generation_per_mechanical_power < 0.0
        {
            return Err(
                "powertrain.thermal conductance and heat generation must be non-negative"
                    .to_string(),
            );
        }
        let heat_fractions = [
            self.exhaust_heat_fraction,
            self.engine_block_heat_fraction,
            self.water_heat_fraction,
            self.oil_heat_fraction,
        ];
        if heat_fractions
            .iter()
            .any(|fraction| !(0.0..=1.0).contains(fraction))
            || (heat_fractions.iter().sum::<f64>() - 1.0).abs() > 1.0e-6
        {
            return Err(
                "powertrain.thermal heat fractions must be in [0,1] and sum to one".to_string(),
            );
        }
        if self.water_optimal_minimum_temperature_celsius
            >= self.water_optimal_maximum_temperature_celsius
            || self.water_optimal_maximum_temperature_celsius
                > self.water_hot_derating_temperature_celsius
            || self.water_hot_derating_temperature_celsius
                >= self.water_critical_temperature_celsius
        {
            return Err("powertrain.thermal water thresholds are not ordered".to_string());
        }
        if self.oil_optimal_minimum_temperature_celsius
            >= self.oil_optimal_maximum_temperature_celsius
            || self.oil_optimal_maximum_temperature_celsius
                > self.oil_hot_derating_temperature_celsius
            || self.oil_hot_derating_temperature_celsius >= self.oil_critical_temperature_celsius
        {
            return Err("powertrain.thermal oil thresholds are not ordered".to_string());
        }
        if !(0.0..=1.0).contains(&self.minimum_hot_engine_torque_fraction)
            || !(0.0..=1.0).contains(&self.water_minimum_cold_engine_torque_fraction)
            || !(0.0..=1.0).contains(&self.oil_minimum_cold_engine_torque_fraction)
        {
            return Err("powertrain.thermal torque fractions must be in [0,1]".to_string());
        }
        self.water_cooling_duct.validate("water_cooling_duct")?;
        self.oil_cooling_duct.validate("oil_cooling_duct")?;
        Ok(())
    }
}

impl PowertrainCoolingDuctConfig {
    fn validate(&self, duct_name: &str) -> Result<(), String> {
        let finite_values = [
            self.opening,
            self.maximum_inlet_area_square_meters,
            self.discharge_coefficient,
            self.pressure_recovery,
            self.drag_coefficient,
            self.area_response_exponent,
            self.air_specific_heat_joules_per_kilogram_kelvin,
            self.heat_exchanger_conductance_watts_per_kelvin,
            self.base_cooling_conductance_watts_per_kelvin,
        ];
        if finite_values.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "powertrain.thermal.{duct_name} values must be finite"
            ));
        }
        if !(0.0..=1.0).contains(&self.opening) {
            return Err(format!(
                "powertrain.thermal.{duct_name}.opening must be in [0,1]"
            ));
        }
        if self.maximum_inlet_area_square_meters < 0.0
            || self.discharge_coefficient < 0.0
            || self.pressure_recovery < 0.0
            || self.drag_coefficient < 0.0
            || self.heat_exchanger_conductance_watts_per_kelvin < 0.0
            || self.base_cooling_conductance_watts_per_kelvin < 0.0
        {
            return Err(format!(
                "powertrain.thermal.{duct_name} areas and coefficients must be non-negative"
            ));
        }
        if self.area_response_exponent <= 0.0
            || self.air_specific_heat_joules_per_kilogram_kelvin <= 0.0
        {
            return Err(format!(
                "powertrain.thermal.{duct_name} exponent and air heat capacity must be positive"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PowertrainCoolingDuctFlowState {
    pub effective_inlet_area_square_meters: f64,
    pub mass_flow_kilograms_per_second: f64,
    pub cooling_conductance_watts_per_kelvin: f64,
    pub drag_force_newtons: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowertrainThermalSystem {
    pub engine_block_temperature_celsius: f64,
    pub water_temperature_celsius: f64,
    pub oil_temperature_celsius: f64,
    pub water_cooling_duct_flow: PowertrainCoolingDuctFlowState,
    pub oil_cooling_duct_flow: PowertrainCoolingDuctFlowState,
    pub available_engine_torque_fraction: f64,
    pub total_powertrain_cooling_drag_force_newtons: f64,
    pub generated_engine_heat_watts: f64,
    pub generated_exhaust_heat_watts: f64,
    pub generated_engine_block_heat_watts: f64,
    pub generated_water_heat_watts: f64,
    pub generated_oil_heat_watts: f64,
    pub engine_to_water_heat_transfer_watts: f64,
    pub engine_to_oil_heat_transfer_watts: f64,
    pub water_rejected_heat_watts: f64,
    pub oil_rejected_heat_watts: f64,
}

impl PowertrainThermalSystem {
    pub fn new(config: &PowertrainThermalConfig) -> Self {
        let mut system = Self {
            engine_block_temperature_celsius: finite_or(
                config.initial_engine_block_temperature_celsius,
                100.0,
            ),
            water_temperature_celsius: finite_or(config.initial_water_temperature_celsius, 85.0),
            oil_temperature_celsius: finite_or(config.initial_oil_temperature_celsius, 100.0),
            water_cooling_duct_flow: PowertrainCoolingDuctFlowState::default(),
            oil_cooling_duct_flow: PowertrainCoolingDuctFlowState::default(),
            available_engine_torque_fraction: 1.0,
            total_powertrain_cooling_drag_force_newtons: 0.0,
            generated_engine_heat_watts: 0.0,
            generated_exhaust_heat_watts: 0.0,
            generated_engine_block_heat_watts: 0.0,
            generated_water_heat_watts: 0.0,
            generated_oil_heat_watts: 0.0,
            engine_to_water_heat_transfer_watts: 0.0,
            engine_to_oil_heat_transfer_watts: 0.0,
            water_rejected_heat_watts: 0.0,
            oil_rejected_heat_watts: 0.0,
        };
        system.update_available_engine_torque_fraction(config);
        system
    }

    pub fn update_available_engine_torque_fraction(&mut self, config: &PowertrainThermalConfig) {
        let water_fraction = temperature_torque_fraction(
            self.water_temperature_celsius,
            config.water_optimal_minimum_temperature_celsius,
            config.water_hot_derating_temperature_celsius,
            config.water_critical_temperature_celsius,
            config.water_minimum_cold_engine_torque_fraction,
            config.minimum_hot_engine_torque_fraction,
        );
        let oil_fraction = temperature_torque_fraction(
            self.oil_temperature_celsius,
            config.oil_optimal_minimum_temperature_celsius,
            config.oil_hot_derating_temperature_celsius,
            config.oil_critical_temperature_celsius,
            config.oil_minimum_cold_engine_torque_fraction,
            config.minimum_hot_engine_torque_fraction,
        );
        self.available_engine_torque_fraction = water_fraction.min(oil_fraction).clamp(
            config.minimum_hot_engine_torque_fraction.clamp(0.0, 1.0),
            1.0,
        );
    }

    pub fn evaluate_cooling_ducts(
        &mut self,
        config: &PowertrainThermalConfig,
        air_density_kilograms_per_cubic_meter: f64,
        vehicle_speed_meters_per_second: f64,
    ) {
        self.water_cooling_duct_flow = evaluate_cooling_duct_flow(
            &config.water_cooling_duct,
            air_density_kilograms_per_cubic_meter,
            vehicle_speed_meters_per_second,
        );
        self.oil_cooling_duct_flow = evaluate_cooling_duct_flow(
            &config.oil_cooling_duct,
            air_density_kilograms_per_cubic_meter,
            vehicle_speed_meters_per_second,
        );
        self.total_powertrain_cooling_drag_force_newtons = finite_nonnegative(
            self.water_cooling_duct_flow.drag_force_newtons
                + self.oil_cooling_duct_flow.drag_force_newtons,
        );
    }

    pub fn advance_temperatures(
        &mut self,
        config: &PowertrainThermalConfig,
        input: PowertrainThermalInput,
        delta_time_seconds: f64,
    ) {
        self.engine_block_temperature_celsius = finite_or(
            self.engine_block_temperature_celsius,
            config.initial_engine_block_temperature_celsius,
        );
        self.water_temperature_celsius = finite_or(
            self.water_temperature_celsius,
            config.initial_water_temperature_celsius,
        );
        self.oil_temperature_celsius = finite_or(
            self.oil_temperature_celsius,
            config.initial_oil_temperature_celsius,
        );
        let delta_time_seconds = if delta_time_seconds.is_finite() {
            delta_time_seconds.clamp(MINIMUM_THERMAL_STEP_SECONDS, MAXIMUM_THERMAL_STEP_SECONDS)
        } else {
            MINIMUM_THERMAL_STEP_SECONDS
        };
        let engine_speed_revolutions_per_minute =
            finite_nonnegative(input.engine_speed_revolutions_per_minute);
        let engine_torque_newton_meters = finite_nonnegative(input.engine_torque_newton_meters);
        let angular_velocity_radians_per_second =
            engine_speed_revolutions_per_minute * 2.0 * std::f64::consts::PI / 60.0;
        let mechanical_power_watts =
            finite_nonnegative(engine_torque_newton_meters * angular_velocity_radians_per_second);
        let generated_engine_heat_watts = finite_nonnegative(
            config.idle_heat_generation_watts
                + mechanical_power_watts * config.heat_generation_per_mechanical_power,
        );
        let generated_exhaust_heat_watts =
            generated_engine_heat_watts * config.exhaust_heat_fraction;
        let generated_engine_block_heat_watts =
            generated_engine_heat_watts * config.engine_block_heat_fraction;
        let generated_water_heat_watts = generated_engine_heat_watts * config.water_heat_fraction;
        let generated_oil_heat_watts = generated_engine_heat_watts * config.oil_heat_fraction;
        let engine_to_water_heat_transfer_watts = finite_value(
            config.engine_to_water_conductance_watts_per_kelvin
                * (self.engine_block_temperature_celsius - self.water_temperature_celsius),
            0.0,
        );
        let engine_to_oil_heat_transfer_watts = finite_value(
            config.engine_to_oil_conductance_watts_per_kelvin
                * (self.engine_block_temperature_celsius - self.oil_temperature_celsius),
            0.0,
        );
        let ambient_temperature_celsius = finite_or(
            input.ambient_temperature_celsius,
            config.initial_water_temperature_celsius,
        );
        let water_total_cooling_conductance_watts_per_kelvin = finite_nonnegative(
            config
                .water_cooling_duct
                .base_cooling_conductance_watts_per_kelvin
                + self
                    .water_cooling_duct_flow
                    .cooling_conductance_watts_per_kelvin,
        );
        let oil_total_cooling_conductance_watts_per_kelvin = finite_nonnegative(
            config
                .oil_cooling_duct
                .base_cooling_conductance_watts_per_kelvin
                + self
                    .oil_cooling_duct_flow
                    .cooling_conductance_watts_per_kelvin,
        );
        let water_rejected_heat_watts = finite_value(
            water_total_cooling_conductance_watts_per_kelvin
                * (self.water_temperature_celsius - ambient_temperature_celsius),
            0.0,
        );
        let oil_rejected_heat_watts = finite_value(
            oil_total_cooling_conductance_watts_per_kelvin
                * (self.oil_temperature_celsius - ambient_temperature_celsius),
            0.0,
        );
        let engine_block_net_heat_watts = finite_value(
            generated_engine_block_heat_watts
                - engine_to_water_heat_transfer_watts
                - engine_to_oil_heat_transfer_watts,
            0.0,
        );
        let water_net_heat_watts = finite_value(
            generated_water_heat_watts + engine_to_water_heat_transfer_watts
                - water_rejected_heat_watts,
            0.0,
        );
        let oil_net_heat_watts = finite_value(
            generated_oil_heat_watts + engine_to_oil_heat_transfer_watts - oil_rejected_heat_watts,
            0.0,
        );
        self.engine_block_temperature_celsius = finite_or(
            self.engine_block_temperature_celsius
                + engine_block_net_heat_watts
                    / config.engine_block_heat_capacity_joules_per_kelvin.max(1.0)
                    * delta_time_seconds,
            self.engine_block_temperature_celsius,
        );
        self.water_temperature_celsius = finite_or(
            self.water_temperature_celsius
                + water_net_heat_watts / config.water_heat_capacity_joules_per_kelvin.max(1.0)
                    * delta_time_seconds,
            self.water_temperature_celsius,
        );
        self.oil_temperature_celsius = finite_or(
            self.oil_temperature_celsius
                + oil_net_heat_watts / config.oil_heat_capacity_joules_per_kelvin.max(1.0)
                    * delta_time_seconds,
            self.oil_temperature_celsius,
        );
        self.generated_engine_heat_watts = generated_engine_heat_watts;
        self.generated_exhaust_heat_watts = generated_exhaust_heat_watts;
        self.generated_engine_block_heat_watts = generated_engine_block_heat_watts;
        self.generated_water_heat_watts = generated_water_heat_watts;
        self.generated_oil_heat_watts = generated_oil_heat_watts;
        self.engine_to_water_heat_transfer_watts = engine_to_water_heat_transfer_watts;
        self.engine_to_oil_heat_transfer_watts = engine_to_oil_heat_transfer_watts;
        self.water_rejected_heat_watts = water_rejected_heat_watts;
        self.oil_rejected_heat_watts = oil_rejected_heat_watts;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PowertrainThermalInput {
    pub engine_speed_revolutions_per_minute: f64,
    pub engine_torque_newton_meters: f64,
    pub driver_throttle_fraction: f64,
    pub vehicle_speed_meters_per_second: f64,
    pub air_density_kilograms_per_cubic_meter: f64,
    pub ambient_temperature_celsius: f64,
}

pub fn evaluate_cooling_duct_flow(
    config: &PowertrainCoolingDuctConfig,
    air_density_kilograms_per_cubic_meter: f64,
    vehicle_speed_meters_per_second: f64,
) -> PowertrainCoolingDuctFlowState {
    let air_density_kilograms_per_cubic_meter =
        finite_or(air_density_kilograms_per_cubic_meter, 1.225).clamp(0.5, 1.6);
    let vehicle_speed_meters_per_second = finite_or(vehicle_speed_meters_per_second, 0.0).abs();
    let opening = finite_or(config.opening, 0.0).clamp(0.0, 1.0);
    let maximum_inlet_area_square_meters =
        finite_nonnegative(config.maximum_inlet_area_square_meters);
    let effective_inlet_area_square_meters = finite_nonnegative(
        maximum_inlet_area_square_meters
            * opening.powf(config.area_response_exponent.clamp(0.05, 5.0)),
    );
    let dynamic_pressure_pascals = 0.5
        * air_density_kilograms_per_cubic_meter
        * vehicle_speed_meters_per_second
        * vehicle_speed_meters_per_second;
    let pressure_difference_pascals =
        dynamic_pressure_pascals * finite_nonnegative(config.pressure_recovery).min(1.5);
    let discharge_coefficient = finite_nonnegative(config.discharge_coefficient).min(1.5);
    let mass_flow_kilograms_per_second =
        if effective_inlet_area_square_meters > 0.0 && pressure_difference_pascals > 0.0 {
            finite_nonnegative(
                discharge_coefficient
                    * effective_inlet_area_square_meters
                    * (2.0 * air_density_kilograms_per_cubic_meter * pressure_difference_pascals)
                        .sqrt(),
            )
        } else {
            0.0
        };
    let capacity_rate_watts_per_kelvin = mass_flow_kilograms_per_second
        * finite_nonnegative(config.air_specific_heat_joules_per_kilogram_kelvin);
    let heat_exchanger_conductance_watts_per_kelvin =
        finite_nonnegative(config.heat_exchanger_conductance_watts_per_kelvin);
    let cooling_conductance_watts_per_kelvin = if capacity_rate_watts_per_kelvin > 1.0e-9
        && heat_exchanger_conductance_watts_per_kelvin > 0.0
    {
        finite_nonnegative(
            capacity_rate_watts_per_kelvin
                * (1.0
                    - (-heat_exchanger_conductance_watts_per_kelvin
                        / capacity_rate_watts_per_kelvin)
                        .exp()),
        )
    } else {
        0.0
    };
    let drag_force_newtons = finite_nonnegative(
        dynamic_pressure_pascals
            * finite_nonnegative(config.drag_coefficient)
            * effective_inlet_area_square_meters,
    );
    PowertrainCoolingDuctFlowState {
        effective_inlet_area_square_meters,
        mass_flow_kilograms_per_second,
        cooling_conductance_watts_per_kelvin,
        drag_force_newtons,
    }
}

fn temperature_torque_fraction(
    temperature_celsius: f64,
    optimal_minimum_temperature_celsius: f64,
    hot_derating_temperature_celsius: f64,
    critical_temperature_celsius: f64,
    minimum_cold_engine_torque_fraction: f64,
    minimum_hot_engine_torque_fraction: f64,
) -> f64 {
    if !temperature_celsius.is_finite() {
        return minimum_hot_engine_torque_fraction.clamp(0.0, 1.0);
    }
    if temperature_celsius < optimal_minimum_temperature_celsius {
        let cold_fraction = ((optimal_minimum_temperature_celsius - temperature_celsius)
            / COLD_TEMPERATURE_PENALTY_SPAN_CELSIUS)
            .clamp(0.0, 1.0);
        return (1.0 - (1.0 - minimum_cold_engine_torque_fraction.clamp(0.0, 1.0)) * cold_fraction)
            .clamp(minimum_hot_engine_torque_fraction.clamp(0.0, 1.0), 1.0);
    }
    if temperature_celsius <= hot_derating_temperature_celsius {
        return 1.0;
    }
    let hot_fraction = ((temperature_celsius - hot_derating_temperature_celsius)
        / (critical_temperature_celsius - hot_derating_temperature_celsius).max(1.0))
    .clamp(0.0, 1.0)
    .powf(1.35);
    (1.0 - (1.0 - minimum_hot_engine_torque_fraction.clamp(0.0, 1.0)) * hot_fraction)
        .clamp(minimum_hot_engine_torque_fraction.clamp(0.0, 1.0), 1.0)
}

fn finite_nonnegative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_value(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    finite_value(value, fallback)
}

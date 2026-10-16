use crate::brake_thermals::{BrakeThermalInput, BrakeThermalSystem, BrakeToTireHeat};
use crate::powertrain_thermals::PowertrainThermalSystem;
use crate::simulation::{CoupledThermalAdvanceContext, VehicleState};
use crate::tire_thermals::{
    normalized_contact_zone_weights, AccumulatedTireThermalWork, TireEnvironment,
    TireMechanicalModifiers, TireThermalInput, TireThermalSystem,
};
use crate::tire_wear::TireWearSystem;
use crate::types::WheelIndex;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlowStateBackend {
    #[default]
    Standard,
    Simplified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlowStateUpdateFrequencies {
    pub tire_frequency_hertz: f64,
    pub brake_frequency_hertz: f64,
    pub powertrain_frequency_hertz: f64,
}

impl SlowStateUpdateFrequencies {
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.tire_frequency_hertz,
            self.brake_frequency_hertz,
            self.powertrain_frequency_hertz,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0 || *value > 120.0)
        {
            return Err(
                "Slow-state update frequencies must be finite and in (0, 120] hertz".into(),
            );
        }
        Ok(())
    }
}

impl Default for SlowStateUpdateFrequencies {
    fn default() -> Self {
        static FREQUENCIES: std::sync::OnceLock<SlowStateUpdateFrequencies> =
            std::sync::OnceLock::new();
        *FREQUENCIES.get_or_init(|| {
            let frequencies: Self =
                serde_json::from_str(include_str!("../../../data/config/slow_state_updates.json"))
                    .expect("Invalid slow-state update configuration");
            frequencies
                .validate()
                .expect("Invalid slow-state update frequencies");
            frequencies
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AccumulatedBrakeThermalWork {
    pub mechanical_energy_joules: f64,
    pub vehicle_speed_seconds: f64,
    pub duct_conductance_seconds: f64,
    pub tire_carcass_heat_joules: f64,
    pub tire_gas_heat_joules: f64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SlowStateAccounting {
    pub mechanical_intervals: u64,
    pub elapsed_physical_seconds: f64,
    pub tire_updates: u64,
    pub brake_updates: u64,
    pub powertrain_updates: u64,
    pub friction_work_joules: [f64; 4],
    pub brake_work_joules: [f64; 4],
    pub tire_generated_heat_joules: [f64; 4],
    pub brake_to_tire_heat_joules: [f64; 4],
    pub engine_generated_heat_joules: f64,
    pub engine_mechanical_work_joules: f64,
    pub engine_positive_mechanical_work_joules: f64,
    pub applied_tire_generated_heat_joules: [f64; 4],
    pub applied_brake_work_joules: [f64; 4],
    pub applied_engine_generated_heat_joules: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowStateSystem {
    pub backend: SlowStateBackend,
    pub frequencies: SlowStateUpdateFrequencies,
    pub accounting: SlowStateAccounting,
    pub tire_elapsed_seconds: f64,
    pub brake_elapsed_seconds: f64,
    pub powertrain_elapsed_seconds: f64,
    pub pending_tire_work: [AccumulatedTireThermalWork; 4],
    pub pending_brake_work: [AccumulatedBrakeThermalWork; 4],
    pub pending_engine_heat_joules: f64,
    pub pending_water_conductance_seconds: f64,
    pub pending_oil_conductance_seconds: f64,
    #[serde(skip)]
    pub cached_tire_modifiers: Option<[TireMechanicalModifiers; 4]>,
    pub pending_tire_generated_heat_joules: [f64; 4],
}

impl Default for SlowStateSystem {
    fn default() -> Self {
        Self::new(SlowStateBackend::Standard)
    }
}

#[derive(Clone, Copy)]
struct WheelMechanicalSample {
    brake_input: BrakeThermalInput,
    tire_input: TireThermalInput,
    brake_energy_joules: f64,
    friction_power_watts: f64,
    additional_carcass_heat_watts: f64,
    surface_friction: f64,
}

impl SlowStateSystem {
    pub fn new(backend: SlowStateBackend) -> Self {
        Self {
            backend,
            frequencies: SlowStateUpdateFrequencies::default(),
            accounting: SlowStateAccounting::default(),
            tire_elapsed_seconds: 0.0,
            brake_elapsed_seconds: 0.0,
            powertrain_elapsed_seconds: 0.0,
            pending_tire_work: [AccumulatedTireThermalWork::default(); 4],
            pending_brake_work: [AccumulatedBrakeThermalWork::default(); 4],
            pending_engine_heat_joules: 0.0,
            pending_water_conductance_seconds: 0.0,
            pending_oil_conductance_seconds: 0.0,
            cached_tire_modifiers: None,
            pending_tire_generated_heat_joules: [0.0; 4],
        }
    }

    pub fn from_execution_environment() -> Self {
        let backend = match std::env::var("FORMULA90_SLOW_STATE_BACKEND").as_deref() {
            Ok("simplified") => SlowStateBackend::Simplified,
            Ok("standard") | Err(_) => SlowStateBackend::Standard,
            Ok(value) => panic!("Unsupported slow-state backend: {value}"),
        };
        let mut system = Self::new(backend);
        if backend == SlowStateBackend::Simplified {
            if let Ok(configuration_path) = std::env::var("FORMULA90_SLOW_STATE_CONFIGURATION") {
                let document = std::fs::read_to_string(configuration_path)
                    .expect("Cannot read canonical slow-state update configuration");
                system.frequencies = serde_json::from_str(&document)
                    .expect("Invalid canonical slow-state update configuration");
                system
                    .frequencies
                    .validate()
                    .expect("Invalid canonical slow-state update frequencies");
            }
        }
        if backend == SlowStateBackend::Simplified {
            eprintln!("SLOW_STATE_BACKEND_READY backend=simplified tires_hertz={} brakes_hertz={} powertrain_hertz={}",
                system.frequencies.tire_frequency_hertz, system.frequencies.brake_frequency_hertz, system.frequencies.powertrain_frequency_hertz);
        }
        system
    }

    pub fn flush_tires(&mut self, configuration: &VehicleConfig, tires: &mut TireThermalSystem) {
        if self.tire_elapsed_seconds <= 0.0 {
            return;
        }
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let thermal = configuration.tires.tire_thermal.for_wheel(wheel);
            tires.advance_accumulated_work(
                wheel,
                &configuration.tires.tire_pressure,
                thermal,
                TireEnvironment::fallback(thermal),
                self.pending_tire_work[index],
                self.tire_elapsed_seconds,
            );
            self.accounting.applied_tire_generated_heat_joules[index] +=
                self.pending_tire_generated_heat_joules[index];
        }
        self.pending_tire_work = [AccumulatedTireThermalWork::default(); 4];
        self.pending_tire_generated_heat_joules = [0.0; 4];
        self.tire_elapsed_seconds = 0.0;
        self.cached_tire_modifiers = None;
        self.accounting.tire_updates += 1;
    }

    fn flush_brakes(&mut self, configuration: &VehicleConfig, brakes: &mut BrakeThermalSystem) {
        let duration = self.brake_elapsed_seconds;
        if duration <= 0.0 {
            return;
        }
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let work = self.pending_brake_work[index];
            let instantaneous_duct = brakes.wheels[index].duct;
            let mut duct = instantaneous_duct;
            duct.cooling_conductance_w_k = work.duct_conductance_seconds / duration;
            let input = BrakeThermalInput {
                vehicle_speed_ms: work.vehicle_speed_seconds / duration,
                air_density_kg_m3: configuration.aerodynamics.air_density,
                ambient_temperature_c: configuration
                    .tires
                    .tire_thermal
                    .for_wheel(wheel)
                    .ambient_fallback_c,
                ..BrakeThermalInput::default()
            };
            brakes.advance_accumulated_work(
                wheel,
                &configuration.brakes.brake_thermal,
                input,
                work.mechanical_energy_joules,
                duration,
                duct,
                BrakeToTireHeat {
                    carcass_heat_w: work.tire_carcass_heat_joules / duration,
                    gas_heat_w: work.tire_gas_heat_joules / duration,
                },
            );
            brakes.wheels[index].duct = instantaneous_duct;
            self.accounting.applied_brake_work_joules[index] += work.mechanical_energy_joules;
        }
        self.pending_brake_work = [AccumulatedBrakeThermalWork::default(); 4];
        self.brake_elapsed_seconds = 0.0;
        self.accounting.brake_updates += 1;
    }

    fn flush_powertrain(
        &mut self,
        configuration: &VehicleConfig,
        powertrain: &mut PowertrainThermalSystem,
    ) {
        let duration = self.powertrain_elapsed_seconds;
        if duration <= 0.0 {
            return;
        }
        let water_flow = powertrain.water_cooling_duct_flow;
        let oil_flow = powertrain.oil_cooling_duct_flow;
        powertrain
            .water_cooling_duct_flow
            .cooling_conductance_watts_per_kelvin =
            self.pending_water_conductance_seconds / duration;
        powertrain
            .oil_cooling_duct_flow
            .cooling_conductance_watts_per_kelvin = self.pending_oil_conductance_seconds / duration;
        powertrain.advance_accumulated_heat(
            &configuration.powertrain.powertrain_thermal,
            configuration.tires.tire_thermal.front.ambient_fallback_c,
            self.pending_engine_heat_joules,
            duration,
        );
        powertrain.water_cooling_duct_flow = water_flow;
        powertrain.oil_cooling_duct_flow = oil_flow;
        self.accounting.applied_engine_generated_heat_joules += self.pending_engine_heat_joules;
        self.pending_engine_heat_joules = 0.0;
        self.pending_water_conductance_seconds = 0.0;
        self.pending_oil_conductance_seconds = 0.0;
        self.powertrain_elapsed_seconds = 0.0;
        self.accounting.powertrain_updates += 1;
    }

    pub fn flush_pending(
        &mut self,
        configuration: &VehicleConfig,
        tires: &mut TireThermalSystem,
        brakes: &mut BrakeThermalSystem,
        powertrain: &mut PowertrainThermalSystem,
    ) {
        self.flush_brakes(configuration, brakes);
        self.flush_tires(configuration, tires);
        self.flush_powertrain(configuration, powertrain);
    }

    fn flush_ready(
        &mut self,
        configuration: &VehicleConfig,
        tires: &mut TireThermalSystem,
        brakes: &mut BrakeThermalSystem,
        powertrain: &mut PowertrainThermalSystem,
    ) {
        let tolerance = f64::EPSILON * 16.0;
        if self.brake_elapsed_seconds + tolerance >= 1.0 / self.frequencies.brake_frequency_hertz {
            self.flush_brakes(configuration, brakes);
        }
        if self.tire_elapsed_seconds + tolerance >= 1.0 / self.frequencies.tire_frequency_hertz {
            self.flush_tires(configuration, tires);
        }
        if self.powertrain_elapsed_seconds + tolerance
            >= 1.0 / self.frequencies.powertrain_frequency_hertz
        {
            self.flush_powertrain(configuration, powertrain);
        }
    }

    fn advance_samples(
        &mut self,
        configuration: &VehicleConfig,
        tires: &mut TireThermalSystem,
        brakes: &mut BrakeThermalSystem,
        powertrain: &mut PowertrainThermalSystem,
        wear: &mut TireWearSystem,
        samples: &[WheelMechanicalSample; 4],
        engine_heat_watts: f64,
        duration_seconds: f64,
    ) -> Result<(), String> {
        self.frequencies.validate()?;
        self.accounting.mechanical_intervals += 1;
        self.accounting.elapsed_physical_seconds += duration_seconds;
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            self.accounting.friction_work_joules[index] +=
                samples[index].friction_power_watts * duration_seconds;
            self.accounting.brake_work_joules[index] += samples[index].brake_energy_joules;
            brakes.record_mechanical_energy(
                wheel,
                samples[index].brake_input,
                samples[index].brake_energy_joules,
                duration_seconds,
            );
        }
        self.accounting.engine_generated_heat_joules += engine_heat_watts * duration_seconds;
        let mut remaining = duration_seconds;
        while remaining > 0.0 {
            self.flush_ready(configuration, tires, brakes, powertrain);
            let next_boundary = (1.0 / self.frequencies.tire_frequency_hertz
                - self.tire_elapsed_seconds)
                .min(1.0 / self.frequencies.brake_frequency_hertz - self.brake_elapsed_seconds)
                .min(
                    1.0 / self.frequencies.powertrain_frequency_hertz
                        - self.powertrain_elapsed_seconds,
                );
            let interval = remaining.min(next_boundary).min(0.05);
            if interval <= 0.0 {
                return Err("Invalid slow-state scheduling phase".into());
            }
            for wheel in WheelIndex::ALL {
                let index = wheel as usize;
                let sample = samples[index];
                let thermal = configuration.tires.tire_thermal.for_wheel(wheel);
                let tire_state = tires.wheels[index];
                let brake_state = brakes.wheels[index];
                let brake_configuration = if wheel.is_front() {
                    &configuration.brakes.brake_thermal.front
                } else {
                    &configuration.brakes.brake_thermal.rear
                };
                let zone_weights = normalized_contact_zone_weights(
                    sample.tire_input.zone_contact_weights,
                    (tire_state.pressure_kpa_gauge
                        / configuration.tires.tire_pressure.reference_hot_kpa_gauge[index]
                            .max(20.0))
                    .clamp(0.45, 1.80),
                    sample.tire_input.dynamic_camber_rad,
                );
                let carcass_transfer_watts = brake_configuration.rim_to_tire_carcass_w_k
                    * (brake_state.rim_c - tire_state.carcass_c);
                let gas_transfer_watts = brake_configuration.rim_to_tire_gas_w_k
                    * (brake_state.rim_c - tire_state.gas_c);
                let slip_heat_watts =
                    sample.friction_power_watts * thermal.slip_heat_efficiency.clamp(0.0, 1.5);
                let flex_heat_watts = sample.tire_input.normal_force_n.max(0.0)
                    * sample.tire_input.tire_deflection_velocity_m_s.abs()
                    * thermal.carcass_hysteresis_efficiency.clamp(0.0, 1.0)
                    + sample.additional_carcass_heat_watts;
                let tire_work = &mut self.pending_tire_work[index];
                for zone in 0..3 {
                    tire_work.tread_slip_heat_joules[zone] +=
                        slip_heat_watts * zone_weights[zone] * interval;
                    tire_work.contact_zone_seconds[zone] += zone_weights[zone] * interval;
                }
                let airflow =
                    (1.0 + sample.tire_input.vehicle_speed_ms.abs() * 0.020).clamp(1.0, 4.0);
                tire_work.airflow_seconds += airflow * interval;
                tire_work.carcass_airflow_seconds += airflow.sqrt() * interval;
                tire_work.carcass_heat_joules +=
                    (flex_heat_watts + carcass_transfer_watts) * interval;
                tire_work.gas_heat_joules += gas_transfer_watts * interval;
                tire_work.last_deflection_ratio = if sample.tire_input.max_tire_deflection_m > 1e-5
                {
                    (sample.tire_input.tire_deflection_m.max(0.0)
                        / sample.tire_input.max_tire_deflection_m)
                        .clamp(0.0, 1.5)
                } else {
                    0.0
                };
                let generated_heat = (slip_heat_watts * zone_weights.iter().sum::<f64>()
                    + flex_heat_watts)
                    * interval;
                self.pending_tire_generated_heat_joules[index] += generated_heat;
                self.accounting.tire_generated_heat_joules[index] += generated_heat;
                self.accounting.brake_to_tire_heat_joules[index] +=
                    (carcass_transfer_watts + gas_transfer_watts) * interval;
                let brake_work = &mut self.pending_brake_work[index];
                brake_work.mechanical_energy_joules +=
                    sample.brake_energy_joules * (interval / duration_seconds);
                brake_work.vehicle_speed_seconds += sample.brake_input.vehicle_speed_ms * interval;
                brake_work.duct_conductance_seconds +=
                    brake_state.duct.cooling_conductance_w_k * interval;
                brake_work.tire_carcass_heat_joules += carcass_transfer_watts * interval;
                brake_work.tire_gas_heat_joules += gas_transfer_watts * interval;
                wear.step_after_forces(
                    wheel,
                    configuration.tires.tire_wear.for_wheel(wheel),
                    sample.surface_friction,
                    zone_weights,
                    [
                        tire_state.tread_inner_c,
                        tire_state.tread_center_c,
                        tire_state.tread_outer_c,
                    ],
                    sample.tire_input.longitudinal_force_n,
                    sample.tire_input.lateral_force_n,
                    sample.tire_input.slip_velocity_long_ms,
                    sample.tire_input.slip_velocity_lat_ms,
                    interval,
                );
            }
            self.pending_engine_heat_joules += engine_heat_watts * interval;
            self.pending_water_conductance_seconds += powertrain
                .water_cooling_duct_flow
                .cooling_conductance_watts_per_kelvin
                * interval;
            self.pending_oil_conductance_seconds += powertrain
                .oil_cooling_duct_flow
                .cooling_conductance_watts_per_kelvin
                * interval;
            self.tire_elapsed_seconds += interval;
            self.brake_elapsed_seconds += interval;
            self.powertrain_elapsed_seconds += interval;
            remaining = (remaining - interval).max(0.0);
            self.flush_ready(configuration, tires, brakes, powertrain);
        }
        Ok(())
    }
}

pub fn advance_after_mechanics(
    configuration: &VehicleConfig,
    state: &mut VehicleState,
    context: &CoupledThermalAdvanceContext,
    brake_energy_joules: Option<[f64; 4]>,
) -> Result<(), String> {
    let duration = context.duration_seconds;
    if !duration.is_finite() || duration <= 0.0 {
        return Err("Invalid slow-state physical duration".into());
    }
    if brake_energy_joules.is_some_and(|energies| {
        energies
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    }) {
        return Err("Invalid accumulated brake work".into());
    }
    let mut sample_error = None;
    let samples = std::array::from_fn(|index| {
        let wheel = WheelIndex::ALL[index];
        let suspension = &state.suspension.wheels[index];
        let tire = &state.tires.wheels[index];
        let thermal = configuration.tires.tire_thermal.for_wheel(wheel);
        let tuning = crate::wheel_mechanics::WheelMechanicalTuning::for_wheel(configuration, wheel);
        let brake_input = BrakeThermalInput {
            applied_brake_torque_nm: context.applied_brake_torque_newton_metres[index],
            wheel_spin_pre_rad_s: context.wheel_spin_before_radians_per_second[index],
            wheel_spin_post_rad_s: tire.spin,
            vehicle_speed_ms: context.vehicle_speed_metres_per_second,
            air_density_kg_m3: configuration.aerodynamics.air_density,
            ambient_temperature_c: thermal.ambient_fallback_c,
            tire_carcass_temperature_c: state.tire_thermal.wheels[index].carcass_c,
            tire_gas_temperature_c: state.tire_thermal.wheels[index].gas_c,
        };
        let mechanical_brake_energy = brake_energy_joules
            .map(|energies| energies[index])
            .unwrap_or_else(|| {
                brake_input.applied_brake_torque_nm.abs()
                    * ((brake_input.wheel_spin_pre_rad_s + brake_input.wheel_spin_post_rad_s) * 0.5)
                        .abs()
                    * duration
            });
        let additional_carcass_heat_watts = if brake_energy_joules.is_some() {
            if let Some(coupled) = &configuration.coupled_world {
                if suspension.total_normal_force > 0.0 {
                    coupled
                        .suspension
                        .tire_vertical_damping_newton_seconds_per_metre[index]
                        * suspension.tire_deflection_velocity_m_s.powi(2)
                        * thermal.carcass_hysteresis_efficiency.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            } else {
                sample_error = Some(
                    "Coupled thermal advancement requires suspension damping settings".to_string(),
                );
                0.0
            }
        } else {
            0.0
        };
        WheelMechanicalSample {
            brake_input,
            brake_energy_joules: mechanical_brake_energy,
            friction_power_watts: tire.longitudinal_force.abs() * tire.spin_velocity_diff.abs()
                + tire.lateral_force.abs()
                    * context.wheel_lateral_slip_metres_per_second[index].abs(),
            additional_carcass_heat_watts,
            surface_friction: suspension.effective_friction,
            tire_input: TireThermalInput {
                normal_force_n: suspension.total_normal_force.max(0.0),
                longitudinal_force_n: tire.longitudinal_force,
                lateral_force_n: tire.lateral_force,
                slip_velocity_long_ms: tire.spin_velocity_diff.abs(),
                slip_velocity_lat_ms: context.wheel_lateral_slip_metres_per_second[index],
                tire_deflection_m: suspension.tire_deflection_m,
                tire_deflection_velocity_m_s: if brake_energy_joules.is_some() {
                    0.0
                } else {
                    suspension.tire_deflection_velocity_m_s
                },
                max_tire_deflection_m: tuning.max_tire_deflection_m
                    * context.thermal_modifiers[index]
                        .max_deflection_scale
                        .max(0.05),
                dynamic_camber_rad: suspension.dynamic_camber,
                vehicle_speed_ms: context.vehicle_speed_metres_per_second,
                zone_contact_weights: [
                    if suspension.ray_grounded[0] { 1.0 } else { 0.0 },
                    if suspension.ray_grounded[1] { 2.0 } else { 0.0 },
                    if suspension.ray_grounded[2] { 1.0 } else { 0.0 },
                ],
                ..TireThermalInput::default()
            },
        }
    });
    if let Some(error) = sample_error {
        return Err(error);
    }
    if samples.iter().any(|sample| {
        !sample.brake_energy_joules.is_finite() || !sample.friction_power_watts.is_finite()
    }) {
        return Err("Nonfinite mechanical work in slow-state input".into());
    }
    let powertrain_configuration = &configuration.powertrain.powertrain_thermal;
    let signed_mechanical_power =
        state.powertrain.engine_torque * state.powertrain.rpm.max(0.0) * std::f64::consts::TAU
            / 60.0;
    if !signed_mechanical_power.is_finite() {
        return Err("Nonfinite engine mechanical work".into());
    }
    let mechanical_power = signed_mechanical_power.max(0.0);
    let engine_heat_watts = powertrain_configuration.idle_heat_generation_watts
        + mechanical_power * powertrain_configuration.heat_generation_per_mechanical_power;
    if !engine_heat_watts.is_finite() {
        return Err("Nonfinite engine heat source".into());
    }
    let result = state.slow_states.advance_samples(
        configuration,
        &mut state.tire_thermal,
        &mut state.brake_thermal,
        &mut state.powertrain_thermal,
        &mut state.tire_wear,
        &samples,
        engine_heat_watts,
        duration,
    );
    if result.is_ok() {
        state.slow_states.accounting.engine_mechanical_work_joules +=
            signed_mechanical_power * duration;
        state
            .slow_states
            .accounting
            .engine_positive_mechanical_work_joules += mechanical_power * duration;
        for wheel in WheelIndex::ALL {
            let input = samples[wheel as usize].tire_input;
            let deflection_ratio = if input.max_tire_deflection_m > 1e-5 {
                (input.tire_deflection_m.max(0.0) / input.max_tire_deflection_m).clamp(0.0, 1.5)
            } else {
                0.0
            };
            if state
                .tire_thermal
                .refresh_pressure_from_mechanical_deflection(
                    wheel,
                    &configuration.tires.tire_pressure,
                    configuration.tires.tire_thermal.for_wheel(wheel),
                    deflection_ratio,
                )
            {
                state.slow_states.cached_tire_modifiers = None;
            }
        }
    }
    result
}

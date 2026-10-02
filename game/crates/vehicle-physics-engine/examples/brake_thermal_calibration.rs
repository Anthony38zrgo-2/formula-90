use serde::Serialize;
use std::{collections::HashMap, error::Error, fs, path::Path};
use vehicle_physics_engine::{BrakeThermalInput, BrakeThermalSystem, VehicleConfig, WheelIndex};

#[derive(Clone)]
struct RecordedBrakeSample {
    time_seconds: f64,
    speed_meters_per_second: f64,
    requested_wheel_torque: [f64; 4],
    wheel_spin_before: [f64; 4],
    wheel_spin_after: [f64; 4],
    tire_carcass_temperature: [f64; 4],
    tire_gas_temperature: [f64; 4],
}

fn parse_record(line: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut columns = Vec::new();
    let mut column = String::new();
    let mut quoted = false;
    let mut characters = line.trim_end_matches('\r').chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted && characters.peek() == Some(&'"') => {
                column.push('"');
                characters.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => columns.push(std::mem::take(&mut column)),
            _ => column.push(character),
        }
    }
    if quoted {
        return Err("Unterminated quoted telemetry field".into());
    }
    columns.push(column);
    Ok(columns)
}

fn load_samples(path: &Path) -> Result<(Vec<RecordedBrakeSample>, usize), Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut lines = contents.trim_start_matches('\u{feff}').lines().peekable();
    let header = parse_record(lines.next().ok_or("Missing telemetry header")?)?;
    let indices: HashMap<_, _> = header
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut samples = Vec::new();
    let mut discarded_trailing_records = 0;
    while let Some(line) = lines.next() {
        let columns = match parse_record(line) {
            Ok(columns) if columns.len() == header.len() => columns,
            _ if lines.peek().is_none() => {
                discarded_trailing_records += 1;
                continue;
            }
            _ => return Err("Incomplete telemetry record before end of file".into()),
        };
        let number = |name: &str| -> Result<f64, Box<dyn Error>> {
            let index = *indices
                .get(name)
                .ok_or_else(|| format!("Missing column {name}"))?;
            let value: f64 = columns[index].parse()?;
            if !value.is_finite() {
                return Err(format!("Nonfinite column {name}").into());
            }
            Ok(value)
        };
        let mut sample = RecordedBrakeSample {
            time_seconds: number("Time_ms")? / 1000.0,
            speed_meters_per_second: number("Speed_kmh")? / 3.6,
            requested_wheel_torque: [0.0; 4],
            wheel_spin_before: [0.0; 4],
            wheel_spin_after: [0.0; 4],
            tire_carcass_temperature: [0.0; 4],
            tire_gas_temperature: [0.0; 4],
        };
        for (index, label) in ["FL", "FR", "RL", "RR"].iter().enumerate() {
            let efficiency = number(&format!("{label}_BrakeEfficiency"))?;
            let torque = number(&format!("{label}_BrakeTorque_Nm"))?;
            if torque.abs() > 0.01 && efficiency <= 0.0 {
                return Err("Invalid recorded brake efficiency".into());
            }
            sample.requested_wheel_torque[index] = torque.max(0.0) / efficiency.max(0.01);
            sample.wheel_spin_before[index] = number(&format!("{label}_SpinPre_RadS"))?;
            sample.wheel_spin_after[index] = number(&format!("{label}_SpinPost_RadS"))?;
            sample.tire_carcass_temperature[index] = number(&format!("{label}_Carcass_C"))?;
            sample.tire_gas_temperature[index] = number(&format!("{label}_Gas_C"))?;
        }
        if let Some(previous) = samples.last() {
            let previous: &RecordedBrakeSample = previous;
            if sample.time_seconds <= previous.time_seconds {
                return Err("Nonincreasing telemetry time".into());
            }
        }
        samples.push(sample);
    }
    if samples.len() < 2 {
        return Err("At least two telemetry samples are required".into());
    }
    Ok((samples, discarded_trailing_records))
}

#[derive(Serialize)]
struct WheelThermalResult {
    mean_final_cycle_temperature_celsius: f64,
    minimum_final_cycle_temperature_celsius: f64,
    maximum_temperature_celsius: f64,
    final_temperature_celsius: f64,
    final_cycle_below_optimum_fraction: f64,
    final_cycle_in_optimum_fraction: f64,
    final_cycle_above_optimum_fraction: f64,
    final_cycle_fading_fraction: f64,
    first_optimum_time_seconds: Option<f64>,
    maximum_braking_power_watts: f64,
    total_braking_energy_joules: f64,
}

#[derive(Serialize)]
struct ScenarioResult {
    brake_demand_scale: f64,
    front_duct_opening: f64,
    rear_duct_opening: f64,
    duration_seconds: f64,
    wheels: Vec<WheelThermalResult>,
}

fn simulate(
    configuration: &VehicleConfig,
    samples: &[RecordedBrakeSample],
    repetitions: usize,
    brake_demand_scale: f64,
) -> ScenarioResult {
    let thermal_configuration = &configuration.brake_thermal;
    let mut system = BrakeThermalSystem::new(thermal_configuration);
    let first_time = samples[0].time_seconds;
    let cycle_duration = samples.last().unwrap().time_seconds - first_time;
    let steps_per_cycle = (cycle_duration * 120.0).ceil() as usize;
    let step_seconds = cycle_duration / steps_per_cycle as f64;
    let mut temperature_sums = [0.0; 4];
    let mut minimum_temperatures = [f64::INFINITY; 4];
    let mut maximum_temperatures = [thermal_configuration.initial_temperature_c; 4];
    let mut below_optimum_counts = [0; 4];
    let mut in_optimum_counts = [0; 4];
    let mut fading_counts = [0; 4];
    let mut first_optimum_times = [None; 4];
    let mut maximum_powers = [0.0_f64; 4];
    for repetition in 0..repetitions {
        let mut interval = 0;
        for step in 0..steps_per_cycle {
            let local_time = first_time + step as f64 * step_seconds;
            while interval + 2 < samples.len() && samples[interval + 1].time_seconds < local_time {
                interval += 1;
            }
            let before = &samples[interval];
            let after = &samples[interval + 1];
            let fraction = ((local_time - before.time_seconds)
                / (after.time_seconds - before.time_seconds))
                .clamp(0.0, 1.0);
            let interpolate = |before: f64, after: f64| before + (after - before) * fraction;
            let efficiencies = system.efficiency_scales();
            for wheel in WheelIndex::ALL {
                let index = wheel as usize;
                let requested_torque = interpolate(
                    before.requested_wheel_torque[index],
                    after.requested_wheel_torque[index],
                );
                system.step_after_braking(
                    wheel,
                    thermal_configuration,
                    BrakeThermalInput {
                        applied_brake_torque_nm: requested_torque
                            * brake_demand_scale
                            * efficiencies[index],
                        wheel_spin_pre_rad_s: interpolate(
                            before.wheel_spin_before[index],
                            after.wheel_spin_before[index],
                        ),
                        wheel_spin_post_rad_s: interpolate(
                            before.wheel_spin_after[index],
                            after.wheel_spin_after[index],
                        ),
                        vehicle_speed_ms: interpolate(
                            before.speed_meters_per_second,
                            after.speed_meters_per_second,
                        ),
                        air_density_kg_m3: configuration.air_density,
                        ambient_temperature_c: thermal_configuration.initial_temperature_c,
                        tire_carcass_temperature_c: interpolate(
                            before.tire_carcass_temperature[index],
                            after.tire_carcass_temperature[index],
                        ),
                        tire_gas_temperature_c: interpolate(
                            before.tire_gas_temperature[index],
                            after.tire_gas_temperature[index],
                        ),
                    },
                    step_seconds,
                );
                let state = &system.wheels[index];
                maximum_temperatures[index] = maximum_temperatures[index].max(state.disc_c);
                maximum_powers[index] = maximum_powers[index].max(state.brake_power_w);
                if state.disc_c >= thermal_configuration.optimal_min_temperature_c
                    && first_optimum_times[index].is_none()
                {
                    first_optimum_times[index] =
                        Some((repetition * steps_per_cycle + step + 1) as f64 * step_seconds);
                }
                if repetition + 1 == repetitions {
                    temperature_sums[index] += state.disc_c;
                    minimum_temperatures[index] = minimum_temperatures[index].min(state.disc_c);
                    below_optimum_counts[index] +=
                        usize::from(state.disc_c < thermal_configuration.optimal_min_temperature_c);
                    in_optimum_counts[index] += usize::from(
                        state.disc_c >= thermal_configuration.optimal_min_temperature_c
                            && state.disc_c <= thermal_configuration.optimal_max_temperature_c,
                    );
                    fading_counts[index] +=
                        usize::from(state.disc_c > thermal_configuration.fade_start_temperature_c);
                }
            }
        }
    }
    ScenarioResult {
        brake_demand_scale,
        front_duct_opening: thermal_configuration.front_duct.opening,
        rear_duct_opening: thermal_configuration.rear_duct.opening,
        duration_seconds: cycle_duration * repetitions as f64,
        wheels: (0..4)
            .map(|index| WheelThermalResult {
                mean_final_cycle_temperature_celsius: temperature_sums[index]
                    / steps_per_cycle as f64,
                minimum_final_cycle_temperature_celsius: minimum_temperatures[index],
                maximum_temperature_celsius: maximum_temperatures[index],
                final_temperature_celsius: system.wheels[index].disc_c,
                final_cycle_below_optimum_fraction: below_optimum_counts[index] as f64
                    / steps_per_cycle as f64,
                final_cycle_in_optimum_fraction: in_optimum_counts[index] as f64
                    / steps_per_cycle as f64,
                final_cycle_above_optimum_fraction: (steps_per_cycle
                    - below_optimum_counts[index]
                    - in_optimum_counts[index])
                    as f64
                    / steps_per_cycle as f64,
                final_cycle_fading_fraction: fading_counts[index] as f64 / steps_per_cycle as f64,
                first_optimum_time_seconds: first_optimum_times[index],
                maximum_braking_power_watts: maximum_powers[index],
                total_braking_energy_joules: system.wheels[index].brake_energy_j,
            })
            .collect(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() < 2 {
        return Err("Usage: brake_thermal_calibration vehicle_profile.json telemetry.csv [telemetry.csv ...]".into());
    }
    let configuration = VehicleConfig::from_json_path(Path::new(&arguments[0]))?;
    let mut trace_results = Vec::new();
    for trace_path in &arguments[1..] {
        let (samples, discarded_trailing_records) = load_samples(Path::new(trace_path))?;
        let mut scenarios = Vec::new();
        scenarios.push(simulate(&configuration, &samples, 1, 1.0));
        scenarios.push(simulate(&configuration, &samples, 5, 1.0));
        for opening in [0.0, 0.02, 0.04, 0.08, 0.12, 0.25, 0.5, 1.0] {
            let mut varied = configuration.clone();
            varied.brake_thermal.front_duct.opening = opening;
            varied.brake_thermal.rear_duct.opening = opening;
            scenarios.push(simulate(&varied, &samples, 5, 1.0));
        }
        for brake_demand_scale in [0.5, 1.5, 2.0] {
            scenarios.push(simulate(&configuration, &samples, 5, brake_demand_scale));
        }
        trace_results.push(serde_json::json!({
            "trace_path": trace_path,
            "sample_count": samples.len(),
            "discarded_trailing_records": discarded_trailing_records,
            "scenarios": scenarios,
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "vehicle_profile": arguments[0],
            "thermal_configuration": configuration.brake_thermal,
            "integration_frequency_hertz": 120,
            "wheel_order": ["front_left", "front_right", "rear_left", "rear_right"],
            "method": "Recorded wheel torque divided by recorded thermal efficiency, interpolated at 120 Hz and multiplied by simulated efficiency. Recorded wheel speeds and tire boundary temperatures remain prescribed. Repeated traces retain thermal state.",
            "limitation": "Thermal subsystem replay, not a new closed-loop lap or a prediction of changed wheel locking, tire temperatures, or driver inputs. Demand scales above one are thermal stress cases.",
            "traces": trace_results,
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_record;

    #[test]
    fn quoted_metadata_preserves_column_alignment() {
        assert_eq!(
            parse_record("1,\"{\"\"opening\"\":0.2,\"\"rear\"\":0.1}\",3\r").unwrap(),
            vec!["1", "{\"opening\":0.2,\"rear\":0.1}", "3"]
        );
        assert!(parse_record("1,\"unfinished").is_err());
    }
}

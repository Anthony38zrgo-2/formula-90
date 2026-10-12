use serde_json::{json, Value};
use std::path::Path;
use vehicle_physics_engine::coupled_vehicle_dynamics::{CoupledVehicleModel, CoupledVehicleState};
use vehicle_physics_engine::suspension_component_kinematics::wheel_contact_component;
use vehicle_physics_engine::suspension_coupled_solver::{
    CoupledSuspensionInput, CoupledWheelContact,
};
use vehicle_physics_engine::tire::TireSystem;
use vehicle_physics_engine::tire_thermals::TireThermalSystem;
use vehicle_physics_engine::{SurfaceType, Vec3, VehicleConfig, WheelIndex};

fn steering_trace(
    configuration: &VehicleConfig,
    wheel: WheelIndex,
    normal_force: f64,
    deflection: f64,
    angle: f64,
) -> Vec<f64> {
    let mut comparison_configuration = configuration.clone();
    comparison_configuration.chassis.vehicle_mass = 698.0;
    comparison_configuration.chassis.vehicle_mass_excludes_wheel_assemblies = false;
    let mut thermal = TireThermalSystem::new_with_axles(
        &configuration.tires.tire_pressure,
        &configuration.tires.tire_thermal,
    );
    for state in &mut thermal.wheels {
        state.tread_inner_c = 95.0;
        state.tread_center_c = 95.0;
        state.tread_outer_c = 95.0;
        state.carcass_c = 80.0;
        state.gas_c = 80.0;
        state.pressure_kpa_gauge = 145.0;
    }
    let modifiers = thermal.mechanical_modifiers(
        wheel,
        &configuration.tires.tire_pressure,
        configuration.tires.tire_thermal.for_wheel(wheel),
        configuration.tires.pressure_mechanics,
    );
    let mut tires = TireSystem::new(&comparison_configuration);
    tires.set_mechanical_state(
        &comparison_configuration,
        wheel,
        normal_force,
        0.0,
        deflection,
        1.0,
    );
    tires.set_mechanical_modifiers(wheel, modifiers);
    tires.wheels[wheel as usize].spin =
        30.0 / tires.wheels[wheel as usize].effective_rolling_radius;
    let mut forces = Vec::new();
    for sample in 0..1440 {
        let steering_angle = if (480..960).contains(&sample) {
            angle
        } else {
            0.0
        };
        let velocity = Vec3::new(-steering_angle.tan() * 30.0, 0.0, -30.0);
        tires.process_wheel_forces(
            &comparison_configuration,
            wheel,
            normal_force,
            SurfaceType::Road,
            configuration.tires.surface_friction[&SurfaceType::Road],
            configuration.tires.surface_stiffness[&SurfaceType::Road],
            1.0,
            false,
            velocity,
            1.0 / 960.0,
        );
        forces.push(tires.wheels[wheel as usize].lateral_force.abs());
    }
    forces
}

fn time_to_fraction(trace: &[f64], fraction: f64, release: bool) -> f64 {
    let steady_force = trace[959];
    let start = if release { 960 } else { 480 };
    let offset = trace[start..]
        .iter()
        .position(|force| {
            if release {
                *force <= steady_force * fraction
            } else {
                *force >= steady_force * fraction
            }
        })
        .unwrap();
    (offset + 1) as f64 / 960.0
}

fn clamped_hub_cycle(
    configuration: &VehicleConfig,
    wheel: WheelIndex,
    normal_force: f64,
) -> Result<Value, String> {
    let settings = configuration
        .coupled_world
        .as_ref()
        .unwrap()
        .suspension
        .clone();
    let stiffness = settings.tire_vertical_stiffness_newtons_per_metre[wheel as usize];
    let damping = settings.tire_vertical_damping_newton_seconds_per_metre[wheel as usize];
    let model = CoupledVehicleModel::new(configuration.clone(), settings)?;
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 1.0, 0.0));
    let mut input = CoupledSuspensionInput::default();
    let hub = wheel_contact_component(configuration, wheel, model.kinematic_input(&state, &input))?;
    let radius = if wheel.is_front() {
        configuration.tires.front_tire_radius
    } else {
        configuration.tires.rear_tire_radius
    };
    let static_deflection = normal_force / stiffness;
    let mut loop_energy = 0.0;
    let mut minimum_force = f64::INFINITY;
    let mut maximum_force: f64 = 0.0;
    let mut samples = Vec::new();
    let frequency = 8.0;
    let amplitude = 0.002;
    let duration = 1.0 / frequency;
    let step = duration / 120.0;
    for sample in 0..120 {
        let time = (sample as f64 + 0.5) * step;
        let phase = std::f64::consts::TAU * frequency * time;
        let displacement = amplitude * phase.sin();
        let velocity = amplitude * std::f64::consts::TAU * frequency * phase.cos();
        input.contacts[wheel as usize] = Some(CoupledWheelContact {
            surface_point_world_metres: hub.center_of_mass_world_metres
                - Vec3::UP * (radius - static_deflection - displacement),
            surface_normal_world: Vec3::UP,
            surface_velocity_world_metres_per_second: Vec3::UP * velocity,
            tangential_force_world_newtons: Vec3::ZERO,
        });
        let evaluation = model.evaluate(&state, &input, [0.0; 4])?;
        let force = evaluation.suspension_diagnostics.normal_force_newtons[wheel as usize];
        minimum_force = minimum_force.min(force);
        maximum_force = maximum_force.max(force);
        loop_energy += force * velocity * step;
        samples.push(json!({"time_seconds": time, "deflection_metres": static_deflection + displacement, "normal_force_newtons": force}));
    }
    let expected_energy =
        std::f64::consts::PI * damping * std::f64::consts::TAU * frequency * amplitude * amplitude;
    if (loop_energy - expected_energy).abs() > expected_energy * 1e-8 {
        return Err("Production contact cycle does not close its dissipated-energy balance".into());
    }
    Ok(
        json!({"static_deflection_millimetres": static_deflection * 1000.0,
        "vertical_stiffness_newtons_per_metre": stiffness, "vertical_damping_newton_seconds_per_metre": damping,
        "cycle_dissipated_energy_joules": loop_energy, "minimum_normal_force_newtons": minimum_force,
        "maximum_normal_force_newtons": maximum_force, "samples": samples}),
    )
}

fn measure(configuration: &VehicleConfig) -> Result<Value, String> {
    let mut axles = Vec::new();
    for (name, wheel, load) in [
        ("front", WheelIndex::FrontLeft, 2500.0),
        ("rear", WheelIndex::RearLeft, 3000.0),
    ] {
        let stiffness = configuration
            .coupled_world
            .as_ref()
            .unwrap()
            .suspension
            .tire_vertical_stiffness_newtons_per_metre[wheel as usize];
        let trace = steering_trace(configuration, wheel, load, load / stiffness, 0.02);
        let small_angle = steering_trace(configuration, wheel, load, load / stiffness, 0.001);
        let peak_angle = if wheel.is_front() {
            configuration.tires.front_tire_force.lateral_peak_slip_angle_rad
        } else {
            configuration.tires.rear_tire_force.lateral_peak_slip_angle_rad
        };
        let peak_trace = steering_trace(configuration, wheel, load, load / stiffness, peak_angle);
        axles.push(json!({"axle": name, "normal_force_newtons": load,
            "lateral_stiffness_newtons_per_radian": small_angle[959] / 0.001,
            "step_force_newtons": trace[959], "peak_force_newtons": peak_trace[959],
            "build_to_ninety_percent_seconds": time_to_fraction(&trace, 0.9, false),
            "release_to_ten_percent_seconds": time_to_fraction(&trace, 0.1, true),
            "steering_force_samples_newtons": trace,
            "vertical_cycle": clamped_hub_cycle(configuration, wheel, load)?}));
    }
    Ok(
        json!({"dry_vehicle_mass_kilograms": configuration.complete_dry_vehicle_mass(), "axles": axles}),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() != 3 {
        return Err("Expected reference profile, candidate profile and output report paths".into());
    }
    let reference = VehicleConfig::from_json_path(Path::new(&arguments[0]))?;
    let candidate = VehicleConfig::from_json_path(Path::new(&arguments[1]))?;
    let reference_measurements = measure(&reference)?;
    let candidate_measurements = measure(&candidate)?;
    let mut acceptance = Vec::new();
    for axle in 0..2 {
        let first = &reference_measurements["axles"][axle];
        let second = &candidate_measurements["axles"][axle];
        let value = |entry: &Value, key: &str| entry[key].as_f64().unwrap();
        let vertical_ratio = value(&second["vertical_cycle"], "static_deflection_millimetres")
            / value(&first["vertical_cycle"], "static_deflection_millimetres");
        let stiffness_ratio = value(second, "lateral_stiffness_newtons_per_radian")
            / value(first, "lateral_stiffness_newtons_per_radian");
        let response_ratio = value(second, "build_to_ninety_percent_seconds")
            / value(first, "build_to_ninety_percent_seconds");
        let peak_ratio = value(second, "peak_force_newtons") / value(first, "peak_force_newtons");
        let dissipation_ratio = value(&second["vertical_cycle"], "cycle_dissipated_energy_joules")
            / value(&first["vertical_cycle"], "cycle_dissipated_energy_joules");
        let passed = vertical_ratio > 1.1
            && (0.75..0.99).contains(&stiffness_ratio)
            && response_ratio > 1.15
            && (0.95..1.05).contains(&peak_ratio)
            && dissipation_ratio > 1.0;
        acceptance.push(json!({"axle": second["axle"], "passed": passed, "vertical_deflection_ratio": vertical_ratio,
            "lateral_stiffness_ratio": stiffness_ratio, "response_time_ratio": response_ratio,
            "peak_force_ratio": peak_ratio, "cycle_dissipation_ratio": dissipation_ratio}));
    }
    let passed = acceptance.iter().all(|entry| entry["passed"] == true);
    let report = json!({"reference_profile": arguments[0], "candidate_profile": arguments[1],
        "conditions": {"pressure_gauge_kilopascals":145, "tread_temperature_celsius":95, "carcass_temperature_celsius":80,
            "gas_temperature_celsius":80, "speed_metres_per_second":30, "steering_step_radians":0.02,
            "vertical_irregularity_amplitude_metres":0.002, "vertical_irregularity_frequency_hertz":8,
            "stability_control":"absent from isolated production tire and clamped-hub contact rigs",
            "thermal_evolution":"frozen", "reference_load_mass_kilograms":698},
        "limitations":["Controlled tire and clamped-hub tests; no free chassis yaw or suspension-transient validation", "Candidate coefficients are engineering estimates, not measured manufacturer tire properties", "Reference eighteen-inch profile preserves the historical game construction, not a real 2022 Pirelli tire"],
        "acceptance":acceptance, "passed":passed, "reference":reference_measurements, "candidate":candidate_measurements});
    std::fs::write(&arguments[2], serde_json::to_string_pretty(&report)?)?;
    println!(
        "{}",
        json!({"passed":passed,"acceptance":report["acceptance"],"output":arguments[2]})
    );
    if !passed {
        return Err("Tire construction calibration acceptance failed".into());
    }
    Ok(())
}

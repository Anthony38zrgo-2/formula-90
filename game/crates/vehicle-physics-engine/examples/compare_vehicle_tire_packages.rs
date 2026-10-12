use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};
use serde_json::{json, Value};
use std::path::Path;
use vehicle_physics_engine::coupled_vehicle_world::{CoupledVehicleWorld, TimestampedVehicleInput};
use vehicle_physics_engine::physical_collision_world::{
    PhysicalCollisionWorld, PhysicalWorldShape,
};
use vehicle_physics_engine::{Vec3, VehicleConfig, VehicleInput};

fn comparison_road(irregularity: bool) -> PhysicalCollisionWorld {
    let mut sections = vec![(100.0, 0.0), (-28.0, 0.0)];
    for section in 1..=40 {
        let distance = section as f64 / 40.0;
        let height = if irregularity {
            0.005 * (std::f64::consts::PI * distance).sin().powi(2)
        } else {
            0.0
        };
        sections.push((-28.0 - distance, height));
    }
    sections.push((-300.0, 0.0));
    let vertices: Vec<Point3<f64>> = sections
        .iter()
        .flat_map(|(position, height)| {
            [
                Point3::new(-50.0, *height, *position),
                Point3::new(50.0, *height, *position),
            ]
        })
        .collect();
    let mut triangles = Vec::new();
    for section in 0..sections.len() - 1 {
        let first = (section * 2) as u32;
        triangles.push([first, first + 1, first + 2]);
        triangles.push([first + 1, first + 3, first + 2]);
    }
    let shape = SharedShape::new(TriMesh::new(vertices, triangles).unwrap());
    PhysicalCollisionWorld {
        source_digest: "controlled_comparison_road_generated_by_runner".into(),
        shapes: vec![PhysicalWorldShape {
            identifier: "controlled_comparison_road".into(),
            surface_code: 0,
            driveable: true,
            shape,
            wheel_impact_shape: None,
        }],
    }
}

fn freeze_temperature(world: &mut CoupledVehicleWorld, identifier: u64) {
    let vehicle = world.vehicles.get_mut(&identifier).unwrap();
    for thermal in &mut vehicle.systems.state.tire_thermal.wheels {
        thermal.tread_inner_c = 95.0;
        thermal.tread_center_c = 95.0;
        thermal.tread_outer_c = 95.0;
        thermal.carcass_c = 80.0;
        thermal.gas_c = 80.0;
        thermal.pressure_kpa_gauge = 145.0;
    }
}

fn measure(configuration: &VehicleConfig, irregularity: bool) -> Result<Value, String> {
    let mut configuration = configuration.clone();
    configuration.fuel.current_kg = 0.0;
    configuration.fuel.initial_kg = 0.0;
    let radius = configuration.tires.front_tire_radius;
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(comparison_road(irregularity), settings)?;
    let identifier = world.register_vehicle(
        configuration.clone(),
        Vec3::new(0.0, radius - 0.012, 0.0),
        0.0,
    )?;
    world.enqueue_input(
        identifier,
        TimestampedVehicleInput {
            time_seconds: 0.0,
            input: VehicleInput {
                clutch: 1.0,
                ..VehicleInput::default()
            },
            driving_aids_mask: 0,
        },
    )?;
    for _ in 0..480 {
        freeze_temperature(&mut world, identifier);
        world.advance_host_interval(1.0 / 960.0)?;
    }
    let vehicle = world.vehicles.get_mut(&identifier).unwrap();
    vehicle.state.suspension.generalized_velocity[2] = -30.0;
    vehicle.state.wheel_angular_velocity_radians_per_second = [-30.0 / radius; 4];
    world.enqueue_input(
        identifier,
        TimestampedVehicleInput {
            time_seconds: 1.0,
            input: VehicleInput {
                steering: if irregularity { 0.0 } else { 0.04 },
                clutch: 1.0,
                ..VehicleInput::default()
            },
            driving_aids_mask: 0,
        },
    )?;
    world.enqueue_input(
        identifier,
        TimestampedVehicleInput {
            time_seconds: 1.5,
            input: VehicleInput {
                clutch: 1.0,
                ..VehicleInput::default()
            },
            driving_aids_mask: 0,
        },
    )?;
    let mut samples = Vec::new();
    let mut peak_vertical_acceleration: f64 = 0.0;
    let mut peak_yaw_rate: f64 = 0.0;
    let mut minimum_contact_force = f64::INFINITY;
    for sample in 0..1440 {
        freeze_temperature(&mut world, identifier);
        world.advance_host_interval(1.0 / 960.0)?;
        let vehicle = world.vehicles.get(&identifier).unwrap();
        let evaluation =
            vehicle
                .model
                .evaluate(&vehicle.state, &vehicle.last_force_input, [0.0; 4])?;
        let acceleration = evaluation.suspension_diagnostics.generalized_acceleration[1];
        peak_vertical_acceleration = peak_vertical_acceleration.max(acceleration.abs());
        peak_yaw_rate = peak_yaw_rate.max(vehicle.state.suspension.generalized_velocity[4].abs());
        minimum_contact_force = evaluation
            .suspension_diagnostics
            .normal_force_newtons
            .iter()
            .copied()
            .fold(minimum_contact_force, f64::min);
        if sample % 8 == 0 {
            samples.push(json!({"time_seconds":world.time_seconds, "position_metres":vehicle.state.suspension.body_origin_world_metres,
                "forward_velocity_metres_per_second":vehicle.state.suspension.generalized_velocity[2],
                "yaw_rate_radians_per_second":vehicle.state.suspension.generalized_velocity[4],
                "vertical_acceleration_metres_per_second_squared":acceleration,
                "wheel_travel_metres":vehicle.state.suspension.wheel_travel_metres,
                "normal_forces_newtons":evaluation.suspension_diagnostics.normal_force_newtons,
                "driving_aids_mask":vehicle.driving_aids_mask}));
        }
    }
    Ok(
        json!({"samples":samples,"peak_vertical_acceleration_metres_per_second_squared":peak_vertical_acceleration,
        "peak_yaw_rate_radians_per_second":peak_yaw_rate,"minimum_contact_force_newtons":minimum_contact_force,
        "dry_vehicle_mass_kilograms":configuration.chassis.complete_dry_vehicle_mass(&configuration.tires)}),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() != 3 {
        return Err("Expected reference profile, candidate profile and output report paths".into());
    }
    let reference = VehicleConfig::from_json_path(Path::new(&arguments[0]))?;
    let candidate = VehicleConfig::from_json_path(Path::new(&arguments[1]))?;
    let mut cases = Vec::new();
    for irregularity in [false, true] {
        let scenario = if irregularity {
            "five_millimetre_one_metre_smooth_road_irregularity"
        } else {
            "steering_input_step"
        };
        let first = measure(&reference, irregularity)?;
        let second = measure(&candidate, irregularity)?;
        println!(
            "{}",
            json!({"scenario":scenario,"reference_peak_vertical_acceleration":first["peak_vertical_acceleration_metres_per_second_squared"],
            "candidate_peak_vertical_acceleration":second["peak_vertical_acceleration_metres_per_second_squared"],
            "reference_peak_yaw_rate":first["peak_yaw_rate_radians_per_second"],"candidate_peak_yaw_rate":second["peak_yaw_rate_radians_per_second"]})
        );
        cases.push(json!({"scenario":scenario,"reference":first,"candidate":second}));
    }
    std::fs::write(
        &arguments[2],
        serde_json::to_string_pretty(&json!({"conditions":{"initial_speed_metres_per_second":30,
        "pressure_gauge_kilopascals":145,"tread_temperature_celsius":95,"carcass_temperature_celsius":80,"gas_temperature_celsius":80,
        "fuel_mass_kilograms":0,"driving_aids_mask":0,"coasting_clutch_disengaged":true,"thermal_state_reset_before_each_physical_step":true},
        "limitations":["Actual package masses differ; normal loads and coasting speeds evolve freely", "Synthetic road exercises production world integration without certifying authored track geometry", "Thermal state is reset each step, rather than disabling thermal integration inside the step"],"cases":cases}))?,
    )?;
    Ok(())
}

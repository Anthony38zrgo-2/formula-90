use vehicle_physics_engine::*;

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root.join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    ).unwrap()
}

fn suspension_response(
    configuration: &VehicleConfig,
    backend: SuspensionBackend,
    disturbed_wheel: WheelIndex,
) -> Vec<[f64; 6]> {
    let mut suspension = SuspensionSystem::new(configuration);
    suspension.select_backend(configuration, backend);
    let spawn_height_metres = default_spawn_height(configuration);
    let mut response = Vec::new();
    for tick in 0..160 {
        let samples = std::array::from_fn(|index| {
            let wheel = WheelIndex::ALL[index];
            let anchor = configuration.tires.wheel_anchor_local(
                &configuration.chassis, &configuration.suspension, wheel);
            let road_height_metres = if tick >= 120 && wheel == disturbed_wheel { 0.01 } else { 0.0 };
            let distance_metres = spawn_height_metres + anchor.y - road_height_metres;
            let hit = RaycastHit {
                is_colliding: true,
                distance: distance_metres,
                point: Vec3::new(anchor.x, road_height_metres, anchor.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            };
            TriRaycastSample { inner: hit, center: hit, outer: hit }
        });
        suspension.step(configuration, &samples, 1.0 / 120.0);
        if tick >= 120 {
            for state in &suspension.wheels {
                response.push([
                    state.antiroll_force, state.total_normal_force,
                    state.suspension_compression_m, state.damping_force,
                    suspension.arb_energy_front_j, suspension.arb_energy_rear_j,
                ]);
            }
        }
    }
    response
}

fn set_axle_ratio(configuration: &mut VehicleConfig, wheel: WheelIndex, ratio: f64) {
    if wheel.is_front() {
        configuration.suspension.front_arb_ratio = ratio;
    } else {
        configuration.suspension.rear_arb_ratio = ratio;
    }
}

#[test]
fn simplified_front_and_rear_follow_their_axle_settings() {
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        let mut soft = williams_configuration();
        set_axle_ratio(&mut soft, wheel, 0.0);
        let mut hard = soft.clone();
        set_axle_ratio(&mut hard, wheel, 0.8);
        let soft_response = suspension_response(&soft, SuspensionBackend::Simplified, wheel);
        let hard_response = suspension_response(&hard, SuspensionBackend::Simplified, wheel);
        let energy_index = if wheel.is_front() { 4 } else { 5 };
        let wheel_index = wheel as usize;
        assert!(soft_response.iter().skip(wheel_index).step_by(4)
            .all(|sample| sample[0] == 0.0 && sample[energy_index] == 0.0));
        let maximum_hard_force = hard_response.iter().skip(wheel_index).step_by(4)
            .map(|sample| sample[0].abs()).fold(0.0_f64, f64::max);
        assert!(maximum_hard_force > 10.0, "{wheel:?}: {maximum_hard_force}");
        assert!(hard_response.iter().any(|sample| sample[energy_index] > 0.0));
        assert_ne!(soft_response, hard_response);
    }
}

#[test]
fn standard_front_and_rear_ignore_changes_to_the_legacy_axle_settings() {
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        let mut soft = williams_configuration();
        set_axle_ratio(&mut soft, wheel, 0.0);
        let mut hard = soft.clone();
        set_axle_ratio(&mut hard, wheel, 0.8);
        assert_eq!(
            suspension_response(&soft, SuspensionBackend::Standard, wheel),
            suspension_response(&hard, SuspensionBackend::Standard, wheel),
        );
    }
}

#[test]
fn explicit_physical_bars_keep_their_authored_parameters_in_simplified() {
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        let mut soft = williams_configuration();
        let physical_bar = AntiRollConfig::MotionRatio {
            bar_rate_N_per_m: 25000.0, motion_ratio: 0.7,
        };
        let geometry = soft.suspension.geometric_suspension.as_mut().unwrap();
        if wheel.is_front() { geometry.front_arb = physical_bar; } else { geometry.rear_arb = physical_bar; }
        set_axle_ratio(&mut soft, wheel, 0.0);
        let mut hard = soft.clone();
        set_axle_ratio(&mut hard, wheel, 0.8);
        let soft_response = suspension_response(&soft, SuspensionBackend::Simplified, wheel);
        assert!(soft_response.iter().skip(wheel as usize).step_by(4).any(|sample| sample[0].abs() > 10.0));
        assert_eq!(soft_response, suspension_response(&hard, SuspensionBackend::Simplified, wheel));
    }
}

#[test]
fn simplified_ratio_bars_ignore_the_inactive_geometric_ratio() {
    let configured = williams_configuration();
    let mut changed_geometry = configured.clone();
    let geometry = changed_geometry.suspension.geometric_suspension.as_mut().unwrap();
    geometry.front_arb = AntiRollConfig::LegacyRatio { ratio: 0.99 };
    geometry.rear_arb = AntiRollConfig::LegacyRatio { ratio: 0.99 };
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        assert_eq!(
            suspension_response(&configured, SuspensionBackend::Simplified, wheel),
            suspension_response(&changed_geometry, SuspensionBackend::Simplified, wheel),
        );
    }
}

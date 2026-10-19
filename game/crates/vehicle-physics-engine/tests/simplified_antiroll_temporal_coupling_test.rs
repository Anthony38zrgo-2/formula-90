use vehicle_physics_engine::*;

fn configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .unwrap()
}

fn contact_samples(
    configuration: &VehicleConfig,
    road_heights_metres: [f64; 4],
    airborne_wheel: Option<WheelIndex>,
) -> [TriRaycastSample; 4] {
    let spawn_height = default_spawn_height(configuration);
    std::array::from_fn(|index| {
        let wheel = WheelIndex::ALL[index];
        if airborne_wheel == Some(wheel) {
            return TriRaycastSample::default();
        }
        let anchor = configuration.tires.wheel_anchor_local(
            &configuration.chassis,
            &configuration.suspension,
            wheel,
        );
        let hit = RaycastHit {
            is_colliding: true,
            distance: spawn_height + anchor.y - road_heights_metres[index],
            point: Vec3::new(anchor.x, road_heights_metres[index], anchor.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        TriRaycastSample {
            inner: hit,
            center: hit,
            outer: hit,
        }
    })
}

#[test]
fn symmetric_axle_motion_has_no_bar_force_or_change_in_vertical_response() {
    for frequency_hertz in [120, 240, 480] {
        for rear_axle in [false, true] {
            let active_configuration = configuration();
            let mut disconnected_configuration = active_configuration.clone();
            disconnected_configuration.suspension.front_arb_ratio = 0.0;
            disconnected_configuration.suspension.rear_arb_ratio = 0.0;
            let mut active = SuspensionSystem::new(&active_configuration);
            active.select_backend(&active_configuration, SuspensionBackend::Simplified);
            let mut disconnected = SuspensionSystem::new(&disconnected_configuration);
            disconnected.select_backend(&disconnected_configuration, SuspensionBackend::Simplified);
            let mut maximum_bar_force = 0.0_f64;
            let mut maximum_response_difference = 0.0_f64;
            for tick in 0..frequency_hertz * 2 {
                let mut heights = [0.0; 4];
                if tick >= frequency_hertz && tick < frequency_hertz * 5 / 4 {
                    let first_index = if rear_axle { 2 } else { 0 };
                    heights[first_index] = 0.005;
                    heights[first_index + 1] = 0.005;
                }
                let samples = contact_samples(&active_configuration, heights, None);
                active.step(
                    &active_configuration,
                    &samples,
                    1.0 / frequency_hertz as f64,
                );
                disconnected.step(
                    &disconnected_configuration,
                    &samples,
                    1.0 / frequency_hertz as f64,
                );
                for index in 0..4 {
                    maximum_bar_force =
                        maximum_bar_force.max(active.wheels[index].antiroll_force.abs());
                    maximum_response_difference = maximum_response_difference.max(
                        (active.wheels[index].total_normal_force
                            - disconnected.wheels[index].total_normal_force)
                            .abs(),
                    );
                    assert!(
                        (active.wheels[index].suspension_compression_m
                            - disconnected.wheels[index].suspension_compression_m)
                            .abs()
                            < 1e-9
                    );
                }
            }
            println!("COMMON_AXLE frequency={frequency_hertz} rear={rear_axle} maximum_bar_force_newtons={maximum_bar_force} maximum_normal_force_difference_newtons={maximum_response_difference}");
            assert!(maximum_bar_force < 1e-6);
            assert!(maximum_response_difference < 1e-5);
        }
    }
}

#[test]
fn axle_forces_energy_and_impulses_stay_consistent_through_bumps_and_contact_loss() {
    let selected = configuration();
    let mut suspension = SuspensionSystem::new(&selected);
    suspension.select_backend(&selected, SuspensionBackend::Simplified);
    for tick in 0..360 {
        let mut heights = [0.0; 4];
        if (120..180).contains(&tick) {
            heights[2] = 0.08;
        }
        if (240..300).contains(&tick) {
            heights[1] = 0.03;
        }
        let samples = contact_samples(
            &selected,
            heights,
            if (180..240).contains(&tick) {
                Some(WheelIndex::RearLeft)
            } else {
                None
            },
        );
        let previous_velocities = suspension.wheels.map(|state| state.unsprung_velocity_m_s);
        suspension.step(&selected, &samples, 1.0 / 120.0);
        for index in 0..4 {
            let state = &suspension.wheels[index];
            let mass = WheelMechanicalTuning::for_wheel(&selected, WheelIndex::ALL[index])
                .unsprung_mass_kg
                .max(1.0);
            let momentum = mass * (state.unsprung_velocity_m_s - previous_velocities[index]);
            let impulse = state.normal_impulse_newton_seconds
                - state.suspension_impulse_newton_seconds
                + state.travel_limit_impulse_newton_seconds;
            assert!(
                (momentum - impulse).abs() < 1e-7,
                "tick={tick} wheel={index}: {momentum} versus {impulse}"
            );
        }
        for (left_index, right_index, ratio, energy) in [
            (
                0,
                1,
                selected.suspension.front_arb_ratio,
                suspension.arb_energy_front_j,
            ),
            (
                2,
                3,
                selected.suspension.rear_arb_ratio,
                suspension.arb_energy_rear_j,
            ),
        ] {
            let left = &suspension.wheels[left_index];
            let right = &suspension.wheels[right_index];
            assert_eq!(left.antiroll_force, -right.antiroll_force);
            let prepared = suspension.prepared_geometric().unwrap();
            let stiffness = 0.5
                * (prepared.wheels[left_index].k_wheel_rest_n_per_m
                    + prepared.wheels[right_index].k_wheel_rest_n_per_m)
                * ratio;
            let difference = left.suspension_compression_m - right.suspension_compression_m;
            assert!((left.antiroll_force - stiffness * difference).abs() < 1e-8);
            assert!((energy - 0.5 * stiffness * difference * difference).abs() < 1e-9);
        }
    }
}

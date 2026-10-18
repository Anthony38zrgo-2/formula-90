use vehicle_physics_engine::static_lateral_contact::{
    solve_static_lateral_contact_forces, StaticLateralContact,
};
use vehicle_physics_engine::*;

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .unwrap()
}

fn kinetic_energy(
    mass: f64,
    inertia: Vec3,
    basis: Mat3,
    linear_velocity: Vec3,
    angular_velocity: Vec3,
) -> f64 {
    let angular_velocity_local = basis.inverse_transform_vector(angular_velocity);
    0.5 * (mass * linear_velocity.length_squared()
        + inertia.x * angular_velocity_local.x.powi(2)
        + inertia.y * angular_velocity_local.y.powi(2)
        + inertia.z * angular_velocity_local.z.powi(2))
}

#[test]
fn simultaneous_lateral_contacts_dissipate_energy_with_saturation_and_rotated_inertia() {
    let mass = 612.6;
    let inertia = Vec3::new(396.0, 462.2, 104.9);
    for basis in [Mat3::IDENTITY, Mat3::from_euler_yxz(1.57, 0.07, 0.04)] {
        for force_capacity in [0.1, 10000.0] {
            for blend_weight in [0.25, 1.0] {
                let mut linear_velocity = basis.transform_vector(Vec3::new(0.001, 0.0, 0.0));
                let mut angular_velocity = basis.transform_vector(Vec3::new(0.0, 0.001, -0.0003));
                let previous_energy =
                    kinetic_energy(mass, inertia, basis, linear_velocity, angular_velocity);
                let contacts = std::array::from_fn(|index| {
                    let arm = basis.transform_vector(Vec3::new(
                        if index % 2 == 0 { -0.8 } else { 0.8 },
                        -0.295,
                        if index < 2 { -1.605 } else { 1.285 },
                    ));
                    let direction = basis.transform_vector(Vec3::RIGHT);
                    StaticLateralContact {
                        arm_from_center_world_metres: arm,
                        lateral_direction_world: direction,
                        lateral_velocity_metres_per_second: (linear_velocity
                            + angular_velocity.cross(arm))
                        .dot(direction),
                        force_capacity_newtons: force_capacity,
                        blend_weight,
                    }
                });
                let duration = 1.0 / 120.0;
                let forces =
                    solve_static_lateral_contact_forces(mass, inertia, basis, &contacts, duration);
                for (contact, force) in contacts.iter().zip(forces) {
                    assert!(force.abs() <= force_capacity + 1e-10);
                    let impulse = contact.lateral_direction_world * (force * duration);
                    linear_velocity += impulse / mass;
                    let angular_impulse_local = basis.inverse_transform_vector(
                        contact.arm_from_center_world_metres.cross(impulse),
                    );
                    angular_velocity += basis.transform_vector(Vec3::new(
                        angular_impulse_local.x / inertia.x,
                        angular_impulse_local.y / inertia.y,
                        angular_impulse_local.z / inertia.z,
                    ));
                }
                let next_energy =
                    kinetic_energy(mass, inertia, basis, linear_velocity, angular_velocity);
                assert!(
                    next_energy <= previous_energy + 1e-12,
                    "contact energy grew from {previous_energy} to {next_energy}"
                );
                if blend_weight == 1.0 && force_capacity > 1.0 {
                    for contact in contacts {
                        let residual_velocity = (linear_velocity
                            + angular_velocity.cross(contact.arm_from_center_world_metres))
                        .dot(contact.lateral_direction_world);
                        assert!(
                            residual_velocity.abs() < 1e-8,
                            "uncancelled contact velocity {residual_velocity}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn wheel_linkage_transmits_force_and_moment_for_individual_asymmetric_loads() {
    let configuration = williams_configuration();
    let geometry = configuration
        .suspension
        .geometric_suspension
        .as_ref()
        .unwrap();
    let center = center_of_mass_local(&configuration);
    for wheel in WheelIndex::ALL {
        for travel in [-0.05, 0.0, 0.02] {
            let solution = solve_corner(
                geometry.corners.get(wheel),
                travel,
                0.0,
                wheel.is_front(),
                0.0,
                0.0,
                1.0,
            )
            .unwrap();
            let radius = if wheel.is_front() {
                configuration.tires.front_tire_radius
            } else {
                configuration.tires.rear_tire_radius
            };
            let point = solution.hub - Vec3::UP * radius;
            for force in [Vec3::new(500.0, 1500.0, -1000.0), Vec3::UP * 1500.0] {
                let couple = Vec3::new(1.0, -12.0, 3.0);
                let wrench = geometric_wheel_wrench(
                    geometry,
                    wheel,
                    travel,
                    0.0,
                    force,
                    point,
                    couple,
                    true,
                    0.0,
                    0.0,
                    2000.0,
                    -100.0,
                    50.0,
                    0.5,
                    0.0,
                    Transform3D::IDENTITY,
                    center,
                );
                let expected_moment = (point - center).cross(force) + couple;
                assert!(
                    (wrench.force_world - force).length() < 0.001,
                    "{wheel:?} force {:?}",
                    wrench.force_world - force
                );
                assert!(
                    (wrench.torque_world - expected_moment).length() < 0.001,
                    "{wheel:?} moment {:?}",
                    wrench.torque_world - expected_moment
                );
                assert!(wrench.paths.moment_balance_residual_newton_metres < 0.001);
            }
        }
    }
}

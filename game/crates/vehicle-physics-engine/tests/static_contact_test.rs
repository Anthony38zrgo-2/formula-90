use vehicle_physics_engine::simulation::{
    static_contact_blend, static_contact_lateral_force, STATIC_CONTACT_FADE_SPEED_M_S,
    STATIC_CONTACT_FULL_BLEND_SPEED_M_S, STATIC_CONTACT_FULL_FORWARD_SPEED_M_S,
    STATIC_CONTACT_ZERO_FORWARD_SPEED_M_S,
};
use vehicle_physics_engine::Vec3;

#[test]
fn static_contact_blend_is_full_at_rest_and_gone_at_speed() {
    assert_eq!(static_contact_blend(0.0, 0.0), 1.0);
    assert_eq!(
        static_contact_blend(STATIC_CONTACT_FULL_BLEND_SPEED_M_S * 0.5, 0.0),
        1.0
    );
    assert_eq!(static_contact_blend(STATIC_CONTACT_FADE_SPEED_M_S, 0.0), 0.0);
    assert_eq!(static_contact_blend(-STATIC_CONTACT_FADE_SPEED_M_S * 2.0, 0.0), 0.0);
    let midpoint = 0.5 * (STATIC_CONTACT_FULL_BLEND_SPEED_M_S + STATIC_CONTACT_FADE_SPEED_M_S);
    assert!((static_contact_blend(midpoint, 0.0) - 0.5).abs() < 1e-12);
    assert_eq!(static_contact_blend(f64::NAN, 0.0), 0.0);
}

#[test]
fn static_contact_force_cancels_the_contact_lateral_velocity_in_one_step() {
    let total_mass = 612.6;
    let inertia = Vec3::new(396.0, 462.2, 104.9);
    let arm = Vec3::new(0.0, -0.54, 1.60);
    let contact_right = Vec3::new(1.0, 0.0, 0.0);
    let dt = 1.0 / 120.0;
    let velocity = 0.05;

    let force = static_contact_lateral_force(
        total_mass,
        inertia,
        arm,
        contact_right,
        velocity,
        dt,
    );
    assert!(force.is_finite());
    assert!(force < 0.0);

    let rotational_arm = arm.cross(contact_right);
    let inverse_inertia = Vec3::new(
        1.0 / inertia.x,
        1.0 / inertia.y,
        1.0 / inertia.z,
    );
    let angular_response = Vec3::new(
        inverse_inertia.x * rotational_arm.x,
        inverse_inertia.y * rotational_arm.y,
        inverse_inertia.z * rotational_arm.z,
    );
    let inverse_effective_mass =
        1.0 / total_mass + angular_response.cross(arm).dot(contact_right);
    let cancelled_velocity = force * inverse_effective_mass * dt;
    assert!((cancelled_velocity + velocity).abs() < 1e-9);
}

#[test]
fn static_contact_force_is_zero_without_motion_or_valid_step() {
    let inertia = Vec3::new(396.0, 462.2, 104.9);
    let arm = Vec3::new(0.0, -0.54, 1.60);
    let contact_right = Vec3::new(1.0, 0.0, 0.0);
    assert_eq!(
        static_contact_lateral_force(612.6, inertia, arm, contact_right, 0.0, 1.0 / 120.0),
        0.0
    );
    assert_eq!(
        static_contact_lateral_force(612.6, inertia, arm, contact_right, 0.05, 0.0),
        0.0
    );
    assert_eq!(
        static_contact_lateral_force(612.6, inertia, arm, contact_right, f64::NAN, 1.0 / 120.0),
        0.0
    );
}

#[test]
fn static_contact_does_not_act_while_the_wheel_is_rolling() {
    assert_eq!(static_contact_blend(0.0, STATIC_CONTACT_ZERO_FORWARD_SPEED_M_S * 2.0), 0.0);
    assert_eq!(static_contact_blend(0.0, STATIC_CONTACT_FULL_FORWARD_SPEED_M_S), 1.0);
    assert!((static_contact_blend(0.0, 1.25) - 0.5).abs() < 1e-12);
}
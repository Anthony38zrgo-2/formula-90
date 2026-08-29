use game_sim::DriverInput;
use vehicle_physics_engine::{
    BodyKinematics, Mat3, Transform3D, Vec3, VehicleConfig, default_spawn_height,
};

use formula90_core::underfloor::UnderfloorSample;
use formula90_core::{CoreConfig, CoreFacade};

/// The facade must load the floor/diffuser when the host underfloor probes report
/// no valid contact (valid_mask == 0): a nominal environment derived from the
/// profile's underfloor optimal stance replaces the invalid sample so the rears
/// still get downforce at speed (no top-gear wheelspin).
#[test]
fn facade_aero_fallback_loads_floor_when_probes_invalid() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn_y = default_spawn_height(&cfg);
    let mut facade = CoreFacade::new(CoreConfig::default()).expect("facade must build");
    let id = facade.ensure_spawned().expect("spawn");

    let body = BodyKinematics {
        transform: Transform3D {
            origin: Vec3::new(0.0, spawn_y, 0.0),
            basis: Mat3::IDENTITY,
        },
        orientation: vehicle_physics_engine::Quat::new(0.0, 0.0, 0.0, 1.0),
        linear_velocity: Vec3::new(0.0, 0.0, -60.0),
        angular_velocity: Vec3::ZERO,
    };
    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };
    let samples = facade.flat_samples(id);
    let vi = input.to_vehicle_input();

    // UnderfloorSample::default() is an invalid probe set (valid_mask == 0),
    // which is exactly the case the fallback must handle.
    let mut frame = None;
    for _ in 0..120 {
        frame = Some(facade.step_with_underfloor(
            id,
            body,
            &vi,
            0,
            &samples,
            &UnderfloorSample::default(),
            1.0 / 120.0,
        ));
    }
    let frame = frame.unwrap();
    assert!(
        frame.aero_total_downforce_n > 0.0,
        "invalid underfloor probes must still yield downforce via the nominal fallback (got {})",
        frame.aero_total_downforce_n
    );
}

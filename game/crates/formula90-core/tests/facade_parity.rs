//! Parity + determinism tests for the facade (`formula90_core`).
//!
//! The orchestrator must stay byte-compatible with `game_sim` on the standalone
//! path (same solver, same samples, same snapshot schema) and deterministic.

use std::collections::HashMap;

use game_sim::{DriverInput, World};
use vehicle_physics_engine::{
    BodyKinematics, Mat3, Transform3D, Vec3, VehicleConfig, VehicleInput, default_spawn_height,
};

use formula90_core::{CoreConfig, CoreFacade};

/// TC bit (matches `AidsMask` bit layout; see physics simulation.rs).
const BIT_TC: u32 = 1 << 1;
const BIT_STABILITY: u32 = 1 << 2;

const DT: f64 = 1.0 / 120.0;

fn build_facade(modules: Vec<String>) -> CoreFacade {
    let mut cfg = CoreConfig::default();
    cfg.modules = modules;
    CoreFacade::new(cfg).expect("facade must build")
}

/// The facade's standalone path and a pure `game_sim` world, given the same input
/// and scene references, must emit BYTE-IDENTICAL core snapshots.
#[test]
fn facade_standalone_matches_game_sim_byte_exactly() {
    let steps = 120usize;

    let mut facade = build_facade(Vec::new());
    let id = facade.ensure_spawned().expect("spawn");
    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };
    for _ in 0..steps {
        let samples = facade.flat_samples(id);
        facade.step_standalone(id, &input, &samples, DT);
    }

    let mut sim_world = World::new(DT);
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = Transform3D {
        origin: Vec3::new(0.0, default_spawn_height(&cfg), 0.0),
        basis: Mat3::IDENTITY,
    };
    let sim_id = sim_world.spawn_vehicle(
        cfg,
        spawn,
        facade.config().vehicle_scene.clone(),
        facade.config().track_scene.clone(),
    );
    let mut inputs = HashMap::new();
    inputs.insert(sim_id, input);
    for _ in 0..steps {
        sim_world.step(&inputs);
    }

    assert_eq!(
        facade.core_snapshot().to_bytes().expect("serialize facade core"),
        sim_world.snapshot().to_bytes().expect("serialize sim core"),
        "facade standalone snapshot must equal game_sim snapshot (same solver path)"
    );
}

/// Running the orchestrator twice with identical args yields byte-identical
/// orchestrated snapshots (including module contributions).
#[test]
fn facade_snapshot_is_deterministic() {
    let steps = 60usize;
    let modules = vec!["weather".to_string(), "ai".to_string()];

    let run = || -> Vec<u8> {
        let mut facade = build_facade(modules.clone());
        let id = facade.ensure_spawned().expect("spawn");
        let input = DriverInput {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..steps {
            let samples = facade.flat_samples(id);
            facade.step_standalone(id, &input, &samples, DT);
        }
        facade.facade_snapshot()
            .to_bytes()
            .expect("serialize facade snapshot")
    };

    assert_eq!(run(), run(), "facade snapshots must be byte-identical across identical runs");
}

/// `--parity-sim` equivalent: with modules the INNER core snapshot still matches a
/// pure game_sim run (modules must not mutate physics state).
#[test]
fn modules_do_not_perturb_physics_parity() {
    let steps = 90usize;
    let mods = vec!["weather".to_string(), "ai".to_string()];

    let mut facade = build_facade(mods);
    let id = facade.ensure_spawned().expect("spawn");
    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };
    for _ in 0..steps {
        let samples = facade.flat_samples(id);
        facade.step_standalone(id, &input, &samples, DT);
    }

    let mut sim_world = World::new(DT);
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = Transform3D {
        origin: Vec3::new(0.0, default_spawn_height(&cfg), 0.0),
        basis: Mat3::IDENTITY,
    };
    let sim_id = sim_world.spawn_vehicle(
        cfg,
        spawn,
        facade.config().vehicle_scene.clone(),
        facade.config().track_scene.clone(),
    );
    let mut inputs = HashMap::new();
    inputs.insert(sim_id, input);
    for _ in 0..steps {
        sim_world.step(&inputs);
    }

    assert_eq!(
        facade.core_snapshot().to_bytes().unwrap(),
        sim_world.snapshot().to_bytes().unwrap(),
        "registered modules must not alter the physics snapshot"
    );
}

/// The FORCE path (`step`, used in-engine) must publish telemetry into the frame
/// and produce physical output — guards the `ent.last` publishing bug found via the
/// E2E ABI handshake test.
#[test]
fn force_path_publishes_telemetry() {
    let mut facade = build_facade(Vec::new());
    let id = facade.ensure_spawned().expect("spawn");

    let cfg = VehicleConfig::f1_94_canonical();
    let y = default_spawn_height(&cfg);
    let body = BodyKinematics {
        transform: Transform3D::new(Vec3::new(0.0, y, 0.0), Mat3::IDENTITY),
        orientation: vehicle_physics_engine::Quat::new(0.0, 0.0, 0.0, 1.0),
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
    };
    let input = VehicleInput {
        throttle: 1.0,
        gear_request: Some(1),
        ..Default::default()
    };

    for _ in 0..120 {
        let samples = facade.flat_samples(id);
        facade.step(id, body, &input, BIT_TC, &samples, DT);
    }

    let f = facade.latest_frame();
    assert!(f.rpm > 1000.0, "force path must publish engine rpm, got {}", f.rpm);
    assert!(
        f.force.iter().all(|v| v.is_finite()),
        "force path force must be finite"
    );
    assert!(f.force[2] < 0.0, "full-throttle gear-1 on flat ground pushes forward (-z), got {}", f.force[2]);
}

/// P0: the facade must apply the FULL aids mask every step (no frozen defaults,
/// no first-frame toggle) and react immediately to mask changes.
#[test]
fn facade_applies_full_aids_mask_each_step() {
    let mut facade = build_facade(Vec::new());
    let id = facade.ensure_spawned().expect("spawn");
    let cfg = VehicleConfig::f1_94_canonical();
    let y = default_spawn_height(&cfg);
    let body = BodyKinematics {
        transform: Transform3D::new(Vec3::new(0.0, y, 0.0), Mat3::IDENTITY),
        orientation: vehicle_physics_engine::Quat::new(0.0, 0.0, 0.0, 1.0),
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
    };
    let input = VehicleInput {
        throttle: 1.0,
        gear_request: Some(1),
        ..Default::default()
    };

    // TC + stability ON.
    let mask_on = BIT_TC | BIT_STABILITY;
    for _ in 0..10 {
        let samples = facade.flat_samples(id);
        facade.step(id, body, &input, mask_on, &samples, DT);
    }
    assert!(facade.latest_frame().tc_active, "TC bit must reach the solver (ON)");

    // TC OFF -> must flip immediately on the next steps (no frozen default).
    let mask_off = BIT_STABILITY;
    for _ in 0..10 {
        let samples = facade.flat_samples(id);
        facade.step(id, body, &input, mask_off, &samples, DT);
    }
    assert!(
        !facade.latest_frame().tc_active,
        "TC must be OFF after the mask changes (no first-frame toggle / frozen default)"
    );
}

/// P0: `apply_runtime_config` reaches the facade's solver (tuning panel / setters
/// keep working in bridge_controlled mode).
#[test]
fn facade_apply_runtime_config_is_accepted() {
    let mut facade = build_facade(Vec::new());
    let id = facade.ensure_spawned().expect("spawn");

    let mut cfg = unsafe { std::mem::zeroed::<vehicle_physics_engine::FfiRuntimeConfig>() };
    cfg.vehicle_mass = 505.0;
    cfg.front_brake_bias = 0.62;
    cfg.diff_preload = 123.0;
    cfg.aids_enabled_mask = BIT_STABILITY;

    assert!(
        facade.apply_runtime_config(id, &cfg),
        "apply_runtime_config must succeed"
    );
    // The entity still steps fine afterwards.
    let input = VehicleInput {
        throttle: 0.0,
        ..Default::default()
    };
    let samples = facade.flat_samples(id);
    let f = facade.step(id, body_at(&facade, id), &input, BIT_STABILITY, &samples, DT);
    assert!(f.rpm.is_finite());
}

fn body_at(_facade: &CoreFacade, _id: u32) -> BodyKinematics {
    let cfg = VehicleConfig::f1_94_canonical();
    let y = default_spawn_height(&cfg);
    BodyKinematics {
        transform: Transform3D::new(Vec3::new(0.0, y, 0.0), Mat3::IDENTITY),
        orientation: vehicle_physics_engine::Quat::new(0.0, 0.0, 0.0, 1.0),
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
    }
}

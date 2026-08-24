use std::collections::HashMap;
use std::env;
use std::path::Path;

use vehicle_physics_engine::{default_spawn_height, Mat3, Transform3D, Vec3, VehicleConfig};

use game_sim::{DriverInput, EntityId, World};

/// Headless runner for the authoritative core. Exercises the full snapshot-server
/// path without Godot: loads a vehicle config, spawns it, runs a fixed-timestep
/// launch, prints telemetry, and writes the final `Snapshot` as a bincode blob
/// (the same bytes a mirror would receive). Re-running with identical args yields
/// an identical snapshot checksum -> deterministic, headless, decoupled.
fn main() {
    let args: Vec<String> = env::args().collect();
    let mut config_path: Option<String> = None;
    let mut positional: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" && i + 1 < args.len() {
            config_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            positional.push(args[i].clone());
            i += 1;
        }
    }

    let duration: f64 = positional
        .get(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3.0);
    let hz: f64 = positional
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(120.0);
    let dt = 1.0 / hz.max(1.0);
    let total_steps = (duration / dt) as usize;

    let mut world = World::new(dt);
    let cfg = match &config_path {
        Some(p) => VehicleConfig::from_json_path(Path::new(p)).expect("Failed to load JSON config"),
        None => VehicleConfig::f1_94_canonical(),
    };
    let spawn = Transform3D {
        origin: Vec3::new(0.0, default_spawn_height(&cfg), 0.0),
        basis: Mat3::IDENTITY,
    };
    let id: EntityId = world.spawn_vehicle(
        cfg,
        spawn,
        "res://scenes/vehicles/f1_94/f1_94_rust.tscn".to_string(),
        "res://scenes/tracks/test_field/la_chutana_track.tscn".to_string(),
    );

    // Scripted input: full throttle, no steering/brake.
    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };

    println!(
        "=== game_sim headless === config={:?} dur={}s hz={} steps={}",
        config_path, duration, hz, total_steps
    );
    let print_every = ((hz / 2.0).round() as usize).max(1);
    let mut last_snapshot: Option<Vec<u8>> = None;
    for step in 0..total_steps {
        let mut inputs = HashMap::new();
        inputs.insert(id, input);
        world.step(&inputs);
        let snap = world.snapshot();
        let bytes = snap.to_bytes().expect("serialize snapshot");
        last_snapshot = Some(bytes);
        if step % print_every == 0 {
            if let Some(t) = world.entity_telemetry(id) {
                println!(
                    "T={:5.2}s v={:6.1}km/h rpm={:5.0} gear={} FL={:4.1} RL={:4.1} RSlip={:.3} TC={}",
                    world.time, t.speed_kmh, t.rpm, t.gear, t.fl_comp_mm, t.rl_comp_mm, t.rear_slip, t.tc_active
                );
            }
        }
    }

    if let Some(bytes) = &last_snapshot {
        let out = Path::new("reports/game_sim_snapshot.bin");
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(out, bytes).expect("write snapshot");
        println!(
            "Snapshot ({} bytes) -> {:?}; checksum=0x{:08X}",
            bytes.len(),
            out,
            checksum(bytes)
        );
    }
    println!("Re-run with identical args -> identical checksum (deterministic headless core).");
}

fn checksum(b: &[u8]) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for &byte in b {
        h ^= byte as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

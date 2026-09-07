//! Headless runner for the facade orchestrator (`formula90_core`).
//!
//! Exercises the EXACT same orchestrator the in-engine `F90Core` node drives, but
//! without Godot: loads a vehicle config, spawns it, runs a fixed-timestep launch,
//! prints telemetry + audio readouts, and writes the final `FacadeSnapshot` as a
//! bincode blob. `--parity-sim` also runs a pure `game_sim` world and asserts the
//! inner core snapshots are byte-identical (standalone path parity).

use std::env;
use std::path::PathBuf;

use game_sim::DriverInput;
use game_sim::World;
use vehicle_physics_engine::{default_spawn_height, Mat3, Transform3D, Vec3, VehicleConfig};

use formula90_core::{CoreConfig, CoreFacade};

fn checksum(b: &[u8]) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for &byte in b {
        h ^= byte as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn usage() {
    eprintln!(
        "core_cli [--config PATH] [--bank DIR] [--modules a,b] [--parity-sim] [duration] [hz]"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut config_path: Option<String> = None;
    let mut bank_dir: Option<String> = None;
    let mut modules: Vec<String> = Vec::new();
    let mut parity_sim = false;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" if i + 1 < args.len() => {
                config_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--bank" if i + 1 < args.len() => {
                bank_dir = Some(args[i + 1].clone());
                i += 2;
            }
            "--modules" if i + 1 < args.len() => {
                modules = args[i + 1]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
            }
            "--parity-sim" => {
                parity_sim = true;
                i += 1;
            }
            "--help" | "-h" => {
                usage();
                return;
            }
            _ => {
                positional.push(args[i].clone());
                i += 1;
            }
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

    let mut cfg = CoreConfig::default();
    cfg.use_canonical = config_path.is_none();
    cfg.config_json_path = config_path.as_ref().map(PathBuf::from);
    cfg.bank_dir = bank_dir.as_ref().map(PathBuf::from);
    cfg.enable_audio = cfg.bank_dir.is_some();
    cfg.modules = modules;

    let mut facade = CoreFacade::new(cfg.clone()).expect("failed to build facade");
    let id = facade
        .ensure_spawned()
        .expect("failed to spawn primary entity");

    println!(
        "=== formula90_core headless === modules={:?} audio_healthy={} config={:?} dur={}s hz={} steps={}",
        facade.module_names(),
        facade.audio_healthy(),
        config_path,
        duration,
        hz,
        total_steps
    );

    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };

    let print_every = ((hz / 2.0).round() as usize).max(1);
    let mut last: Option<Vec<u8>> = None;
    for step in 0..total_steps {
        let samples = facade.flat_samples(id);
        // Clone the frame so `facade` is free again for readouts/snapshot below.
        let frame = facade.step_standalone(id, &input, &samples, dt).clone();
        let snap = facade.facade_snapshot();
        last = Some(snap.to_bytes().expect("serialize facade snapshot"));
        if step % print_every == 0 {
            println!(
                "T={:5.2}s v={:6.1}km/h rpm={:5.0} gear={} FL={:4.1} RL={:4.1} RSlip={:.3} TC={} surf={} nrm={:.2}",
                facade.world_time(),
                frame.speed_kmh,
                frame.rpm,
                frame.gear,
                frame.fl_comp_mm,
                frame.rl_comp_mm,
                frame.rear_slip,
                frame.tc_active,
                frame.audio.surface_code,
                frame.audio.last_norm,
            );
        }
    }

    // Parity: a pure game_sim world over the same script must yield byte-identical
    // inner core snapshots (standalone path).
    if parity_sim {
        let mut sim_world = World::new(dt);
        let sim_cfg = match &config_path {
            Some(p) => VehicleConfig::from_json_path(std::path::Path::new(p))
                .expect("failed to load JSON config for parity sim"),
			None => VehicleConfig::f1_2026_2008_canonical(),
        };
        let spawn = Transform3D {
            origin: Vec3::new(0.0, default_spawn_height(&sim_cfg), 0.0),
            basis: Mat3::IDENTITY,
        };
        let sim_id = sim_world.spawn_vehicle(
            sim_cfg,
            spawn,
            cfg.vehicle_scene.clone(),
            cfg.track_scene.clone(),
        );
        let mut inputs = std::collections::HashMap::new();
        inputs.insert(sim_id, input);
        let mut sim_last: Option<Vec<u8>> = None;
        for _ in 0..total_steps {
            sim_world.step(&inputs);
            sim_last = Some(
                sim_world
                    .snapshot()
                    .to_bytes()
                    .expect("serialize sim snapshot"),
            );
        }
        let fac_core = facade
            .core_snapshot()
            .to_bytes()
            .expect("serialize facade core");
        match sim_last {
            Some(sim) if sim == fac_core => {
                println!(
                    "PARITY OK: facade core snapshot == game_sim snapshot ({} bytes, checksum=0x{:08X})",
                    fac_core.len(),
                    checksum(&fac_core)
                );
            }
            Some(sim) => {
                eprintln!(
                    "PARITY FAIL: facade={} bytes (0x{:08X}) != game_sim={} bytes (0x{:08X})",
                    fac_core.len(),
                    checksum(&fac_core),
                    sim.len(),
                    checksum(&sim)
                );
                std::process::exit(2);
            }
            None => {
                eprintln!("PARITY FAIL: game_sim produced no snapshot");
                std::process::exit(2);
            }
        }
    }

    if let Some(bytes) = &last {
        let out = std::path::Path::new("reports/facade_snapshot.bin");
        if let Some(parent) = out.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(out, bytes).expect("write facade snapshot");
        println!(
            "FacadeSnapshot ({} bytes) -> {:?}; checksum=0x{:08X}",
            bytes.len(),
            out,
            checksum(bytes)
        );
    }
    println!("Re-run with identical args -> identical checksum (deterministic orchestrator).");
}

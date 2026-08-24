use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use vehicle_physics_engine::*;

fn flat_ground_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = [TriRaycastSample::default(); 4];
    for (sample, &wheel) in samples.iter_mut().zip(WheelIndex::ALL.iter()) {
        let hub_local = sim.config.wheel_anchor_local(wheel);
        let hub_world = sim.state.transform.transform_point(hub_local);

        let tire_w = if wheel.is_front() {
            sim.config.front_tire_width
        } else {
            sim.config.rear_tire_width
        };
        let span = tire_w * sim.config.tri_ray_spacing_ratio;

        let left_offset = sim
            .state
            .transform
            .basis
            .transform_vector(Vec3::new(-span, 0.0, 0.0));
        let right_offset = sim
            .state
            .transform
            .basis
            .transform_vector(Vec3::new(span, 0.0, 0.0));

        let p_in = hub_world + left_offset;
        let p_mid = hub_world;
        let p_out = hub_world + right_offset;

        *sample = TriRaycastSample {
            inner: RaycastHit {
                is_colliding: true,
                distance: p_in.y.max(0.0),
                point: Vec3::new(p_in.x, 0.0, p_in.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            center: RaycastHit {
                is_colliding: true,
                distance: p_mid.y.max(0.0),
                point: Vec3::new(p_mid.x, 0.0, p_mid.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            outer: RaycastHit {
                is_colliding: true,
                distance: p_out.y.max(0.0),
                point: Vec3::new(p_out.x, 0.0, p_out.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
        };
    }
    samples
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut config_path: Option<String> = None;
    let mut mode: String = "launch".to_string();
    let mut positional: Vec<&String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" && i + 1 < args.len() {
            config_path = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--mode" && i + 1 < args.len() {
            mode = args[i + 1].clone();
            i += 2;
        } else {
            positional.push(&args[i]);
            i += 1;
        }
    }

    let duration_sec: f64 = if positional.len() > 0 {
        positional[0].parse().unwrap_or(5.0)
    } else {
        5.0
    };
    let output_path = if positional.len() > 1 {
        Some(positional[1].as_str())
    } else {
        None
    };

    println!("=== Formula-90 Vehicle Physics CLI ===");

    let cfg = if let Some(path) = &config_path {
        println!("Loading config from: {}", path);
        VehicleConfig::from_json_path(Path::new(path)).expect("Failed to load JSON config")
    } else {
        println!("No --config supplied; using f1_94_canonical() defaults.");
        VehicleConfig::f1_94_canonical()
    };

    println!(
        "TC default enabled: {} | max_clutch_torque_ratio: {} | max_torque: {} N·m",
        cfg.aids.traction_control_default_enabled, cfg.max_clutch_torque_ratio, cfg.max_torque
    );

    let mode_desc = match mode.as_str() {
        "static" => "static rest (no throttle / no steering)",
        "slalom" => "slalom (throttle 0.5, steering sine 0.8 Hz)",
        _ => "full throttle straight acceleration",
    };
    println!("Simulating {} seconds of {}...", duration_sec, mode_desc);

    let spawn_height = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0);

    let phys_hz: f64 = if positional.len() > 2 {
        positional[2].parse().unwrap_or(120.0)
    } else {
        120.0
    };
    let dt = 1.0 / phys_hz.max(1.0);
    let total_steps = (duration_sec / dt) as usize;
    let mut lines = Vec::new();
    lines.push(TelemetryFrame::CSV_HEADER.join(","));

    for step in 0..total_steps {
        let t = sim.state.sim_time;
        let (thr, steer) = match mode.as_str() {
            "static" => (0.0_f64, 0.0_f64),
            "slalom" => (
                0.5_f64,
                0.5_f64 * (2.0 * std::f64::consts::PI * 0.8 * t).sin(),
            ),
            _ => (1.0_f64, 0.0_f64),
        };
        let input = VehicleInput {
            throttle: thr,
            steering: steer,
            brake: 0.0,
            handbrake: 0.0,
            clutch: 0.0,
            gear_request: None,
        };

        let samples = flat_ground_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        lines.push(telem.to_csv_line());

        if step % 60 == 0 || step == total_steps - 1 {
            println!(
                "T={:5.2}s | Speed={:6.1} km/h | RPM={:5.0} | Gear={} | FL_Comp={:4.1}mm | LongG={:+5.2}",
                sim.state.sim_time, telem.speed_kmh, telem.rpm, telem.gear, telem.fl_comp_mm, telem.long_g
            );
        }
    }

    if let Some(path) = output_path {
        if config_path.as_deref() == Some(path) {
            eprintln!(
                "ERROR: refusing to overwrite the config file '{}' with telemetry output.",
                path
            );
        } else {
            let mut f = File::create(path).expect("Unable to create telemetry CSV file");
            for line in lines {
                writeln!(f, "{}", line).unwrap();
            }
            println!("Telemetry saved to '{}'", path);
        }
    }
}

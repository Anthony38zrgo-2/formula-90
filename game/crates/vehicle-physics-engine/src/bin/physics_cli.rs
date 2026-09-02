// BASE-000 deterministic calibration harness for the vehicle physics engine.
// Runs scripted, open-loop driver inputs on flat ground (optionally with a curb
// section) so results can be diffed between engine revisions. The `--raw` flag
// disables TC/stability/steering-slip/countersteer/brake-assist so baselines
// capture the pure mechanical model.
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use vehicle_physics_engine::*;

fn flat_ground_samples(sim: &VehicleSimulator, wheel_lift: [f64; 4]) -> [TriRaycastSample; 4] {
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

        let lift = wheel_lift[wheel as usize];
        let p_in = hub_world + left_offset;
        let p_mid = hub_world;
        let p_out = hub_world + right_offset;

        *sample = TriRaycastSample {
            inner: RaycastHit {
                is_colliding: true,
                distance: (p_in.y - lift).max(0.0),
                point: Vec3::new(p_in.x, lift, p_in.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            center: RaycastHit {
                is_colliding: true,
                distance: (p_mid.y - lift).max(0.0),
                point: Vec3::new(p_mid.x, lift, p_mid.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            outer: RaycastHit {
                is_colliding: true,
                distance: (p_out.y - lift).max(0.0),
                point: Vec3::new(p_out.x, lift, p_out.z),
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
    let mut raw_aids = false;
    let mut init_speed_ms: Option<f64> = None;
    let mut steer_amp: f64 = 0.25;
    let mut ramp_sec: f64 = 8.0;
    let mut step_sec: f64 = 2.0;
    let mut lift_ratio: f64 = 0.3;
    let mut curb_height_m: f64 = 0.04;
    let mut positional: Vec<&String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" && i + 1 < args.len() {
            config_path = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--mode" && i + 1 < args.len() {
            mode = args[i + 1].clone();
            i += 2;
        } else if args[i] == "--raw" {
            raw_aids = true;
            i += 1;
        } else if args[i] == "--init-speed-ms" && i + 1 < args.len() {
            init_speed_ms = args[i + 1].parse().ok();
            i += 2;
        } else if args[i] == "--steer-amp" && i + 1 < args.len() {
            steer_amp = args[i + 1].parse().unwrap_or(0.25);
            i += 2;
        } else if args[i] == "--ramp-sec" && i + 1 < args.len() {
            ramp_sec = args[i + 1].parse().unwrap_or(8.0);
            i += 2;
        } else if args[i] == "--step-sec" && i + 1 < args.len() {
            step_sec = args[i + 1].parse().unwrap_or(2.0);
            i += 2;
        } else if args[i] == "--lift-ratio" && i + 1 < args.len() {
            lift_ratio = args[i + 1].parse().unwrap_or(0.3);
            i += 2;
        } else if args[i] == "--curb-height-m" && i + 1 < args.len() {
            curb_height_m = args[i + 1].parse().unwrap_or(0.04);
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
    let phys_hz: f64 = if positional.len() > 2 {
        positional[2].parse().unwrap_or(120.0)
    } else {
        120.0
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

    let raw_aids_note = if raw_aids {
        " (all driving aids OFF except auto-clutch: raw mechanical baseline)"
    } else {
        ""
    };
    let mode_desc = match mode.as_str() {
        "static" => "static rest (no throttle / no steering)",
        "slalom" => "slalom (throttle 0.5, steering sine 0.8 Hz)",
        "coast_down" => "coast-down from --init-speed-ms (throttle 0 / brake 0)",
        "brake_100" => "100 km/h full-brake stop (throttle 0 / brake 1.0)",
        "skidpad" => "skidpad (throttle 0.7, steering ramp to --steer-amp over --ramp-sec)",
        "step_steer" => "step steer (throttle 0.6, steering step to --steer-amp at --step-sec)",
        "lift_off" => "lift-off rotation (throttle 1.0 then cut at --lift-ratio, steering --steer-amp)",
        "power_on_exit" => "power-on exit (throttle 1.0, constant steering --steer-amp)",
        "curb" => "curb traversal (throttle 0.6, right wheels raised --curb-height-m at 35-55% of run)",
        _ => "full throttle straight acceleration",
    };
    println!("Simulating {} seconds of {}{}...", duration_sec, mode_desc, raw_aids_note);

    let spawn_height = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0);

    if raw_aids {
        sim.aids = AidsMask {
            abs: false,
            traction_control: false,
            stability: false,
            steering_slip_assist: false,
            countersteer: false,
            auto_clutch: true,
            launch_control: false,
            brake_assist: false,
        };
    }

    let init_speed = init_speed_ms.unwrap_or_else(|| match mode.as_str() {
        "coast_down" | "brake_100" => 27.8,
        _ => 0.0,
    });

    if init_speed > 0.0 {
        sim.state.linear_velocity = Vec3::new(0.0, 0.0, -init_speed);
        for wheel in WheelIndex::ALL {
            let radius = sim.state.tires.wheels[wheel as usize]
                .effective_rolling_radius
                .max(0.05);
            sim.state.tires.wheels[wheel as usize].spin = init_speed / radius;
        }
        println!("Initial speed: {:.1} m/s ({:.1} km/h)", init_speed, init_speed * 3.6);
    }

    let dt = 1.0 / phys_hz.max(1.0);
    let total_steps = (duration_sec / dt) as usize;
    let mut lines = Vec::new();
    lines.push(TelemetryFrame::CSV_HEADER.join(","));

    let curb_seconds = [duration_sec * 0.35, duration_sec * 0.55];

    for step in 0..total_steps {
        let t = sim.state.sim_time;
        let (thr, steer, brake, wheel_lift) = match mode.as_str() {
            "static" => (0.0_f64, 0.0_f64, 0.0_f64, [0.0; 4]),
            "coast_down" => (0.0_f64, 0.0_f64, 0.0_f64, [0.0; 4]),
            "brake_100" => (0.0_f64, 0.0_f64, 1.0_f64, [0.0; 4]),
            "slalom" => (
                0.5_f64,
                0.5_f64 * (2.0 * std::f64::consts::PI * 0.8 * t).sin(),
                0.0_f64,
                [0.0; 4],
            ),
            "skidpad" => (
                0.7_f64,
                steer_amp * (t / ramp_sec).min(1.0),
                0.0_f64,
                [0.0; 4],
            ),
            "step_steer" => (
                0.6_f64,
                if t >= step_sec { steer_amp } else { 0.0 },
                0.0_f64,
                [0.0; 4],
            ),
            "lift_off" => (
                if t < duration_sec * lift_ratio { 1.0 } else { 0.0 },
                steer_amp,
                0.0_f64,
                [0.0; 4],
            ),
            "power_on_exit" => (1.0_f64, steer_amp, 0.0_f64, [0.0; 4]),
            "curb" => {
                let on = t >= curb_seconds[0] && t <= curb_seconds[1];
                (
                    0.6_f64,
                    0.0_f64,
                    0.0_f64,
                    if on {
                        [0.0, curb_height_m, 0.0, curb_height_m]
                    } else {
                        [0.0; 4]
                    },
                )
            }
            _ => (1.0_f64, 0.0_f64, 0.0_f64, [0.0; 4]),
        };
        let input = VehicleInput {
            throttle: thr,
            steering: steer,
            brake,
            handbrake: 0.0,
            clutch: 0.0,
            gear_request: None,
        };

        let samples = flat_ground_samples(&sim, wheel_lift);
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

            let (branch, commit) = git_head_info();
            let metadata = serde_json::json!({
                "tool": "physics_cli",
                "git_branch": branch,
                "git_commit": commit,
                "config_path": config_path.clone().unwrap_or_else(|| "f1_94_canonical()".to_string()),
                "config_fnv64_hash": config_path.as_deref().map(|p| file_fnv64(Path::new(p))).unwrap_or_else(|| "builtin-default".to_string()),
                "godot_version": godot_project_version(),
                "mode": mode,
                "raw_aids": raw_aids,
                "physics_hz": phys_hz,
                "duration_sec": duration_sec,
                "step_count": total_steps,
                "init_speed_ms": init_speed,
                "steer_amp": steer_amp,
                "ramp_sec": ramp_sec,
                "step_sec": step_sec,
                "lift_ratio": lift_ratio,
                "curb_height_m": curb_height_m,
                "test_scene": "flat_ground_scripted",
            });
            if let Err(err) = write_meta_json(Path::new(path), metadata) {
                eprintln!("WARNING: could not write metadata: {err}");
            }
        }
    }
}

// BASE-000 tire sweep: evaluates a single wheel over pure-slip and combined-slip
// grids (Fz x slip angle x slip ratio) without Godot integration. Runs the same
// transient solver as the live sim to steady state, producing load-scaled tire
// curves for future-parts calibration.
//
// Outputs:
//   <out>/tire_sweep_lateral.csv
//   <out>/tire_sweep_longitudinal.csv
//   <out>/tire_sweep_combined.csv
//   <out>/tire_sweep_meta.json
use std::fs;
use std::path::PathBuf;
use vehicle_physics_engine::*;

#[derive(Clone, Copy)]
enum SweepMode {
    Lateral,
    Longitudinal,
    Combined,
}

impl SweepMode {
    fn all() -> [SweepMode; 3] {
        [SweepMode::Lateral, SweepMode::Longitudinal, SweepMode::Combined]
    }
    fn name(&self) -> &'static str {
        match self {
            SweepMode::Lateral => "lateral",
            SweepMode::Longitudinal => "longitudinal",
            SweepMode::Combined => "combined",
        }
    }
}

const LOAD_MULTIPLES: [f64; 4] = [0.5, 1.0, 1.5, 2.0];
const ALPHA_GRID: [f64; 15] = [
    -0.35, -0.30, -0.25, -0.20, -0.15, -0.10, -0.05, 0.00, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30,
    0.35,
];
const KAPPA_GRID: [f64; 19] = [
    -0.90, -0.80, -0.70, -0.60, -0.50, -0.40, -0.30, -0.20, -0.10, 0.00, 0.10, 0.20, 0.30, 0.40,
    0.50, 0.60, 0.70, 0.80, 0.90,
];

const CSV_HEADER: &str = "Wheel,Fz_N,SlipAngle_Rad,SlipRatio,LateralForce_N,LongitudinalForce_N,AlignTorque_Nm,RollingResistance_N,EffSlipAngle_Rad,EffSlipRatio";

fn wheel_name(wheel: WheelIndex) -> &'static str {
    match wheel {
        WheelIndex::FrontLeft => "FL",
        WheelIndex::FrontRight => "FR",
        WheelIndex::RearLeft => "RL",
        WheelIndex::RearRight => "RR",
    }
}

fn row(wheel: WheelIndex, fz: f64, p: &TireForcePoint) -> String {
    format!(
        "{},{:.3},{:.5},{:.5},{:.3},{:.3},{:.3},{:.3},{:.5},{:.5}",
        wheel_name(wheel),
        fz,
        p.slip_angle_rad,
        p.slip_ratio,
        p.lateral_force_n,
        p.longitudinal_force_n,
        p.aligning_torque_nm,
        p.rolling_resistance_n,
        p.effective_slip_angle_rad,
        p.effective_slip_ratio,
    )
}

fn default_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("vehicles")
        .join("f1_2026_2008")
        .join("f1_2026_2008_physics.json")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path: PathBuf = default_config_path();
    let mut out_dir = PathBuf::from("implementation/calibration");
    let mut mode_arg = "all".to_string();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" && i + 1 < args.len() {
            config_path = PathBuf::from(&args[i + 1]);
            i += 2;
        } else if args[i] == "--out" && i + 1 < args.len() {
            out_dir = PathBuf::from(&args[i + 1]);
            i += 2;
        } else if args[i] == "--mode" && i + 1 < args.len() {
            mode_arg = args[i + 1].clone();
            i += 2;
        } else if args[i] == "--help" || args[i] == "-h" {
            println!(
                "Usage: tire_sweep [--config <path>] [--out <dir>] [--mode lateral|longitudinal|combined|all]"
            );
            return;
        } else {
            eprintln!("Unknown argument: {}", args[i]);
            std::process::exit(2);
        }
    }

    if !config_path.is_file() {
        eprintln!("Config not found: {}", config_path.display());
        std::process::exit(2);
    }
    if !out_dir.is_dir() {
        if let Err(err) = fs::create_dir_all(&out_dir) {
            eprintln!("Cannot create output dir {}: {err}", out_dir.display());
            std::process::exit(2);
        }
    }

    let cfg = VehicleConfig::from_json_path(&config_path).expect("failed to load JSON config");
    println!("=== Formula-90 Tire Sweep ===");
    println!("Config: {}", config_path.display());
    println!("Output dir: {}", out_dir.display());

    let modes: Vec<SweepMode> = match mode_arg.as_str() {
        "lateral" => vec![SweepMode::Lateral],
        "longitudinal" => vec![SweepMode::Longitudinal],
        "combined" => vec![SweepMode::Combined],
        "all" => SweepMode::all().to_vec(),
        other => {
            eprintln!("Unknown mode: {other}");
            std::process::exit(2);
        }
    };

    let (branch, commit) = git_head_info();
    let metadata = serde_json::json!({
        "tool": "tire_sweep",
        "git_branch": branch,
        "git_commit": commit,
        "config_path": config_path.display().to_string(),
        "config_fnv64_hash": file_fnv64(&config_path),
        "godot_version": godot_project_version(),
        "test_scene": "sweep_standalone_flat_road",
        "load_multiples": LOAD_MULTIPLES,
        "alpha_grid_rad": ALPHA_GRID,
        "kappa_grid": KAPPA_GRID,
        "wheels": ["FL", "FR", "RL", "RR"],
        "settle_substeps": 240usize,
        "sweep_dt_s": 1.0 / 120.0,
    });

    for mode in &modes {
        let mut lines = vec![CSV_HEADER.to_string()];
        let mut eval_count = 0usize;
        for &wheel in WheelIndex::ALL.iter() {
            let ref_load = reference_load_n(&cfg, wheel);
            for &multiple in LOAD_MULTIPLES.iter() {
                let fz = ref_load * multiple;
                match mode {
                    SweepMode::Lateral => {
                        for &alpha in ALPHA_GRID.iter() {
                            let p = steady_tire_force(&cfg, wheel, fz, alpha, 0.0);
                            validate(&p, wheel, fz, alpha, 0.0).unwrap_or_else(|msg| {
                                eprintln!("{msg}");
                                std::process::exit(1);
                            });
                            lines.push(row(wheel, fz, &p));
                            eval_count += 1;
                        }
                    }
                    SweepMode::Longitudinal => {
                        for &kappa in KAPPA_GRID.iter() {
                            let p = steady_tire_force(&cfg, wheel, fz, 0.0, kappa);
                            validate(&p, wheel, fz, 0.0, kappa).unwrap_or_else(|msg| {
                                eprintln!("{msg}");
                                std::process::exit(1);
                            });
                            lines.push(row(wheel, fz, &p));
                            eval_count += 1;
                        }
                    }
                    SweepMode::Combined => {
                        for &alpha in ALPHA_GRID.iter() {
                            for &kappa in KAPPA_GRID.iter() {
                                let p = steady_tire_force(&cfg, wheel, fz, alpha, kappa);
                                validate(&p, wheel, fz, alpha, kappa).unwrap_or_else(|msg| {
                                    eprintln!("{msg}");
                                    std::process::exit(1);
                                });
                                lines.push(row(wheel, fz, &p));
                                eval_count += 1;
                            }
                        }
                    }
                }
            }
        }
        let out_path = out_dir.join(format!("tire_sweep_{}.csv", mode.name()));
        match fs::write(&out_path, lines.join("\n") + "\n") {
            Ok(()) => println!(
                "Wrote {} ({eval_count} evaluations)",
                out_path.display()
            ),
            Err(err) => {
                eprintln!("Failed to write {}: {err}", out_path.display());
                std::process::exit(1);
            }
        }
    }

    let meta_path = out_dir.join("tire_sweep_meta.json");
    if let Err(err) = fs::write(&meta_path, serde_json::to_string_pretty(&metadata).unwrap_or_default() + "\n") {
        eprintln!("Failed to write {}: {err}", meta_path.display());
        std::process::exit(1);
    }
    println!("Metadata: {}", meta_path.display());
}

fn validate(p: &TireForcePoint, wheel: WheelIndex, fz: f64, alpha: f64, kappa: f64) -> Result<(), String> {
    for (name, v) in [
        ("LateralForce_N", p.lateral_force_n),
        ("LongitudinalForce_N", p.longitudinal_force_n),
        ("AlignTorque_Nm", p.aligning_torque_nm),
        ("RollingResistance_N", p.rolling_resistance_n),
        ("EffSlipAngle_Rad", p.effective_slip_angle_rad),
        ("EffSlipRatio", p.effective_slip_ratio),
    ] {
        if !v.is_finite() {
            return Err(format!(
                "NON-FINITE {name}={v} at wheel={} fz={fz:.2} alpha={alpha} kappa={kappa}",
                wheel_name(wheel)
            ));
        }
    }
    Ok(())
}

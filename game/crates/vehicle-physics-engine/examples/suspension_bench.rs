//! SUS-GEO-08: comparative bench (cost + sweeps + equivalence + deltas).
//!
//! Usage: cargo run -p vehicle_physics_engine --example suspension_bench
//! Prints a JSON report to stdout (redirect to a file for records).
use std::time::Instant;
use vehicle_physics_engine::*;

const FIXTURE: &str = r#"{
  "version": 1,
  "front": {
    "spring_rate_N_per_m": 117000.0, "spring_free_length_m": 0.2826,
    "spring_installed_length_m": 0.26,
    "damper_bump_Ns_per_m": 11800.0, "damper_rebound_Ns_per_m": 15930.0,
    "damper_knee_m_per_s": 0.127, "damper_fast_factor": 0.5,
    "wheel_droop_m": 0.05, "wheel_bump_m": 0.10,
    "damper_min_m": 0.15, "damper_max_m": 0.40, "bump_stop_mult": 2.6
  },
  "rear": {
    "spring_rate_N_per_m": 185000.0, "spring_free_length_m": 0.2727,
    "spring_installed_length_m": 0.25,
    "damper_bump_Ns_per_m": 20500.0, "damper_rebound_Ns_per_m": 27675.0,
    "damper_knee_m_per_s": 0.127, "damper_fast_factor": 0.5,
    "wheel_droop_m": 0.05, "wheel_bump_m": 0.10,
    "damper_min_m": 0.15, "damper_max_m": 0.40, "bump_stop_mult": 2.8
  },
  "front_arb": {"mode": "legacy_ratio", "ratio": 0.15},
  "rear_arb": {"mode": "legacy_ratio", "ratio": 0.09},
  "corners": {
    "FL": {
      "hub_center": [-0.753, 0.0, -1.475],
      "lower_wishbone": {"inner_front": [-0.165237, -0.032255, -1.520731], "inner_rear": [-0.1912, -0.031258, -1.063059], "outer": [-0.585831, -0.0317, -1.478898]},
      "upper_wishbone": {"inner_front": [-0.165237, 0.136096, -1.520731], "inner_rear": [-0.1912, 0.137093, -1.196632], "outer": [-0.585831, 0.13665, -1.478898]},
      "trackrod": {"inner": [-0.163643, -0.0329515, -1.6063275], "outer": [-0.581991, -0.0334055, -1.586078]},
      "rod": {"outer": [-0.549739, -0.019274, -1.4389295], "attachment": "lower", "type": "pushrod"},
      "rocker": {"pivot": [-0.162718, 0.2204355, -1.4462085], "axis": [1.0, 0.0, 0.0], "pushrod_arm": [-0.162718, 0.2204355, -1.6062085], "damper_arm": [-0.162718, 0.2204355, -1.5262085]},
      "damper": {"chassis": [-0.162718, 0.5004355, -1.5462085]}
    },
    "FR": {"mirror_of": "FL"},
    "RL": {
      "hub_center": [-0.716, 0.0, 1.475],
      "lower_wishbone": {"inner_front": [-0.2916855, -0.1203565, 1.1482685], "inner_rear": [-0.073654, -0.1203565, 1.624112], "outer": [-0.505193, -0.0596035, 1.474073]},
      "upper_wishbone": {"inner_front": [-0.1457255, 0.0552935, 1.1460325], "inner_rear": [-0.073654, 0.0512445, 1.624112], "outer": [-0.505193, 0.1160465, 1.474073]},
      "trackrod": {"inner": [-0.085, -0.015, 1.61], "outer": [-0.505193, -0.015, 1.55]},
      "rod": {"outer": [-0.48, 0.11, 1.43], "attachment": "upper", "type": "pullrod"},
      "rocker": {"pivot": [-0.2, -0.145, 1.39], "axis": [1.0, 0.0, 0.0], "pushrod_arm": [-0.2, -0.145, 1.25], "damper_arm": [-0.2, -0.145, 1.32]},
      "damper": {"chassis": [-0.2, 0.115, 1.34]},
      "driveshaft": {"inner": [-0.088833, -0.0033785, 1.487802], "outer": [-0.716, 0.0, 1.475]}
    },
    "RR": {"mirror_of": "RL"}
  }
}"#;

fn bench_us(iters: usize, mut f: impl FnMut()) -> f64 {
    // Warmup then timed.
    for _ in 0..iters.min(50) {
        f();
    }
    let t = Instant::now();
    for _ in 0..iters {
        f();
    }
    t.elapsed().as_secs_f64() * 1e6 / iters as f64
}

fn main() {
    let geo: GeometricSuspensionConfig =
        vehicle_physics_engine::geometric_from_json_value(&serde_json::from_str(FIXTURE).unwrap())
            .expect("fixture");
    let mut rep = serde_json::Map::new();

    // 1. Motion-ratio sweep per axle corner.
    for (name, wheel) in [("FL", WheelIndex::FrontLeft), ("RL", WheelIndex::RearLeft)] {
        let corner = geo.corners.get(wheel);
        let mut rows = Vec::new();
        let mut rmin = f64::INFINITY;
        let mut rmax = f64::NEG_INFINITY;
        for i in 0..=20 {
            let q = -0.05 + 0.15 * (i as f64 / 20.0);
            let j = jacobian(corner, q, 0.0, wheel.is_front()).unwrap();
            rmin = rmin.min(j.motion_ratio);
            rmax = rmax.max(j.motion_ratio);
            rows.push(serde_json::json!({"q": q, "r": j.motion_ratio}));
        }
        rep.insert(
            format!("motion_ratio_{name}"),
            serde_json::json!({"min": rmin, "max": rmax, "rows": rows}),
        );
        // 2. Tangent rate + wheel damping at rest.
        let axle = geo.axle(wheel);
        let r0 = jacobian(corner, 0.0, 0.0, wheel.is_front()).unwrap().motion_ratio;
        let pre = axle.spring_rate_N_per_m * (axle.spring_free_length_m - axle.spring_installed_length_m);
        let e = 1e-4;
        let rp = jacobian(corner, e, 0.0, wheel.is_front()).unwrap().motion_ratio;
        let rm = jacobian(corner, -e, 0.0, wheel.is_front()).unwrap().motion_ratio;
        let drdq = (rp - rm) / (2.0 * e);
        rep.insert(
            format!("rest_rate_{name}"),
            serde_json::json!({
                "r0": r0,
                "k_wheel_tangent": axle.spring_rate_N_per_m * r0 * r0 + pre * drdq,
                "k_wheel_linear": axle.spring_rate_N_per_m * r0 * r0,
                "c_wheel_bump": axle.damper_bump_Ns_per_m * r0 * r0,
                "c_wheel_rebound": axle.damper_rebound_Ns_per_m * r0 * r0,
            }),
        );
    }

    // 3. Push/pull label equivalence over a sweep (max abs delta).
    let mut v: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    v["corners"]["FL"]["rod"]["type"] = serde_json::json!("pullrod");
    v["corners"]["RL"]["rod"]["type"] = serde_json::json!("pushrod");
    let geo2 = vehicle_physics_engine::geometric_from_json_value(&v).expect("relabeled");
    let mut max_d = 0.0f64;
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        let (a, b) = (geo.corners.get(wheel), geo2.corners.get(wheel));
        for i in 0..=20 {
            let q = -0.05 + 0.15 * (i as f64 / 20.0);
            let sa = solve_corner(a, q, 0.0, wheel.is_front(), 0.0, 0.0, 1.0).unwrap();
            let sb = solve_corner(b, q, 0.0, wheel.is_front(), 0.0, 0.0, 1.0).unwrap();
            max_d = max_d.max((sa.hub - sb.hub).length());
            max_d = max_d.max((sa.damper_length - sb.damper_length).abs());
        }
    }
    rep.insert("push_pull_max_delta_m".to_string(), serde_json::json!(max_d));

    // 4. Cost per tick (release-fast path measured here in this build).
    let fl = geo.corners.get(WheelIndex::FrontLeft);
    let c_solve = bench_us(200, || {
        let _ = solve_corner(fl, 0.02, 0.0, true, 0.0, 0.0, 1.0);
    });
    let c_jac = bench_us(200, || {
        let _ = jacobian(fl, 0.02, 0.0, true);
    });
    rep.insert("cost_us".to_string(), serde_json::json!({"solve_corner": c_solve, "jacobian": c_jac}));

    println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(rep)).unwrap());
}

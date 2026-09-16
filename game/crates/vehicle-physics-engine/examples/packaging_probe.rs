//! SUS-GEO track probe: front rocker/damper packaging sweep for the F1 2030
//! geometric profile.
//!
//! Loads a profile JSON, patches the FL corner's `rocker.damper_arm` and
//! `damper.chassis` per candidate, and reports motion ratio, damper stroke,
//! reachability and the recalibration that preserves the profile's own rest
//! tangent wheel rate and static stance. Used to justify the packaging fix
//! (docs/suspension_validation.md, packaging envelope section).
//!
//! Usage: cargo run -p vehicle_physics_engine --example packaging_probe -- <profile.json>
use std::env;
use std::fs;

use serde_json::{json, Value};
use vehicle_physics_engine::*;

fn fl_patch(value: &mut Value, damper_arm_z: f64, chassis_y: f64, chassis_z: f64) {
    let front = value
        .get_mut("suspension")
        .and_then(|s| s.get_mut("geometry_physical"))
        .and_then(|g| g.get_mut("corners"))
        .and_then(|c| c.get_mut("FL"))
        .expect("geometry_physical.corners.FL");
    front["rocker"]["damper_arm"][2] = json!(damper_arm_z);
    front["damper"]["chassis"][1] = json!(chassis_y);
    front["damper"]["chassis"][2] = json!(chassis_z);
}

fn tangent_and_static(corner: &CornerHardpoints, axle: &AxlePhysicalElements) -> (f64, f64, f64, f64) {
    let e = 1e-5;
    let r0 = jacobian(corner, 0.0, 0.0, true).map(|j| j.motion_ratio).unwrap_or(0.0);
    let r_plus = jacobian(corner, e, 0.0, true).map(|j| j.motion_ratio).unwrap_or(r0);
    let r_minus = jacobian(corner, -e, 0.0, true).map(|j| j.motion_ratio).unwrap_or(r0);
    let dm_dq = (r_plus - r_minus) / (2.0 * e);
    let preload = axle.spring_free_length_m - axle.spring_installed_length_m;
    let f_s_rest = axle.spring_rate_N_per_m * preload;
    let tangent = axle.spring_rate_N_per_m * r0 * r0 + f_s_rest * dm_dq;
    (r0, dm_dq, f_s_rest, tangent)
}

fn report(label: &str, value: &Value, target_tangent: f64) {
    let text = serde_json::to_string(value).unwrap();
    let cfg = match VehicleConfig::from_json_str(&text) {
        Ok(c) => c,
        Err(e) => {
            println!("{label}: CONFIG ERROR {e}");
            return;
        }
    };
    let geo = match cfg.geometric_suspension.as_ref() {
        Some(g) => g,
        None => {
            println!("{label}: no geometric suspension");
            return;
        }
    };
    let corner = &geo.corners.fl;
    let axle = &geo.front;
    let (q_min, q_max) = travel_envelope(&corner, axle.wheel_droop_m, axle.wheel_bump_m, 0.0, true);
    let jac0 = match jacobian(&corner, 0.0, 0.0, true) {
        Some(j) => j,
        None => {
            println!("{label}: rest solve failed");
            return;
        }
    };
    let r0 = jac0.motion_ratio;
    let mut r_min = r0;
    let mut r_max = r0;
    let mut len_min = f64::INFINITY;
    let mut len_max = f64::NEG_INFINITY;
    let mut all_converged = true;
    let steps = 40;
    for step in 0..=steps {
        let q = q_min + (q_max - q_min) * (step as f64 / steps as f64);
        match solve_corner(&corner, q, 0.0, true, 0.0, 0.0, 1.0) {
            Some(sol) => {
                all_converged &= sol.converged;
                let jac = jacobian(&corner, q, 0.0, true);
                if let Some(j) = jac {
                    if j.motion_ratio.is_finite() {
                        r_min = r_min.min(j.motion_ratio);
                        r_max = r_max.max(j.motion_ratio);
                    }
                }
                len_min = len_min.min(sol.damper_length);
                len_max = len_max.max(sol.damper_length);
            }
            None => all_converged = false,
        }
    }
    let damper_rest = (corner.damper_chassis - corner.rocker_damper_arm).length();
    let _ = damper_rest;
    // Recalibration: preserve the SHIPPED rest tangent wheel rate
    // (target_tangent) and the static stance under the new r0.
    let (_r0_t, dm_dq, _fs_t, _tangent_t) = tangent_and_static(corner, axle);
    let static_load = cfg.vehicle_mass * 9.81 * cfg.front_weight_distribution / 2.0;
    let force_rest_needed = static_load / r0;
    // k' such that k'*r0'^2 + F_s'*dm/dq' = target_tangent.
    let k_new = (target_tangent - force_rest_needed * dm_dq) / (r0 * r0);
    let free_new = axle.spring_installed_length_m + force_rest_needed / k_new;
    println!(
        "{label}: r0={r0:.6} r[{r_min:.3},{r_max:.3}] stroke[{len_min:.4},{len_max:.4}] \
envelope[{q_min:.4},{q_max:.4}] converged={all_converged} \
dm/dq={dm_dq:.6} | calib: k'={k_new:.2} free'={free_new:.8} (installed 0.26, F_s'={force_rest_needed:.2} N, static={static_load:.2} N)",
    );
}

fn main() {
    let path = env::args().nth(1).expect("profile json path");
    let raw = fs::read_to_string(&path).expect("read profile");
    let base: Value = serde_json::from_str(&raw).expect("parse profile");

    // The profile's own rest tangent wheel rate is the recalibration target.
    {
        let text = serde_json::to_string(&base).unwrap();
        let cfg = VehicleConfig::from_json_str(&text).expect("base config");
        let geo = cfg.geometric_suspension.as_ref().expect("geometric");
        let (r0, _dm, _fs, tangent) = tangent_and_static(&geo.corners.fl, &geo.front);
        println!("target: r0={r0:.6} tangent_kw={tangent:.2}");
        report("current", &base, tangent);

        // Candidate D (chosen): aft damper arm + low inboard mount.
        let mut v = base.clone();
        fl_patch(&mut v, -1.30, 0.02, -1.24);
        report("D arm_z=-1.30 chassis(0.02,-1.24)", &v, tangent);

        // Candidate C (runner-up) kept for the record.
        let mut v = base.clone();
        fl_patch(&mut v, -1.276, 0.06, -1.12);
        report("C arm_z=-1.276 chassis(0.06,-1.12)", &v, tangent);
    }
}



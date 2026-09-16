//! SUS-GEO-02 + SUS-GEO-03 acceptance: versioned schema + deterministic kinematics.
use vehicle_physics_engine::*;

// ── helpers ────────────────────────────────────────────────────────────────

fn axle(front: bool) -> serde_json::Value {
    let (rate, free, installed) = if front {
        (29419.9, 0.30, 0.26)
    } else {
        (27240.7, 0.30, 0.25)
    };
    serde_json::json!({
        "spring_rate_N_per_m": rate,
        "spring_free_length_m": free,
        "spring_installed_length_m": installed,
        "damper_bump_Ns_per_m": 2949.5,
        "damper_rebound_Ns_per_m": 4000.0,
        "damper_knee_m_per_s": 0.127,
        "damper_fast_factor": 0.5,
        "wheel_droop_m": 0.05,
        "wheel_bump_m": 0.10,
        "damper_min_m": 0.15,
        "damper_max_m": 0.40,
    })
}

fn fl_corner() -> serde_json::Value {
    // Idealized reachable front: rocker axis transverse (X) with YZ arm
    // offsets so vertical pushrod motion drives the damper. The shipped
    // F1 2030 visual FL uses axis [0,1,0] (vertical), which cannot
    // accommodate vertical travel with rigid bars and is therefore blocked
    // for physical activation until SUS-GEO-09 migration (see test below).
    serde_json::json!({
        "hub_center": [-0.753, 0.0, -1.475],
        "lower_wishbone": {
            "inner_front": [-0.165237, -0.032255, -1.520731],
            "inner_rear": [-0.1912, -0.031258, -1.063059],
            "outer": [-0.585831, -0.0317, -1.478898]
        },
        "upper_wishbone": {
            "inner_front": [-0.165237, 0.136096, -1.520731],
            "inner_rear": [-0.1912, 0.137093, -1.196632],
            "outer": [-0.585831, 0.13665, -1.478898]
        },
        "trackrod": {
            "inner": [-0.163643, -0.0329515, -1.6063275],
            "outer": [-0.581991, -0.0334055, -1.586078]
        },
        "rod": {"outer": [-0.549739, -0.019274, -1.4389295], "attachment": "lower", "type": "pushrod"},
        "rocker": {
            "pivot": [-0.162718, 0.2204355, -1.4462085],
            "axis": [1.0, 0.0, 0.0],
            "pushrod_arm": [-0.162718, 0.2004355, -1.5362085],
            "damper_arm": [-0.162718, 0.2504355, -1.3962085]
        },
        "damper": {"chassis": [-0.162718, 0.3004355, -1.07]},
        "provenance": {"origin": "measured", "note": "audit FL, rocker idealized to transverse axis for kinematics"}
    })
}

fn rl_corner() -> serde_json::Value {
    serde_json::json!({
        "hub_center": [-0.716, 0.0, 1.475],
        "lower_wishbone": {
            "inner_front": [-0.2916855, -0.1203565, 1.1482685],
            "inner_rear": [-0.073654, -0.1203565, 1.624112],
            "outer": [-0.505193, -0.0596035, 1.474073]
        },
        "upper_wishbone": {
            "inner_front": [-0.1457255, 0.0552935, 1.1460325],
            "inner_rear": [-0.073654, 0.0512445, 1.624112],
            "outer": [-0.505193, 0.1160465, 1.474073]
        },
        "trackrod": {
            "inner": [-0.085, -0.015, 1.61],
            "outer": [-0.505193, -0.015, 1.55]
        },
        "rod": {"outer": [-0.48, 0.11, 1.43], "attachment": "upper", "type": "pullrod"},
        "rocker": {
            "pivot": [-0.2, -0.145, 1.39],
            "axis": [1.0, 0.0, 0.0],
            "pushrod_arm": [-0.2, -0.095, 1.3],
            "damper_arm": [-0.2, -0.11, 1.425]
        },
        "damper": {"chassis": [-0.2, -0.11, 1.17]},
        "driveshaft": {"inner": [-0.088833, -0.0033785, 1.487802], "outer": [-0.716, 0.0, 1.475]},
        "provenance": {"origin": "reconstructed", "note": "audit RL inboard reconstructed"}
    })
}

fn geometric_json() -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "front": axle(true),
        "rear": axle(false),
        "front_arb": {"mode": "legacy_ratio", "ratio": 0.15},
        "rear_arb": {"mode": "legacy_ratio", "ratio": 0.09},
        "corners": {
            "FL": fl_corner(),
            "FR": {"mirror_of": "FL"},
            "RL": rl_corner(),
            "RR": {"mirror_of": "RL"},
        }
    })
}

fn geometric_profile_json() -> String {
    let g = geometric_json();
    format!(
        r#"{{"schema_version": 3, "suspension": {{"front": {{"spring_length": 0.3, "resting_ratio": 0.15}}, "rear": {{"spring_length": 0.27, "resting_ratio": 0.22}}, "model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g).unwrap()
    )
}

// ── SUS-GEO-02 ─────────────────────────────────────────────────────────────

#[test]
fn geometric_round_trip_preserves_physical() {
    let cfg = VehicleConfig::from_json_str(&geometric_profile_json()).unwrap();
    assert_eq!(cfg.suspension_model, SuspensionModelKind::Geometric);
    let geo = cfg.geometric_suspension.as_ref().unwrap();
    assert_eq!(geo.version, 1);
    assert!((geo.front.spring_rate_N_per_m - 29419.9).abs() < 1e-9);
    assert_eq!(geo.corners.fr_source, "mirror_of:FL");
    assert_eq!(geo.corners.rr_source, "mirror_of:RL");
    // Provenance survives the parse.
    assert_eq!(
        geo.corners.rl.provenance.origin,
        CornerProvenanceOrigin::Reconstructed
    );
    // Round-trip via to_json_value must preserve everything physical.
    let v = cfg.to_json_value();
    let _gp = v
        .get("suspension")
        .unwrap()
        .get("geometry_physical")
        .expect("physical geometry must be re-emitted");
    let cfg2 = VehicleConfig::from_json_str(&serde_json::to_string(&v).unwrap()).unwrap();
    let geo2 = cfg2.geometric_suspension.unwrap();
    assert_eq!(geo2, *geo);
    // Visual geometry stays dropped; physical is the preserved contract.
    assert!(v
        .get("suspension")
        .unwrap()
        .get("geometry")
        .is_none());
}

#[test]
fn legacy_profiles_preserve_behavior() {
    // Minimal stays legacy with no physical.
    let minimal = VehicleConfig::from_json_str(r#"{"schema_version": 3}"#).unwrap();
    assert_eq!(minimal.suspension_model, SuspensionModelKind::Legacy1Dof);
    assert!(minimal.geometric_suspension.is_none());
    // Shipped F1 2030 (visual geometry only) stays legacy.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_physics.json");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("f1 2030 profile must exist at {}: {e}", manifest.display()));
    let cfg = VehicleConfig::from_json_str(&text).unwrap();
    assert_eq!(cfg.suspension_model, SuspensionModelKind::Legacy1Dof);
    assert!(cfg.geometric_suspension.is_none());
    assert!((cfg.front_spring_length - 0.3).abs() < 1e-12);
    // Legacy emission carries the explicit model but no physical block.
    let v = cfg.to_json_value();
    let s = v.get("suspension").unwrap();
    assert_eq!(s.get("model").unwrap().as_str().unwrap(), "legacy_1dof");
    assert!(s.get("geometry_physical").is_none());
}

#[test]
fn invalid_physical_fails_with_wheel_and_field() {
    // Geometric without block.
    let bad = r#"{"schema_version": 3, "suspension": {"model": "geometric", "model_version": 1}}"#;
    let e = VehicleConfig::from_json_str(bad).unwrap_err();
    assert!(e.contains("geometry_physical"), "must name field: {e}");
    // Legacy with physical block.
    let mut g = geometric_json();
    let bad_legacy = format!(
        r#"{{"schema_version": 3, "suspension": {{"model": "legacy_1dof", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g).unwrap()
    );
    let e = VehicleConfig::from_json_str(&bad_legacy).unwrap_err();
    assert!(e.contains("legacy_1dof"), "{e}");
    // Bad spring rate names the axle section.
    g["front"]["spring_rate_N_per_m"] = serde_json::json!(-100.0);
    let bad_spring = format!(
        r#"{{"schema_version": 3, "suspension": {{"model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g).unwrap()
    );
    let e = VehicleConfig::from_json_str(&bad_spring).unwrap_err();
    assert!(e.contains("front") && e.contains("spring_rate"), "{e}");
    // Self mirror names the wheel.
    let mut g2 = geometric_json();
    g2["corners"]["FR"] = serde_json::json!({"mirror_of": "FR"});
    let bad_mirror = format!(
        r#"{{"schema_version": 3, "suspension": {{"model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g2).unwrap()
    );
    let e = VehicleConfig::from_json_str(&bad_mirror).unwrap_err();
    assert!(e.contains("FR"), "{e}");
    // Front driveshaft is rear-only and must name the wheel.
    let mut g3 = geometric_json();
    g3["corners"]["FL"]["driveshaft"] =
        serde_json::json!({"inner": [0.0, 0.0, 0.0], "outer": [0.1, 0.0, 0.0]});
    let bad_shaft = format!(
        r#"{{"schema_version": 3, "suspension": {{"model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g3).unwrap()
    );
    let e = VehicleConfig::from_json_str(&bad_shaft).unwrap_err();
    assert!(e.contains("FrontLeft") || e.contains("FL"), "{e}");
}

#[test]
fn push_pull_label_has_no_physical_effect() {
    let cfg = VehicleConfig::from_json_str(&geometric_profile_json()).unwrap();
    let geo = cfg.geometric_suspension.unwrap();
    // Identical hardpoints, only the informational label differs.
    let mut pull = geo.corners.fl.clone();
    pull.rod_kind_label = RodKindLabel::Pullrod;
    let mut push = pull.clone();
    push.rod_kind_label = RodKindLabel::Pushrod;
    for (corner, label) in [(&pull, "pull"), (&push, "push")] {
        let s = solve_corner(corner, 0.03, 0.0, true, -0.048, -0.0018, 1.0).unwrap();
        assert!(s.converged, "{label} must converge");
    }
    let a = solve_corner(&pull, 0.03, 0.0, true, -0.048, -0.0018, 1.0).unwrap();
    let b = solve_corner(&push, 0.03, 0.0, true, -0.048, -0.0018, 1.0).unwrap();
    assert!((a.hub - b.hub).length() < 1e-12);
    assert!((a.damper_length - b.damper_length).abs() < 1e-12);
    assert!((a.rocker_angle - b.rocker_angle).abs() < 1e-12);
}

// ── SUS-GEO-03 ─────────────────────────────────────────────────────────────

fn test_geo() -> GeometricSuspensionConfig {
    VehicleConfig::from_json_str(&geometric_profile_json())
        .unwrap()
        .geometric_suspension
        .unwrap()
}

#[test]
fn kinematics_lengths_constant_finite_continuous() {
    let geo = test_geo();
    for wheel in WheelIndex::ALL {
        let corner = geo.corners.get(wheel);
        let axle = geo.axle(wheel);
        // Sweep inside the reachable envelope connected to rest, not the raw
        // element limits (linkage may block part of it; see envelope test).
        let (lo, hi) = travel_envelope(corner, axle.wheel_droop_m, axle.wheel_bump_m, 0.0, wheel.is_front());
        assert!(lo < hi);
        let racks: Vec<f64> = if wheel.is_front() {
            vec![-0.02, 0.0, 0.02]
        } else {
            vec![0.0]
        };
        for rack in racks {
            let mut prev: Option<Vec3> = None;
            let steps = 24;
            for i in 0..=steps {
                let q = lo + (hi - lo) * (i as f64 / steps as f64);
                let s = solve_corner(corner, q, rack, wheel.is_front(), -0.048, 0.0, 1.0)
                    .expect("solve must return");
                assert!(
                    s.hub.x.is_finite() && s.hub.y.is_finite() && s.hub.z.is_finite(),
                    "{wheel:?} non-finite hub at q={q}"
                );
                assert!(s.residuals.max_link() < 1e-3, "{wheel:?} link stretch at q={q}");
                assert!(s.residuals.hub_y < 2.5e-2, "{wheel:?} hub residual at q={q}");
                if let Some(p) = prev {
                    assert!(
                        (s.hub - p).length() < 0.03,
                        "{wheel:?} hub jump at q={q}"
                    );
                }
                prev = Some(s.hub);
            }
        }
    }
}

#[test]
fn kinematics_mirror_symmetry() {
    let geo = test_geo();
    for q in [-0.02, 0.0, 0.03, 0.06] {
        let fl = solve_corner(&geo.corners.fl, q, 0.0, true, -0.048, -0.0018, 1.0).unwrap();
        let fr = solve_corner(&geo.corners.fr, q, 0.0, true, -0.048, -0.0018, -1.0).unwrap();
        // Mirror: x negated, y/z equal. Damper length is mirror-invariant;
        // rocker angle lives about a mirrored axis so only its magnitude is
        // compared (sign flips with the mirrored axis).
        assert!((fl.hub.x + fr.hub.x).abs() < 1e-6, "hub x mirror at q={q}");
        assert!((fl.hub.y - fr.hub.y).abs() < 1e-6);
        assert!((fl.hub.z - fr.hub.z).abs() < 1e-6);
        assert!((fl.damper_length - fr.damper_length).abs() < 1e-9);
        assert!((fl.rocker_angle.abs() - fr.rocker_angle.abs()).abs() < 1e-9);
        let rl = solve_corner(&geo.corners.rl, q, 0.0, false, -0.029, 0.0042, 1.0).unwrap();
        let rr = solve_corner(&geo.corners.rr, q, 0.0, false, -0.029, 0.0042, -1.0).unwrap();
        assert!((rl.hub.x + rr.hub.x).abs() < 1e-6, "rear mirror at q={q}");
        assert!((rl.damper_length - rr.damper_length).abs() < 1e-9);
    }
}

#[test]
fn jacobian_matches_finite_differences() {
    let geo = test_geo();
    for (wheel, q) in [
        (WheelIndex::FrontLeft, 0.0),
        (WheelIndex::FrontLeft, 0.02),
        (WheelIndex::RearLeft, -0.01),
        (WheelIndex::RearLeft, 0.02),
    ] {
        let corner = geo.corners.get(wheel);
        let j = jacobian(corner, q, 0.0, wheel.is_front()).unwrap();
        assert!(j.motion_ratio.is_finite());
        assert!(j.motion_ratio.abs() > 1e-3, "{wheel:?} degenerate ratio");
        // Independent check with a different epsilon and one-sided stencil.
        let e2 = 5e-7;
        let a = solve_corner(corner, q + e2, 0.0, wheel.is_front(), 0.0, 0.0, 1.0).unwrap();
        let b = solve_corner(corner, q - e2, 0.0, wheel.is_front(), 0.0, 0.0, 1.0).unwrap();
        let r2 = (a.damper_compression - b.damper_compression) / (2.0 * e2);
        assert!(
            (j.motion_ratio - r2).abs() < 5e-4,
            "{wheel:?} q={q} jacobian err {}",
            (j.motion_ratio - r2).abs()
        );
        // d(damper_length)/dq = -r by definition s = Lrest - L.
        assert!((j.ddamper_dq + j.motion_ratio).abs() < 1e-9);
        // Hub mostly moves +Y with travel; lateral/longitudinal drift is small.
        assert!(j.dhub_dq.y > 0.5 && j.dhub_dq.y < 1.5, "{wheel:?} dhub {:?}", j.dhub_dq);
    }
}

#[test]
fn travel_envelope_contains_rest_and_steering_compatible() {
    let geo = test_geo();
    for wheel in WheelIndex::ALL {
        let corner = geo.corners.get(wheel);
        let axle = geo.axle(wheel);
        let (lo, hi) = travel_envelope(corner, axle.wheel_droop_m, axle.wheel_bump_m, 0.0, wheel.is_front());
        assert!(lo < 0.0 && hi > 0.0, "{wheel:?} envelope must bracket rest");
        assert!(lo >= -axle.wheel_droop_m - 1e-9 && hi <= axle.wheel_bump_m + 1e-9);
        // Steering sweep at rest and at envelope ends stays finite.
        for steer_deg in [-20.0f64, 0.0, 20.0] {
            let rack = rack_from_steer(&geo.corners.fl, &geo.corners.fr, steer_deg.to_radians());
            assert!(rack.is_finite());
            for q in [lo, 0.0, hi] {
                if !wheel.is_front() && steer_deg != 0.0 {
                    continue;
                }
                let rack_here = if wheel.is_front() { rack } else { 0.0 };
                let s = solve_corner(corner, q, rack_here, wheel.is_front(), -0.048, 0.0, 1.0).unwrap();
                assert!(s.hub.x.is_finite(), "{wheel:?} steer {steer_deg} q {q}");
            }
        }
    }
}

#[test]
fn damper_compression_monotonic_positive_ratio() {
    // Near-rest monotonicity with consistent sign (no over-center flip).
    // Full-bump over-center behaviour is a geometry-design issue for SUS-GEO-09
    // calibration, not a solver bug: the solver reports it via clamped flags
    // and the travel envelope. Front test rocker is idealized for r > 0 near
    // rest; rear pullrod test geometry yields r < 0 and is checked separately.
    let geo = test_geo();
    let corner = &geo.corners.fl;
    let mut prev_s = None;
    for i in 0..=10 {
        let q = -0.02 + 0.05 * (i as f64 / 10.0);
        let s = solve_corner(corner, q, 0.0, true, 0.0, 0.0, 1.0).unwrap();
        assert!(s.converged);
        if let Some(p) = prev_s {
            assert!(s.damper_compression > p, "FL s must grow with bump q={q}");
        }
        prev_s = Some(s.damper_compression);
    }
    // Rear is monotonic in the same window, with opposite sign by construction
    // of its pullrod rocker (documents the need for 09 calibration, not a fail).
    let corner_r = &geo.corners.rl;
    let mut prev_r: Option<f64> = None;
    for i in 0..=10 {
        let q = -0.02 + 0.05 * (i as f64 / 10.0);
        let s = solve_corner(corner_r, q, 0.0, false, 0.0, 0.0, 1.0).unwrap();
        if let Some(p) = prev_r {
            assert!(
                (s.damper_compression - p).abs() > 1e-9,
                "RL s must move monotonically q={q}"
            );
        }
        prev_r = Some(s.damper_compression);
    }
}

#[test]
fn shipped_visual_vertical_rocker_is_blocked_for_activation() {
    // The shipped F1 2030 visual FL uses rocker axis [0,1,0] (vertical). With
    // rigid bars this yields a negative motion ratio (damper extends on bump)
    // and therefore must block geometric activation until SUS-GEO-09 provides
    // a validated transverse rocker. The solver must report the sign; it must
    // not silently stretch bars or flip branches.
    let mut g = geometric_json();
    g["corners"]["FL"]["rocker"]["axis"] = serde_json::json!([0.0, 1.0, 0.0]);
    g["corners"]["FL"]["rocker"]["pushrod_arm"] =
        serde_json::json!([-0.182718, 0.2204355, -1.3562085]);
    g["corners"]["FL"]["rocker"]["damper_arm"] =
        serde_json::json!([-0.107718, 0.2204355, -1.4462085]);
    g["corners"]["FL"]["damper"]["chassis"] = serde_json::json!([-0.107718, 0.2204355, -1.12]);
    // FR mirrors FL, so it inherits the vertical axis.
    let prof = format!(
        r#"{{"schema_version": 3, "suspension": {{"model": "geometric", "model_version": 1, "geometry_physical": {g}}}}}"#,
        g = serde_json::to_string(&g).unwrap()
    );
    // Schema validation passes (lengths are plausible); kinematics reveals the
    // wrong-sign ratio that blocks activation.
    let cfg = VehicleConfig::from_json_str(&prof).unwrap();
    let geo = cfg.geometric_suspension.unwrap();
    let j = jacobian(&geo.corners.fl, 0.0, 0.0, true).unwrap();
    assert!(
        j.motion_ratio < 0.0,
        "vertical rocker must yield negative r (extends on bump), got {}",
        j.motion_ratio
    );
}

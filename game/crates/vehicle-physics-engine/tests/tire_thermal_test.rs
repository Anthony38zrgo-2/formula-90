// Deterministic pressure + thermal tire tests (ACCEPTANCE.md sections 2-8, 10).
use vehicle_physics_engine::*;

fn flat_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut out = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let i = wheel as usize;
        let anchor_local = sim.config.wheel_anchor_local(wheel);
        let anchor_world = sim.state.transform.transform_point(anchor_local);
        let distance = anchor_world.y.max(0.0);
        let span = if wheel.is_front() { sim.config.front_tire_width } else { sim.config.rear_tire_width }
            * sim.config.tri_ray_spacing_ratio;
        let side = if wheel.is_left() { 1.0 } else { -1.0 };
        let hit = |x: f64| RaycastHit {
            is_colliding: true,
            distance,
            point: Vec3::new(anchor_world.x + x, 0.0, anchor_world.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        out[i] = TriRaycastSample {
            inner: hit(side * span),
            center: hit(0.0),
            outer: hit(-side * span),
        };
    }
    out
}

fn braking_thermal_input() -> TireThermalInput {
    TireThermalInput {
        normal_force_n: 5000.0,
        longitudinal_force_n: 4200.0,
        lateral_force_n: 1500.0,
        slip_velocity_long_ms: 5.0,
        slip_velocity_lat_ms: 2.0,
        tire_deflection_m: 0.008,
        tire_deflection_velocity_m_s: 0.4,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: -0.02,
        vehicle_speed_ms: 40.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [1.0, 2.0, 1.0],
    }
}

#[test]
fn pressure_sweep_mechanical_modifiers_are_monotonic() {
    let p = TirePressureConfig::default();
    let t = TireThermalConfig::default();
    let mut sys = TireThermalSystem::new(&p, &t);
    let pressures = [90.0, 110.0, 145.0, 170.0];
    for (i, &pr) in pressures.iter().enumerate() {
        sys.wheels[i].pressure_kpa_gauge = pr;
    }
    let mods: Vec<TireMechanicalModifiers> = (0..4)
        .map(|i| sys.mechanical_modifiers(WheelIndex::ALL[i], &p, &t))
        .collect();
    for w in 0..3 {
        assert!(
            mods[w].vertical_stiffness_scale < mods[w + 1].vertical_stiffness_scale,
            "vertical stiffness must increase with pressure"
        );
        assert!(
            mods[w].max_deflection_scale > mods[w + 1].max_deflection_scale,
            "max deflection must decrease with pressure"
        );
        assert!(
            mods[w].contact_patch_scale > mods[w + 1].contact_patch_scale,
            "contact patch must decrease with pressure"
        );
        assert!(
            mods[w].relaxation_length_scale > mods[w + 1].relaxation_length_scale,
            "relaxation length must decrease with pressure"
        );
        assert!(
            mods[w].rolling_resistance_scale > mods[w + 1].rolling_resistance_scale,
            "rolling resistance must decrease with pressure"
        );
    }
    for m in &mods {
        assert!(m.vertical_stiffness_scale.is_finite());
        assert!(m.grip_scale.is_finite());
        assert!(m.pneumatic_trail_scale.is_finite());
    }
}

#[test]
fn pressure_sweep_vertical_deflection_decreases() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;
    let rest_fl = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let rest_rl = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    let hit = |d: f64| RaycastHit {
        is_colliding: true,
        distance: d,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    let samples = [
        TriRaycastSample { inner: hit(rest_fl), center: hit(rest_fl), outer: hit(rest_fl) },
        TriRaycastSample { inner: hit(rest_fl), center: hit(rest_fl), outer: hit(rest_fl) },
        TriRaycastSample { inner: hit(rest_rl), center: hit(rest_rl), outer: hit(rest_rl) },
        TriRaycastSample { inner: hit(rest_rl), center: hit(rest_rl), outer: hit(rest_rl) },
    ];

    let mut deflections = Vec::new();
    for &pr in &[90.0, 110.0, 145.0, 170.0] {
        let mut pcfg = TirePressureConfig::default();
        pcfg.cold_kpa_gauge = [pr; 4];
        let sys = TireThermalSystem::new(&pcfg, &cfg.tire_thermal.front);
        let mods: [TireMechanicalModifiers; 4] = [0, 1, 2, 3]
            .map(|i| sys.mechanical_modifiers(WheelIndex::ALL[i], &pcfg, &cfg.tire_thermal.front));
        let mut suspension = SuspensionSystem::new(&cfg);
        for _ in 0..240 {
            suspension.step_with_modifiers(&cfg, &mods, &samples, dt);
        }
        deflections.push(suspension.wheels[0].tire_deflection_m);
        assert!(
            suspension.wheels[0].total_normal_force.is_finite()
                && suspension.wheels[0].tire_deflection_m.is_finite()
        );
    }
    for w in 0..3 {
        assert!(
            deflections[w] > deflections[w + 1] + 1e-6,
            "carcass deflection must decrease with pressure: {} vs {}",
            deflections[w],
            deflections[w + 1]
        );
    }
}

#[test]
fn heat_soak_tread_fastest_carcass_slower_gas_slowest() {
    let p = TirePressureConfig::default();
    let t = TireThermalConfig::default();
    let env = TireEnvironment::fallback(&t);
    let dt = 1.0 / 120.0;
    let input = braking_thermal_input();

    let mut sys = TireThermalSystem::new(&p, &t);
    // Early window (1 s): tread already rising, gas still cold.
    for _ in 0..120 {
        sys.step_after_forces(WheelIndex::FrontLeft, &p, &t, env, input, dt);
    }
    assert!(sys.wheels[0].average_tread_c() > 26.0, "tread must rise quickly");
    assert!(
        (sys.wheels[0].gas_c - 25.0).abs() < 0.5,
        "gas must barely move early: {}",
        sys.wheels[0].gas_c
    );

    // Sustained heavy braking for 5 s.
    for _ in 0..600 {
        sys.step_after_forces(WheelIndex::FrontLeft, &p, &t, env, input, dt);
    }
    let w = &sys.wheels[0];
    assert!(w.average_tread_c() > w.carcass_c, "tread hotter than carcass");
    assert!(w.carcass_c > w.gas_c, "carcass hotter than gas");
    assert!(w.gas_c > 25.0, "gas must warm during the run");

    // Gas drives the pressure: a warm gas directly lifts hot gauge pressure well
    // above the cold setup (ideal-gas law through step_after_forces).
    let mut sys2 = TireThermalSystem::new(&p, &t);
    sys2.wheels[0].gas_c = 70.0;
    sys2.step_after_forces(WheelIndex::FrontLeft, &p, &t, env, input, dt);
    assert!(
        sys2.wheels[0].pressure_kpa_gauge > p.cold_kpa_gauge[0] + 20.0,
        "hot pressure must rise after gas warms: {}",
        sys2.wheels[0].pressure_kpa_gauge
    );

    // Release slip: straight-line coasting cools the tread faster than the gas.
    let tread_before = sys.wheels[0].average_tread_c();
    let gas_before = sys.wheels[0].gas_c;
    let cool = TireThermalInput {
        normal_force_n: 4000.0,
        longitudinal_force_n: 0.0,
        lateral_force_n: 0.0,
        slip_velocity_long_ms: 0.0,
        slip_velocity_lat_ms: 0.0,
        tire_deflection_m: 0.008,
        tire_deflection_velocity_m_s: 0.0,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: 0.0,
        vehicle_speed_ms: 40.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [1.0, 2.0, 1.0],
    };
    for _ in 0..600 {
        sys.step_after_forces(WheelIndex::FrontLeft, &p, &t, env, cool, dt);
    }
    assert!(sys.wheels[0].average_tread_c() < tread_before, "tread must cool");
    assert!(
        (sys.wheels[0].gas_c - gas_before).abs() < 0.5,
        "gas must lag during the cooling window"
    );
}

#[test]
fn straight_line_symmetry_fl_equals_fr_rl_equals_rr() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    let dt = 1.0 / 120.0;
    let input = VehicleInput { throttle: 0.6, ..VehicleInput::default() };
    for _ in 0..600 {
        let samples = flat_samples(&sim);
        sim.step(&input, &samples, dt);
    }
    let t = &sim.state.tire_thermal;
    let cmp = |a: &WheelThermalState, b: &WheelThermalState| {
        (a.pressure_kpa_gauge - b.pressure_kpa_gauge).abs() < 0.2
            && (a.tread_inner_c - b.tread_inner_c).abs() < 0.2
            && (a.tread_center_c - b.tread_center_c).abs() < 0.2
            && (a.tread_outer_c - b.tread_outer_c).abs() < 0.2
            && (a.carcass_c - b.carcass_c).abs() < 0.2
            && (a.gas_c - b.gas_c).abs() < 0.2
    };
    assert!(cmp(&t.wheels[0], &t.wheels[1]), "FL must equal FR");
    assert!(cmp(&t.wheels[2], &t.wheels[3]), "RL must equal RR");
    // Rear cold setup differs from front: 105/140 vs 110/145.
    assert!(
        (t.wheels[0].pressure_kpa_gauge - t.wheels[2].pressure_kpa_gauge).abs() > 0.5,
        "front/rear pressure setups must remain distinct"
    );
    // Deflection symmetry too.
    let d_fl = sim.state.suspension.wheels[0].tire_deflection_m;
    let d_fr = sim.state.suspension.wheels[1].tire_deflection_m;
    let d_rl = sim.state.suspension.wheels[2].tire_deflection_m;
    let d_rr = sim.state.suspension.wheels[3].tire_deflection_m;
    assert!((d_fl - d_fr).abs() < 1e-6);
    assert!((d_rl - d_rr).abs() < 1e-6);
}

#[test]
fn wheelspin_heats_driven_rear_but_not_unloaded_front() {
    let cfg = VehicleConfig::f1_94_canonical();
    let p = &cfg.tire_pressure;
    let t = &cfg.tire_thermal.front;
    let env = TireEnvironment::fallback(t);
    let dt = 1.0 / 120.0;
    let mut sys = TireThermalSystem::new(p, t);

    // Driven rear wheel spinning: positive longitudinal slip + load.
    let spin = TireThermalInput {
        normal_force_n: 4000.0,
        longitudinal_force_n: 3600.0,
        lateral_force_n: 0.0,
        slip_velocity_long_ms: 4.0,
        slip_velocity_lat_ms: 0.0,
        tire_deflection_m: 0.008,
        tire_deflection_velocity_m_s: 0.0,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: 0.0,
        vehicle_speed_ms: 30.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [1.0, 2.0, 1.0],
    };
    for _ in 0..1800 {
        sys.step_after_forces(WheelIndex::RearLeft, p, t, env, spin, dt);
    }

    // Unloaded front wheel: no normal force -> no slip heat (airborne path).
    let idle = TireThermalInput {
        normal_force_n: 0.0,
        longitudinal_force_n: 0.0,
        lateral_force_n: 0.0,
        slip_velocity_long_ms: 0.0,
        slip_velocity_lat_ms: 0.0,
        tire_deflection_m: 0.0,
        tire_deflection_velocity_m_s: 0.0,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: 0.0,
        vehicle_speed_ms: 30.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [0.0, 0.0, 0.0],
    };
    for _ in 0..600 {
        sys.step_after_forces(WheelIndex::FrontLeft, p, t, env, idle, dt);
    }

    assert!(
        sys.wheels[2].average_tread_c() > sys.wheels[0].average_tread_c() + 10.0,
        "driven rear must heat far more than the unloaded front: {} vs {}",
        sys.wheels[2].average_tread_c(),
        sys.wheels[0].average_tread_c()
    );
}

#[test]
fn partial_tricast_outer_only_heats_outer_zone() {
    let cfg = VehicleConfig::f1_94_canonical();
    let p = &cfg.tire_pressure;
    let t = &cfg.tire_thermal.front;
    let env = TireEnvironment::fallback(t);
    let dt = 1.0 / 120.0;
    let mut sys = TireThermalSystem::new(p, t);

    let outer_only = TireThermalInput {
        normal_force_n: 4000.0,
        longitudinal_force_n: 2500.0,
        lateral_force_n: 1200.0,
        slip_velocity_long_ms: 3.0,
        slip_velocity_lat_ms: 2.0,
        tire_deflection_m: 0.008,
        tire_deflection_velocity_m_s: 0.0,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: 0.0,
        vehicle_speed_ms: 30.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [0.0, 0.0, 1.0],
    };
    for _ in 0..600 {
        sys.step_after_forces(WheelIndex::FrontLeft, p, t, env, outer_only, dt);
    }
    let w = &sys.wheels[0];
    assert!(
        w.tread_outer_c > w.tread_inner_c + 5.0,
        "supported outer zone must heat, unsupported inner stays indirect: out={} in={}",
        w.tread_outer_c,
        w.tread_inner_c
    );
    assert!(
        w.tread_center_c > w.tread_inner_c,
        "center must heat via lateral conduction"
    );
}

#[test]
fn airborne_no_road_heat_but_state_stays_finite() {
    let cfg = VehicleConfig::f1_94_canonical();
    let p = &cfg.tire_pressure;
    let t = &cfg.tire_thermal.front;
    let env = TireEnvironment::fallback(t);
    let dt = 1.0 / 120.0;
    let mut sys = TireThermalSystem::new(p, t);

    let airborne = TireThermalInput {
        normal_force_n: 0.0,
        longitudinal_force_n: 0.0,
        lateral_force_n: 0.0,
        slip_velocity_long_ms: 0.0,
        slip_velocity_lat_ms: 0.0,
        tire_deflection_m: 0.0,
        tire_deflection_velocity_m_s: 0.0,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: 0.0,
        vehicle_speed_ms: 50.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [0.0, 0.0, 0.0],
    };
    for _ in 0..600 {
        sys.step_after_forces(WheelIndex::FrontLeft, p, t, env, airborne, dt);
    }
    let w = &sys.wheels[0];
    for v in [w.tread_inner_c, w.tread_center_c, w.tread_outer_c, w.carcass_c, w.gas_c] {
        assert!(v.is_finite(), "airborne temps must stay finite");
        assert!((v - 25.0).abs() < 2.0, "no road heat airborne: {v}");
    }
}

#[test]
fn thermal_grip_cold_below_window_overheated_below_window() {
    let cfg = VehicleConfig::f1_94_canonical();
    let p = &cfg.tire_pressure;
    let t = &cfg.tire_thermal.front;
    let mut sys = TireThermalSystem::new(p, t);
    for i in 0..3 {
        sys.wheels[i].tread_inner_c = 20.0;
        sys.wheels[i].tread_center_c = 20.0;
        sys.wheels[i].tread_outer_c = 20.0;
    }
    let cold = sys.mechanical_modifiers(WheelIndex::FrontLeft, p, t).grip_scale;
    for i in 0..3 {
        sys.wheels[i].tread_inner_c = 95.0;
        sys.wheels[i].tread_center_c = 95.0;
        sys.wheels[i].tread_outer_c = 95.0;
    }
    let optimal = sys.mechanical_modifiers(WheelIndex::FrontRight, p, t).grip_scale;
    for i in 0..3 {
        sys.wheels[i].tread_inner_c = 145.0;
        sys.wheels[i].tread_center_c = 145.0;
        sys.wheels[i].tread_outer_c = 145.0;
    }
    let overheated = sys.mechanical_modifiers(WheelIndex::RearLeft, p, t).grip_scale;
    assert!(cold < optimal, "cold grip must be below operating window");
    assert!(overheated < optimal, "overheated grip must fall below operating window");
    assert!(cold < overheated, "cold must be the weakest of the three");
}

#[test]
fn reset_returns_all_wheels_to_cold_setup() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    let dt = 1.0 / 120.0;
    let input = VehicleInput { throttle: 1.0, brake: 1.0, ..VehicleInput::default() };
    for _ in 0..1200 {
        let samples = flat_samples(&sim);
        sim.step(&input, &samples, dt);
    }
    // Tires should have warmed by now.
    assert!(sim.state.tire_thermal.wheels[0].average_tread_c() > 26.0);

    let ptr: *mut std::ffi::c_void =
        (&mut sim as *mut VehicleSimulator) as *mut std::ffi::c_void;
    unsafe {
        vehicle_physics_engine::f1_94_physics_reset(ptr, 0.0, spawn, 0.0, 0.0);
    }
    for w in WheelIndex::ALL {
        let st = &sim.state.tire_thermal.wheels[w as usize];
        assert!(
            (st.pressure_kpa_gauge - cfg.tire_pressure.cold_kpa_gauge[w as usize]).abs() < 1e-9,
            "reset must restore cold pressure for wheel {w:?}"
        );
        assert!((st.tread_inner_c - 25.0).abs() < 1e-9);
        assert!((st.tread_center_c - 25.0).abs() < 1e-9);
        assert!((st.tread_outer_c - 25.0).abs() < 1e-9);
        assert!((st.carcass_c - 25.0).abs() < 1e-9);
        assert!((st.gas_c - 25.0).abs() < 1e-9);
    }
}

#[test]
fn ffi_telemetry_carries_tire_state() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);

    let mut samples = [FfiTriRaycastSample {
        inner: FfiRaycastHit {
            is_colliding: false,
            distance: 0.0,
            point_x: 0.0,
            point_y: 0.0,
            point_z: 0.0,
            normal_x: 0.0,
            normal_y: 1.0,
            normal_z: 0.0,
            surface_type: 0,
        },
        center: FfiRaycastHit {
            is_colliding: false,
            distance: 0.0,
            point_x: 0.0,
            point_y: 0.0,
            point_z: 0.0,
            normal_x: 0.0,
            normal_y: 1.0,
            normal_z: 0.0,
            surface_type: 0,
        },
        outer: FfiRaycastHit {
            is_colliding: false,
            distance: 0.0,
            point_x: 0.0,
            point_y: 0.0,
            point_z: 0.0,
            normal_x: 0.0,
            normal_y: 1.0,
            normal_z: 0.0,
            surface_type: 0,
        },
    }; 4];
    {
        let rust_samples = flat_samples(&sim);
        for (i, rs) in rust_samples.iter().enumerate() {
            samples[i] = FfiTriRaycastSample {
                inner: FfiRaycastHit {
                    is_colliding: rs.inner.is_colliding,
                    distance: rs.inner.distance,
                    point_x: rs.inner.point.x,
                    point_y: rs.inner.point.y,
                    point_z: rs.inner.point.z,
                    normal_x: rs.inner.normal.x,
                    normal_y: rs.inner.normal.y,
                    normal_z: rs.inner.normal.z,
                    surface_type: 0,
                },
                center: FfiRaycastHit {
                    is_colliding: rs.center.is_colliding,
                    distance: rs.center.distance,
                    point_x: rs.center.point.x,
                    point_y: rs.center.point.y,
                    point_z: rs.center.point.z,
                    normal_x: rs.center.normal.x,
                    normal_y: rs.center.normal.y,
                    normal_z: rs.center.normal.z,
                    surface_type: 0,
                },
                outer: FfiRaycastHit {
                    is_colliding: rs.outer.is_colliding,
                    distance: rs.outer.distance,
                    point_x: rs.outer.point.x,
                    point_y: rs.outer.point.y,
                    point_z: rs.outer.point.z,
                    normal_x: rs.outer.normal.x,
                    normal_y: rs.outer.normal.y,
                    normal_z: rs.outer.normal.z,
                    surface_type: 0,
                },
            };
        }
    }
    let mut body = FfiBodyKinematics {
        pos_x: 0.0,
        pos_y: spawn,
        pos_z: 0.0,
        rot_quat_x: 0.0,
        rot_quat_y: 0.0,
        rot_quat_z: 0.0,
        rot_quat_w: 1.0,
        lin_vel_x: 0.0,
        lin_vel_y: 0.0,
        lin_vel_z: 0.0,
        ang_vel_x: 0.0,
        ang_vel_y: 0.0,
        ang_vel_z: 0.0,
    };
    let input = FfiVehicleInput {
        throttle: 0.5,
        steering: 0.0,
        brake: 0.0,
        handbrake: 0.0,
        clutch: 0.0,
        gear_request: -2,
    };
    let mut forces = FfiForceTorqueOutput {
        force_x: 0.0,
        force_y: 0.0,
        force_z: 0.0,
        torque_x: 0.0,
        torque_y: 0.0,
        torque_z: 0.0,
    };
    let mut telem = unsafe { std::mem::zeroed::<FfiTelemetryOutput>() };
    let sim_ptr: *mut std::ffi::c_void = (&mut sim as *mut VehicleSimulator) as *mut std::ffi::c_void;
    unsafe {
        vehicle_physics_engine::f1_94_physics_solve_forces(
            sim_ptr,
            &body as *const FfiBodyKinematics,
            &input as *const FfiVehicleInput,
            samples.as_ptr(),
            1.0 / 120.0,
            &mut forces as *mut FfiForceTorqueOutput,
            &mut telem as *mut FfiTelemetryOutput,
        );
    }
    // One solve tick leaves the gas essentially cold but the SETTLED carcass
    // deflection applies the small volume correction (cold gauge + ~1.5 kPa at
    // rest). Per-wheel cold setups must remain distinct and in plausible ranges.
    assert!(
        telem.fl_pressure_kpa > 110.0 && telem.fl_pressure_kpa < 113.0,
        "FL settled near 110 kPa + deflection correction: {}",
        telem.fl_pressure_kpa
    );
    assert!(
        telem.rl_pressure_kpa > 105.0 && telem.rl_pressure_kpa < 108.0,
        "RL settled near 105 kPa + deflection correction: {}",
        telem.rl_pressure_kpa
    );
    assert!(telem.fl_pressure_kpa > telem.rl_pressure_kpa);
    assert!((telem.fl_tread_center_c - 25.0).abs() < 0.5);
    assert!((telem.rr_gas_c - 25.0).abs() < 0.5);
    assert!(telem.fl_tread_inner_c.is_finite());
    assert!(telem.rl_carcass_c.is_finite());
}

#[test]
fn thermal_step_is_nan_safe_under_extreme_inputs() {
    let cfg = VehicleConfig::f1_94_canonical();
    let p = &cfg.tire_pressure;
    let t = &cfg.tire_thermal.front;
    let env = TireEnvironment::fallback(t);
    let mut sys = TireThermalSystem::new(p, t);
    let brutal = TireThermalInput {
        normal_force_n: f64::MAX / 1e10,
        longitudinal_force_n: f64::MAX / 1e10,
        lateral_force_n: -f64::MAX / 1e10,
        slip_velocity_long_ms: 1e9,
        slip_velocity_lat_ms: -1e9,
        tire_deflection_m: 1e9,
        tire_deflection_velocity_m_s: -1e9,
        max_tire_deflection_m: 0.0,
        dynamic_camber_rad: 10.0,
        vehicle_speed_ms: 1e9,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [1.0, 2.0, 1.0],
    };
    for _ in 0..120 {
        sys.step_after_forces(WheelIndex::FrontLeft, p, t, env, brutal, 1.0 / 120.0);
    }
    let w = &sys.wheels[0];
    for v in [
        w.tread_inner_c,
        w.tread_center_c,
        w.tread_outer_c,
        w.carcass_c,
        w.gas_c,
        w.pressure_kpa_gauge,
    ] {
        assert!(v.is_finite(), "thermal state must stay finite: {v}");
    }
    assert!(
        w.pressure_kpa_gauge >= t.minimum_pressure_kpa_gauge
            && w.pressure_kpa_gauge <= t.maximum_pressure_kpa_gauge,
        "pressure must stay clamped"
    );
}

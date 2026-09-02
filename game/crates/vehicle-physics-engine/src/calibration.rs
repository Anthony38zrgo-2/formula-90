// BASE-000 calibration harnesses.
//
// `steady_tire_force` evaluates one tire at a fixed load / slip angle / slip
// ratio by relaxing the carcass to steady state at constant forward speed,
// without Godot or the rest of the vehicle. It is the single source for
// pure-slip and combined-slip sweep data produced by `tire_sweep`.
use crate::tire::TireSystem;
use crate::types::{SurfaceType, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;

/// Steady-state single-tire force point at a fixed load, slip angle and slip ratio.
#[derive(Debug, Clone, Copy, Default)]
pub struct TireForcePoint {
    pub normal_force_n: f64,
    pub slip_angle_rad: f64,
    pub slip_ratio: f64,
    pub effective_slip_angle_rad: f64,
    pub effective_slip_ratio: f64,
    pub lateral_force_n: f64,
    pub longitudinal_force_n: f64,
    pub rolling_resistance_n: f64,
    pub aligning_torque_nm: f64,
}

const SWEEP_FORWARD_SPEED_MS: f64 = 10.0;
const SWEEP_SETTLE_SUBSTEPS: usize = 240;
const SWEEP_DT: f64 = 1.0 / 120.0;

fn roll_velocity_for_kappa(kappa: f64) -> f64 {
    let k = kappa.clamp(-0.99, 0.99);
    if k > 0.0 {
        SWEEP_FORWARD_SPEED_MS / (1.0 - k)
    } else {
        SWEEP_FORWARD_SPEED_MS * (1.0 + k)
    }
}

/// Evaluates a single wheel at steady state for the requested load `fz`, slip
/// angle `alpha_rad` and slip ratio `kappa` on flat Road.
///
/// The wheel velocity is derived so the geometric slip targets match the
/// requested inputs exactly, then the carcass is relaxed for 2 seconds of
/// constant joint-speed travel (240 substeps at 120 Hz) before sampling.
pub fn steady_tire_force(
    config: &VehicleConfig,
    wheel: WheelIndex,
    fz: f64,
    alpha_rad: f64,
    kappa: f64,
) -> TireForcePoint {
    let mut tires = TireSystem::new(config);
    let v = SWEEP_FORWARD_SPEED_MS;
    let v_lat = -(alpha_rad.tan()) * v;
    let roll_velocity = roll_velocity_for_kappa(kappa);

    tires.set_mechanical_state(config, wheel, fz, 0.0, 0.0, 1.0);
    let radius = tires.wheels[wheel as usize].effective_rolling_radius.max(0.05);
    tires.wheels[wheel as usize].spin = roll_velocity / radius;
    tires.wheels[wheel as usize].camber_rad = 0.0;

    let friction = config
        .surface_friction
        .get(&SurfaceType::Road)
        .copied()
        .unwrap_or(1.0);
    let stiffness = config
        .surface_stiffness
        .get(&SurfaceType::Road)
        .copied()
        .unwrap_or(5.0);
    let rolling_resistance = config
        .surface_rolling_resistance
        .get(&SurfaceType::Road)
        .copied()
        .unwrap_or(1.0);

    let local_wheel_velocity = Vec3::new(v_lat, 0.0, -v);
    for _ in 0..SWEEP_SETTLE_SUBSTEPS {
        tires.process_wheel_forces(
            config,
            wheel,
            fz,
            SurfaceType::Road,
            friction,
            stiffness,
            rolling_resistance,
            false,
            local_wheel_velocity,
            SWEEP_DT,
        );
    }

    let t = &tires.wheels[wheel as usize];
    TireForcePoint {
        normal_force_n: fz,
        slip_angle_rad: t.slip_angle_rad,
        slip_ratio: t.slip_ratio,
        effective_slip_angle_rad: t.effective_slip_angle_rad,
        effective_slip_ratio: t.effective_slip_ratio,
        lateral_force_n: t.lateral_force,
        longitudinal_force_n: t.longitudinal_force,
        rolling_resistance_n: t.rolling_resistance,
        aligning_torque_nm: t.aligning_torque,
    }
}

/// Static reference load for a wheel, exported so sweep grids can be built
/// around multiples of the normal weight carried by each axle.
pub fn reference_load_n(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    crate::wheel_mechanics::static_wheel_load(config, wheel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle_config::VehicleConfig;

    fn test_config() -> VehicleConfig {
        VehicleConfig::f1_94_canonical()
    }

    #[test]
    fn steady_lateral_force_is_antisymmetric() {
        let cfg = test_config();
        let fz = reference_load_n(&cfg, WheelIndex::FrontLeft);
        let pos = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, 0.1, 0.0);
        let neg = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, -0.1, 0.0);
        assert!((pos.lateral_force_n + neg.lateral_force_n).abs() < pos.lateral_force_n * 1e-3);
        assert!(pos.lateral_force_n.abs() > 100.0);
    }

    #[test]
    fn steady_slips_match_requested_targets() {
        let cfg = test_config();
        let fz = reference_load_n(&cfg, WheelIndex::FrontLeft);
        let p = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, 0.05, -0.2);
        assert!((p.slip_angle_rad - 0.05).abs() < 1e-3);
        assert!((p.slip_ratio - (-0.2)).abs() < 1e-3);
        assert!((p.effective_slip_angle_rad - 0.05).abs() < 5e-3);
        assert!((p.effective_slip_ratio - (-0.2)).abs() < 5e-3);
    }

    #[test]
    fn grid_is_finite_and_sign_correct() {
        let cfg = test_config();
        let fz = reference_load_n(&cfg, WheelIndex::FrontLeft);
        for alpha in [-0.30, -0.15, -0.05, 0.05, 0.15, 0.30] {
            for kappa in [-0.6, -0.2, 0.0, 0.2, 0.6] {
                let p = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, alpha, kappa);
                for v in [
                    p.lateral_force_n,
                    p.longitudinal_force_n,
                    p.aligning_torque_nm,
                    p.rolling_resistance_n,
                ] {
                    assert!(v.is_finite(), "non-finite at alpha={alpha} kappa={kappa}");
                }
                assert_eq!(p.lateral_force_n.signum(), alpha.signum(), "alpha={alpha}");
            }
        }
    }

    #[test]
    fn lateral_slope_is_positive_near_zero() {
        let cfg = test_config();
        let fz = reference_load_n(&cfg, WheelIndex::FrontLeft);
        let p = steady_tire_force(&cfg, WheelIndex::FrontLeft, fz, 0.02, 0.0);
        assert!(p.lateral_force_n > 0.0);
    }
}

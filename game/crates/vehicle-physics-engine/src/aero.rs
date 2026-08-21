use crate::types::Vec3;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WingPolarPoint {
    pub angle_deg: f64,
    pub cl: f64,
    pub cd: f64,
}
impl Default for WingPolarPoint {
    fn default() -> Self {
        Self {
            angle_deg: 0.0,
            cl: 0.0,
            cd: 0.05,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WingAeroConfig {
    pub area_m2: f64,
    pub incidence_deg: f64,
    pub min_angle_deg: f64,
    pub max_angle_deg: f64,
    pub yaw_decay_exponent: f64,
    pub flex_coefficient: f64,
    pub polar: Vec<WingPolarPoint>,
}
impl WingAeroConfig {
    fn front_default() -> Self {
        Self {
            area_m2: 0.72,
            incidence_deg: 12.0,
            min_angle_deg: 4.0,
            max_angle_deg: 20.0,
            yaw_decay_exponent: 1.15,
            flex_coefficient: 0.0008,
            polar: vec![
                WingPolarPoint {
                    angle_deg: 4.0,
                    cl: 0.45,
                    cd: 0.075,
                },
                WingPolarPoint {
                    angle_deg: 8.0,
                    cl: 0.82,
                    cd: 0.105,
                },
                WingPolarPoint {
                    angle_deg: 12.0,
                    cl: 1.12,
                    cd: 0.155,
                },
                WingPolarPoint {
                    angle_deg: 16.0,
                    cl: 1.30,
                    cd: 0.230,
                },
                WingPolarPoint {
                    angle_deg: 20.0,
                    cl: 1.20,
                    cd: 0.340,
                },
            ],
        }
    }
    fn rear_default() -> Self {
        Self {
            area_m2: 0.84,
            incidence_deg: 16.0,
            min_angle_deg: 6.0,
            max_angle_deg: 26.0,
            yaw_decay_exponent: 1.05,
            flex_coefficient: 0.0010,
            polar: vec![
                WingPolarPoint {
                    angle_deg: 6.0,
                    cl: 0.55,
                    cd: 0.090,
                },
                WingPolarPoint {
                    angle_deg: 10.0,
                    cl: 0.90,
                    cd: 0.130,
                },
                WingPolarPoint {
                    angle_deg: 16.0,
                    cl: 1.35,
                    cd: 0.220,
                },
                WingPolarPoint {
                    angle_deg: 21.0,
                    cl: 1.52,
                    cd: 0.320,
                },
                WingPolarPoint {
                    angle_deg: 26.0,
                    cl: 1.38,
                    cd: 0.470,
                },
            ],
        }
    }
}
impl Default for WingAeroConfig {
    fn default() -> Self {
        Self::front_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnderfloorAeroConfig {
    pub lift_area_m2: f64,
    pub optimal_height_m: f64,
    pub choke_height_m: f64,
    pub high_height_m: f64,
    pub optimal_rake_deg: f64,
    pub rake_window_deg: f64,
    pub optimal_expansion_deg: f64,
    pub expansion_window_deg: f64,
    pub yaw_decay_exponent: f64,
    pub stall_attack_tau_s: f64,
    pub stall_recovery_tau_s: f64,
    pub induced_drag_ratio: f64,
}
impl Default for UnderfloorAeroConfig {
    fn default() -> Self {
        Self {
            lift_area_m2: 0.90,
            optimal_height_m: 0.045,
            choke_height_m: 0.012,
            high_height_m: 0.160,
            optimal_rake_deg: 0.5,
            rake_window_deg: 2.5,
            optimal_expansion_deg: 3.0,
            expansion_window_deg: 5.0,
            yaw_decay_exponent: 1.8,
            stall_attack_tau_s: 0.030,
            stall_recovery_tau_s: 0.160,
            induced_drag_ratio: 0.16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EraAeroLimits {
    pub profile: String,
    pub soft_max_load_ratio: f64,
    pub hard_max_load_ratio: f64,
    pub hard_max_downforce_n: f64,
}
impl Default for EraAeroLimits {
    fn default() -> Self {
        Self {
            profile: "f1_1994_post_safety".into(),
            soft_max_load_ratio: 1.65,
            hard_max_load_ratio: 1.85,
            hard_max_downforce_n: 10_800.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AeroModelConfig {
    pub front_wing: WingAeroConfig,
    pub rear_wing: WingAeroConfig,
    pub underfloor: UnderfloorAeroConfig,
    pub limits: EraAeroLimits,
}
impl Default for AeroModelConfig {
    fn default() -> Self {
        Self {
            front_wing: WingAeroConfig::front_default(),
            rear_wing: WingAeroConfig::rear_default(),
            underfloor: UnderfloorAeroConfig::default(),
            limits: EraAeroLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AeroEnvironment {
    pub clearance_m: [f64; 5],
    pub valid_mask: u32,
    pub rake_rad: f64,
    pub roll_rad: f64,
    pub bottoming_mask: u32,
    pub contact_confidence: f64,
}
impl Default for AeroEnvironment {
    fn default() -> Self {
        Self {
            clearance_m: [0.09, 0.09, 0.095, 0.130, 0.130],
            valid_mask: 0x1f,
            rake_rad: 0.0,
            roll_rad: 0.0,
            bottoming_mask: 0,
            contact_confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AeroForces {
    pub total_downforce: f64,
    pub front_downforce: f64,
    pub diffuser_downforce: f64,
    pub rear_downforce: f64,
    pub drag_force: f64,
    pub effective_cl: f64,
    #[serde(default = "one")]
    pub yaw_decay_factor: f64,
    #[serde(default)]
    pub blend_factor: f64,
    #[serde(default = "one")]
    pub flex_factor: f64,
    #[serde(default)]
    pub front_wing_angle_deg: f64,
    #[serde(default)]
    pub rear_wing_angle_deg: f64,
    #[serde(default)]
    pub front_wing_cl: f64,
    #[serde(default)]
    pub rear_wing_cl: f64,
    #[serde(default)]
    pub floor_height_factor: f64,
    #[serde(default)]
    pub floor_rake_factor: f64,
    #[serde(default)]
    pub floor_seal_factor: f64,
    #[serde(default)]
    pub diffuser_expansion_deg: f64,
    #[serde(default)]
    pub diffuser_stall_factor: f64,
    #[serde(default)]
    pub raw_downforce: f64,
    #[serde(default = "one")]
    pub global_limit_factor: f64,
    #[serde(default)]
    pub load_ratio: f64,
    #[serde(default)]
    pub balance_front: f64,
}

impl AeroForces {
    pub fn zero() -> Self {
        Self {
            total_downforce: 0.0,
            front_downforce: 0.0,
            diffuser_downforce: 0.0,
            rear_downforce: 0.0,
            drag_force: 0.0,
            effective_cl: 0.0,
            yaw_decay_factor: 1.0,
            blend_factor: 0.0,
            flex_factor: 1.0,
            front_wing_angle_deg: 0.0,
            rear_wing_angle_deg: 0.0,
            front_wing_cl: 0.0,
            rear_wing_cl: 0.0,
            floor_height_factor: 0.0,
            floor_rake_factor: 0.0,
            floor_seal_factor: 0.0,
            diffuser_expansion_deg: 0.0,
            diffuser_stall_factor: 1.0,
            raw_downforce: 0.0,
            global_limit_factor: 1.0,
            load_ratio: 0.0,
            balance_front: 0.45,
        }
    }

    /// Compatibility path for callers without ride-height probes.
    pub fn step(&mut self, config: &VehicleConfig, local_velocity: Vec3, dt: f64) {
        let v_fwd = (-local_velocity.z).max(0.0);
        let v_lat = local_velocity.x.abs();
        let range = (config.aero_blend_full_speed - config.aero_blend_min_speed).max(1e-6);
        let x = ((v_fwd - config.aero_blend_min_speed) / range).clamp(0.0, 1.0);
        let blend = 3.0 * x * x - 2.0 * x * x * x;
        let yaw = (v_fwd / (v_fwd * v_fwd + v_lat * v_lat + 1e-6).sqrt())
            .clamp(0.0, 1.0)
            .powf(config.aero_yaw_decay_exponent);
        let flex = 1.0 / (1.0 + config.aero_flex_coefficient.max(0.0) * v_fwd);
        let q = 0.5 * config.air_density.max(0.0) * v_fwd * v_fwd;
        let target = q
            * config.frontal_area.max(0.0)
            * config.coefficient_of_downforce.max(0.0)
            * blend
            * yaw
            * flex;
        let drag_target = q * config.frontal_area.max(0.0) * config.coefficient_of_drag.max(0.0);
        let alpha = (dt / config.aero_lag_tau.max(1e-4)).clamp(0.0, 1.0);
        self.total_downforce += alpha * (target - self.total_downforce);
        self.drag_force += alpha * (drag_target - self.drag_force);
        self.front_downforce = self.total_downforce * config.aero_split_front;
        self.diffuser_downforce = self.total_downforce * config.aero_split_diffuser;
        self.rear_downforce = self.total_downforce * config.aero_split_rear;
        self.effective_cl = config.coefficient_of_downforce * blend * yaw * flex;
        self.yaw_decay_factor = yaw;
        self.blend_factor = blend;
        self.flex_factor = flex;
        self.raw_downforce = target;
        self.global_limit_factor = 1.0;
        self.load_ratio = self.total_downforce / (config.vehicle_mass * 9.81).max(1.0);
        self.balance_front = config.aero_split_front;
    }

    pub fn step_with_environment(
        &mut self,
        config: &VehicleConfig,
        local_velocity: Vec3,
        env: &AeroEnvironment,
        pitch_rad: f64,
        dt: f64,
    ) {
        let v_fwd = (-local_velocity.z).max(0.0);
        let v_lat = local_velocity.x.abs();
        let denom = (config.aero_blend_full_speed - config.aero_blend_min_speed).max(1e-6);
        let x = ((v_fwd - config.aero_blend_min_speed) / denom).clamp(0.0, 1.0);
        let blend = 3.0 * x * x - 2.0 * x * x * x;
        let cos_yaw = (v_fwd / (v_fwd * v_fwd + v_lat * v_lat + 1e-6).sqrt()).clamp(0.0, 1.0);
        let q = 0.5 * config.air_density.max(0.0) * v_fwd * v_fwd;
        let m = &config.aero_model;
        let pitch_deg = pitch_rad.to_degrees();
        let fw_angle = (m.front_wing.incidence_deg + pitch_deg)
            .clamp(m.front_wing.min_angle_deg, m.front_wing.max_angle_deg);
        let rw_angle = (m.rear_wing.incidence_deg + pitch_deg)
            .clamp(m.rear_wing.min_angle_deg, m.rear_wing.max_angle_deg);
        let (fw_cl, fw_cd) = polar_at(&m.front_wing.polar, fw_angle);
        let (rw_cl, rw_cd) = polar_at(&m.rear_wing.polar, rw_angle);
        let fw_yaw = cos_yaw.powf(m.front_wing.yaw_decay_exponent.max(0.0));
        let rw_yaw = cos_yaw.powf(m.rear_wing.yaw_decay_exponent.max(0.0));
        let fw_flex = 1.0 / (1.0 + m.front_wing.flex_coefficient.max(0.0) * v_fwd);
        let rw_flex = 1.0 / (1.0 + m.rear_wing.flex_coefficient.max(0.0) * v_fwd);
        let fw_target =
            q * m.front_wing.area_m2.max(0.0) * fw_cl.max(0.0) * fw_yaw * fw_flex * blend;
        let rw_target =
            q * m.rear_wing.area_m2.max(0.0) * rw_cl.max(0.0) * rw_yaw * rw_flex * blend;
        let f = &m.underfloor;
        let confidence = env.valid_mask.count_ones().min(5) as f64 / 5.0;
        let floor_h = (0.5 * (env.clearance_m[0] + env.clearance_m[1])
            + env.clearance_m[2]
            + env.clearance_m[3])
            / 3.0;
        let height_factor = height_efficiency(floor_h, f);
        let rake_factor = (1.0
            - (env.rake_rad.to_degrees() - f.optimal_rake_deg).abs() / f.rake_window_deg.max(0.1))
        .clamp(0.15, 1.0);
        let expansion_deg = ((env.clearance_m[4] - env.clearance_m[3]) / 0.80)
            .atan()
            .to_degrees();
        let expansion_factor = (1.0
            - (expansion_deg - f.optimal_expansion_deg).abs() / f.expansion_window_deg.max(0.1))
        .clamp(0.10, 1.0);
        let asymmetry = (env.clearance_m[0] - env.clearance_m[1]).abs();
        let seal_factor = (1.0 - asymmetry / 0.080 - env.roll_rad.abs() / 0.12).clamp(0.15, 1.0);
        let contact_factor = if env.bottoming_mask != 0 { 0.20 } else { 1.0 };
        let target_flow = height_factor
            * rake_factor
            * expansion_factor
            * seal_factor
            * confidence
            * cos_yaw.powf(f.yaw_decay_exponent.max(0.0))
            * contact_factor;
        let flow_tau = if target_flow < self.diffuser_stall_factor {
            f.stall_attack_tau_s
        } else {
            f.stall_recovery_tau_s
        }
        .max(1e-4);
        self.diffuser_stall_factor +=
            (target_flow - self.diffuser_stall_factor) * (dt / flow_tau).clamp(0.0, 1.0);
        let floor_target = q * f.lift_area_m2.max(0.0) * self.diffuser_stall_factor * blend;
        let raw = fw_target + floor_target + rw_target;
        let weight = (config.vehicle_mass * 9.81).max(1.0);
        let soft = (weight * m.limits.soft_max_load_ratio.max(0.0))
            .min(m.limits.hard_max_downforce_n.max(0.0));
        let hard = (weight
            * m.limits
                .hard_max_load_ratio
                .max(m.limits.soft_max_load_ratio))
        .min(m.limits.hard_max_downforce_n.max(soft + 1.0));
        let limited = soft_limit(raw, soft, hard);
        let limit_factor = if raw > 1e-6 { limited / raw } else { 1.0 };
        let alpha = (dt / config.aero_lag_tau.max(1e-4)).clamp(0.0, 1.0);
        self.front_downforce += alpha * (fw_target * limit_factor - self.front_downforce);
        self.diffuser_downforce += alpha * (floor_target * limit_factor - self.diffuser_downforce);
        self.rear_downforce += alpha * (rw_target * limit_factor - self.rear_downforce);
        self.total_downforce = self.front_downforce + self.diffuser_downforce + self.rear_downforce;
        let body_drag = q * config.coefficient_of_drag.max(0.0) * config.frontal_area.max(0.0);
        let wing_drag =
            q * (m.front_wing.area_m2 * fw_cd.max(0.0) + m.rear_wing.area_m2 * rw_cd.max(0.0));
        self.drag_force += alpha
            * (body_drag + wing_drag + floor_target * f.induced_drag_ratio.max(0.0) * limit_factor
                - self.drag_force);
        self.effective_cl = if q * config.frontal_area > 1e-6 {
            self.total_downforce / (q * config.frontal_area)
        } else {
            0.0
        };
        self.yaw_decay_factor = cos_yaw;
        self.blend_factor = blend;
        self.flex_factor = 0.5 * (fw_flex + rw_flex);
        self.front_wing_angle_deg = fw_angle;
        self.rear_wing_angle_deg = rw_angle;
        self.front_wing_cl = fw_cl;
        self.rear_wing_cl = rw_cl;
        self.floor_height_factor = height_factor;
        self.floor_rake_factor = rake_factor;
        self.floor_seal_factor = seal_factor;
        self.diffuser_expansion_deg = expansion_deg;
        self.raw_downforce = raw;
        self.global_limit_factor = limit_factor;
        self.load_ratio = self.total_downforce / weight;
        self.balance_front = if self.total_downforce > 1e-6 {
            self.front_downforce / self.total_downforce
        } else {
            0.45
        };
    }
}

fn polar_at(p: &[WingPolarPoint], a: f64) -> (f64, f64) {
    if p.is_empty() {
        return (0.0, 0.0);
    }
    if a <= p[0].angle_deg {
        return (p[0].cl, p[0].cd);
    }
    for w in p.windows(2) {
        if a <= w[1].angle_deg {
            let t = ((a - w[0].angle_deg) / (w[1].angle_deg - w[0].angle_deg).max(1e-6))
                .clamp(0.0, 1.0);
            return (
                w[0].cl + (w[1].cl - w[0].cl) * t,
                w[0].cd + (w[1].cd - w[0].cd) * t,
            );
        }
    }
    let x = p[p.len() - 1];
    (x.cl, x.cd)
}
fn height_efficiency(h: f64, c: &UnderfloorAeroConfig) -> f64 {
    if h <= 0.0 {
        return 0.0;
    }
    if h < c.choke_height_m {
        return 0.20 * h / c.choke_height_m.max(1e-4);
    }
    if h < c.optimal_height_m {
        let t = (h - c.choke_height_m) / (c.optimal_height_m - c.choke_height_m).max(1e-4);
        return 0.20 + 0.80 * t.clamp(0.0, 1.0);
    }
    let t = (h - c.optimal_height_m) / (c.high_height_m - c.optimal_height_m).max(1e-4);
    (1.0 - 0.70 * t.clamp(0.0, 1.0)).max(0.20)
}
fn soft_limit(raw: f64, soft: f64, hard: f64) -> f64 {
    if raw <= soft {
        return raw.max(0.0);
    }
    let range = (hard - soft).max(1.0);
    soft + range * (1.0 - (-(raw - soft) / range).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polar_interpolates_and_clamps() {
        let p = WingAeroConfig::front_default().polar;
        assert_eq!(polar_at(&p, -10.0), (0.45, 0.075));
        assert!((polar_at(&p, 10.0).0 - 0.97).abs() < 1e-9);
        assert_eq!(polar_at(&p, 40.0), (1.20, 0.340));
    }

    #[test]
    fn era_limiter_never_exceeds_hard_cap() {
        assert!(soft_limit(100_000.0, 9_600.0, 10_800.0) <= 10_800.0);
        assert_eq!(soft_limit(5_000.0, 9_600.0, 10_800.0), 5_000.0);
        let cfg = VehicleConfig::f1_94_canonical();
        let mut aero = AeroForces::zero();
        for _ in 0..100 {
            aero.step_with_environment(
                &cfg,
                Vec3::new(0.0, 0.0, -100.0),
                &AeroEnvironment::default(),
                0.0,
                1.0 / 120.0,
            );
        }
        let hard = (cfg.vehicle_mass * 9.81 * cfg.aero_model.limits.hard_max_load_ratio)
            .min(cfg.aero_model.limits.hard_max_downforce_n);
        assert!(aero.total_downforce <= hard + 1e-6);
    }

    #[test]
    fn floor_chokes_near_ground() {
        let c = UnderfloorAeroConfig::default();
        assert!(height_efficiency(0.004, &c) < height_efficiency(c.optimal_height_m, &c));
    }

    #[test]
    fn localized_bottoming_collapses_floor_before_wings() {
        let cfg = VehicleConfig::f1_94_canonical();
        let velocity = Vec3::new(0.0, 0.0, -55.0);
        let mut clear = AeroForces::zero();
        let mut bottoming = AeroForces::zero();
        let mut hit = AeroEnvironment::default();
        hit.bottoming_mask = 1;
        for _ in 0..80 {
            clear.step_with_environment(
                &cfg,
                velocity,
                &AeroEnvironment::default(),
                0.0,
                1.0 / 120.0,
            );
            bottoming.step_with_environment(&cfg, velocity, &hit, 0.0, 1.0 / 120.0);
        }
        assert!(bottoming.diffuser_downforce < clear.diffuser_downforce * 0.45);
        assert!((bottoming.front_wing_cl - clear.front_wing_cl).abs() < 1e-9);
        assert!((bottoming.rear_wing_cl - clear.rear_wing_cl).abs() < 1e-9);
    }
}

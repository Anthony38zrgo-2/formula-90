//! SUS-GEO-02: versioned physical suspension schema.
//! Units N/Ns in field names keep SI capitals by contract (JSON keys).
#![allow(non_snake_case)]
//!
//! Units: metres, seconds, kilograms, newtons, radians.
//! Chassis-local frame: +X right, +Y up, -Z forward (Godot). Origin centred
//! between axles at mean wheel-centre height (same as `wheel_anchor_local`).
//! Left corners use x < 0, right corners x > 0. A `mirror_of` corner negates
//! every x coordinate of its base (y/z preserved).
//!
//! Travel conventions (contract for SUS-GEO-03/04):
//! - `q`: wheel travel relative to design rest (`hub_center`), metres.
//!   Positive = bump (hub moves +Y), negative = rebound/droop.
//! - `s`: elastic-element compression, metres. `s = L_rest - L_current` where
//!   `L` is the damper/spring length along its axis. Bump shortens the
//!   damper, so `s > 0` on bump. Motion ratio `r = ds/dq` (dimensionless,
//!   expected > 0). Generalised wheel force from the spring is `F_s * r` via
//!   virtual work; the applied force opposes the displacement/velocity.
//! - Tangent wheel rate for a linear spring with variable geometry:
//!   `k_wheel = k_s * r^2 + F_s * dr/dq`. Viscous wheel damping for one
//!   coordinate: `c_wheel = c_s * r^2`. These do not replace preload,
//!   bump-stops, couplings or other coordinates (see instrucciones 4.2).
//!
//! Scope: rigid links, ideal joints. No arm elasticity, play, ball-joint
//! friction, buckling or failure. A low-mounted pullrod does NOT lower the
//! centre of gravity by itself; that needs explicit mass distribution.
//! `rod_kind_label` (push/pull) is informational only and must never affect
//! forces: two mechanisms with identical hardpoints and element properties
//! must respond identically.

use crate::types::{Vec3, WheelIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model selector. Explicit, versioned. Absent in JSON defaults to legacy so
/// existing profiles keep byte-identical behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SuspensionModelKind {
    #[default]
    Legacy1Dof,
    Geometric,
}

impl SuspensionModelKind {
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "legacy_1dof" | "legacy" => Some(Self::Legacy1Dof),
            "geometric" => Some(Self::Geometric),
            _ => None,
        }
    }
    pub fn as_str_name(self) -> &'static str {
        match self {
            Self::Legacy1Dof => "legacy_1dof",
            Self::Geometric => "geometric",
        }
    }
}

pub fn default_suspension_model_version() -> u32 {
    1
}

/// Where the push/pull rod attaches. Independent of the push/pull label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RodMount {
    #[default]
    Lower,
    Upper,
}

/// Informational only. Must never influence forces (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RodKindLabel {
    #[default]
    Pushrod,
    Pullrod,
}

impl RodKindLabel {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "pushrod" | "push" => Some(Self::Pushrod),
            "pullrod" | "pull" => Some(Self::Pullrod),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CornerProvenanceOrigin {
    #[default]
    Measured,
    Reconstructed,
    Mirrored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CornerProvenance {
    #[serde(default)]
    pub origin: CornerProvenanceOrigin,
    #[serde(default)]
    pub note: String,
}

impl Default for CornerProvenance {
    fn default() -> Self {
        Self {
            origin: CornerProvenanceOrigin::Measured,
            note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WishbonePoints {
    pub inner_front: Vec3,
    pub inner_rear: Vec3,
    pub outer: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CornerHardpoints {
    pub hub_center: Vec3,
    pub lower: WishbonePoints,
    pub upper: WishbonePoints,
    pub trackrod_inner: Vec3,
    pub trackrod_outer: Vec3,
    pub rod_outer: Vec3,
    pub rod_mount: RodMount,
    #[serde(default)]
    pub rod_kind_label: RodKindLabel,
    pub rocker_pivot: Vec3,
    pub rocker_axis: Vec3,
    pub rocker_pushrod_arm: Vec3,
    pub rocker_damper_arm: Vec3,
    pub damper_chassis: Vec3,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driveshaft_inner: Option<Vec3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driveshaft_outer: Option<Vec3>,
    #[serde(default)]
    pub provenance: CornerProvenance,
}

/// Per-axle physical spring/damper/travel. Explicit element properties;
/// nothing is inferred from static load here (legacy `calculate_spring_rate`
/// stays untouched for `Legacy1Dof`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxlePhysicalElements {
    /// Linear spring rate N/m (> 0).
    pub spring_rate_N_per_m: f64,
    /// Spring free (unloaded) length m (> 0).
    pub spring_free_length_m: f64,
    /// Spring installed length at design rest (`hub_center`) m.
    /// Preload = rate * (free - installed), must be >= 0.
    pub spring_installed_length_m: f64,
    /// Viscous damper bump rate N·s/m (>= 0).
    pub damper_bump_Ns_per_m: f64,
    /// Viscous damper rebound rate N·s/m (>= 0).
    pub damper_rebound_Ns_per_m: f64,
    /// Digressive knee m/s (> 0). Mirrors legacy 0.127.
    pub damper_knee_m_per_s: f64,
    /// Fast-shaft factor (0,1]. Mirrors legacy 0.5.
    pub damper_fast_factor: f64,
    /// Max rebound from design rest m (> 0, stored positive).
    pub wheel_droop_m: f64,
    /// Max bump from design rest m (> 0, stored positive).
    pub wheel_bump_m: f64,
    /// Damper length limits m (min < max, both > 0).
    pub damper_min_m: f64,
    pub damper_max_m: f64,
    /// Progressive bump-stop multiplier (>= 0). Multiplies the wheel tangent
    /// rate in the soft-stop region, mirroring the legacy shape explicitly.
    #[serde(default = "default_bump_stop_mult")]
    pub bump_stop_mult: f64,
}

fn default_bump_stop_mult() -> f64 {
    2.6
}

impl Default for AxlePhysicalElements {
    fn default() -> Self {
        Self {
            spring_rate_N_per_m: 29000.0,
            spring_free_length_m: 0.30,
            spring_installed_length_m: 0.26,
            damper_bump_Ns_per_m: 3000.0,
            damper_rebound_Ns_per_m: 4000.0,
            damper_knee_m_per_s: 0.127,
            damper_fast_factor: 0.5,
            wheel_droop_m: 0.05,
            wheel_bump_m: 0.10,
            damper_min_m: 0.15,
            damper_max_m: 0.40,
            bump_stop_mult: default_bump_stop_mult(),
        }
    }
}

/// Anti-roll contract. `LegacyRatio` preserves the current solver exactly:
/// `F_arb_self = (x_self - x_opp) * k_axle * ratio`, net axle zero.
/// `MotionRatio` uses an explicit bar rate and motion ratio; it never falls
/// back to the spring motion ratio.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum AntiRollConfig {
    LegacyRatio { ratio: f64 },
    MotionRatio {
        bar_rate_N_per_m: f64,
        motion_ratio: f64,
    },
}

impl Default for AntiRollConfig {
    fn default() -> Self {
        Self::LegacyRatio { ratio: 0.15 }
    }
}

/// Resolved four-corner geometry (mirrors already expanded).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometricCornerSet {
    pub fl: CornerHardpoints,
    pub fr: CornerHardpoints,
    pub fl_source: String,
    pub fr_source: String,
    pub rl: CornerHardpoints,
    pub rr: CornerHardpoints,
    pub rl_source: String,
    pub rr_source: String,
}

impl GeometricCornerSet {
    pub fn get(&self, wheel: WheelIndex) -> &CornerHardpoints {
        match wheel {
            WheelIndex::FrontLeft => &self.fl,
            WheelIndex::FrontRight => &self.fr,
            WheelIndex::RearLeft => &self.rl,
            WheelIndex::RearRight => &self.rr,
        }
    }
}

/// Top-level physical suspension. Version must be 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometricSuspensionConfig {
    pub version: u32,
    pub front: AxlePhysicalElements,
    pub rear: AxlePhysicalElements,
    pub front_arb: AntiRollConfig,
    pub rear_arb: AntiRollConfig,
    pub corners: GeometricCornerSet,
}

impl GeometricSuspensionConfig {
    pub const VERSION: u32 = 1;

    pub fn axle(&self, wheel: WheelIndex) -> &AxlePhysicalElements {
        if wheel.is_front() {
            &self.front
        } else {
            &self.rear
        }
    }

    pub fn arb(&self, front: bool) -> &AntiRollConfig {
        if front {
            &self.front_arb
        } else {
            &self.rear_arb
        }
    }

    /// Spring preload at design rest, N (>= 0 by validation).
    pub fn spring_preload_N(&self, wheel: WheelIndex) -> f64 {
        let a = self.axle(wheel);
        a.spring_rate_N_per_m * (a.spring_free_length_m - a.spring_installed_length_m)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "suspension.geometry_physical.version must be {} (got {})",
                Self::VERSION,
                self.version
            ));
        }
        validate_axle("front", &self.front)?;
        validate_axle("rear", &self.rear)?;
        validate_arb("front_arb", &self.front_arb)?;
        validate_arb("rear_arb", &self.rear_arb)?;
        for wheel in WheelIndex::ALL {
            validate_corner(wheel, self.corners.get(wheel))?;
        }
        // Mirror coherence: FR must be the x-mirror of FL, RR of RL.
        check_mirror(WheelIndex::FrontLeft, WheelIndex::FrontRight, &self.corners.fl, &self.corners.fr)?;
        check_mirror(WheelIndex::RearLeft, WheelIndex::RearRight, &self.corners.rl, &self.corners.rr)?;
        Ok(())
    }
}

fn validate_axle(name: &str, a: &AxlePhysicalElements) -> Result<(), String> {
    for (field, v) in [
        ("spring_rate_N_per_m", a.spring_rate_N_per_m),
        ("spring_free_length_m", a.spring_free_length_m),
        ("spring_installed_length_m", a.spring_installed_length_m),
        ("damper_bump_Ns_per_m", a.damper_bump_Ns_per_m),
        ("damper_rebound_Ns_per_m", a.damper_rebound_Ns_per_m),
        ("damper_knee_m_per_s", a.damper_knee_m_per_s),
        ("damper_fast_factor", a.damper_fast_factor),
        ("wheel_droop_m", a.wheel_droop_m),
        ("wheel_bump_m", a.wheel_bump_m),
        ("damper_min_m", a.damper_min_m),
        ("damper_max_m", a.damper_max_m),
        ("bump_stop_mult", a.bump_stop_mult),
    ] {
        if !v.is_finite() {
            return Err(format!(
                "suspension.geometry_physical.{name}.{field} must be finite (got {v})"
            ));
        }
    }
    if a.spring_rate_N_per_m <= 0.0 {
        return Err(format!(
            "suspension.geometry_physical.{name}.spring_rate_N_per_m must be > 0"
        ));
    }
    if a.spring_free_length_m <= 0.0 || a.spring_installed_length_m <= 0.0 {
        return Err(format!(
            "suspension.geometry_physical.{name}.spring free/installed lengths must be > 0"
        ));
    }
    if a.spring_free_length_m <= a.spring_installed_length_m {
        return Err(format!(
            "suspension.geometry_physical.{name}.spring_free_length_m must exceed installed length (preload >= 0)"
        ));
    }
    if a.damper_bump_Ns_per_m < 0.0 || a.damper_rebound_Ns_per_m < 0.0 {
        return Err(format!(
            "suspension.geometry_physical.{name}.damper rates must be >= 0"
        ));
    }
    if !(a.damper_knee_m_per_s > 0.0) {
        return Err(format!(
            "suspension.geometry_physical.{name}.damper_knee_m_per_s must be > 0"
        ));
    }
    if !(0.0 < a.damper_fast_factor && a.damper_fast_factor <= 1.0) {
        return Err(format!(
            "suspension.geometry_physical.{name}.damper_fast_factor must be in (0,1]"
        ));
    }
    if !(a.wheel_droop_m > 0.0 && a.wheel_bump_m > 0.0) {
        return Err(format!(
            "suspension.geometry_physical.{name}.wheel droop/bump must be > 0"
        ));
    }
    if !(a.damper_min_m > 0.0 && a.damper_max_m > a.damper_min_m) {
        return Err(format!(
            "suspension.geometry_physical.{name}.damper_min_m must be > 0 and < damper_max_m"
        ));
    }
    if !a.bump_stop_mult.is_finite() || a.bump_stop_mult < 0.0 {
        return Err(format!(
            "suspension.geometry_physical.{name}.bump_stop_mult must be finite and >= 0"
        ));
    }
    Ok(())
}

fn validate_arb(name: &str, arb: &AntiRollConfig) -> Result<(), String> {
    match *arb {
        AntiRollConfig::LegacyRatio { ratio } => {
            if !ratio.is_finite() {
                return Err(format!(
                    "suspension.geometry_physical.{name}.ratio must be finite"
                ));
            }
            if !(0.0..=2.0).contains(&ratio) {
                return Err(format!(
                    "suspension.geometry_physical.{name}.ratio must be in [0,2]"
                ));
            }
            Ok(())
        }
        AntiRollConfig::MotionRatio {
            bar_rate_N_per_m,
            motion_ratio,
        } => {
            if !bar_rate_N_per_m.is_finite() || !motion_ratio.is_finite() {
                return Err(format!(
                    "suspension.geometry_physical.{name} bar_rate/motion_ratio must be finite"
                ));
            }
            if bar_rate_N_per_m <= 0.0 || motion_ratio <= 0.0 {
                return Err(format!(
                    "suspension.geometry_physical.{name} bar_rate and motion_ratio must be > 0"
                ));
            }
            Ok(())
        }
    }
}

fn finite_vec(wheel: WheelIndex, field: &str, v: Vec3) -> Result<(), String> {
    if v.x.is_finite() && v.y.is_finite() && v.z.is_finite() {
        Ok(())
    } else {
        Err(format!(
            "suspension.geometry_physical.corners.{:?}.{field} must be finite (got {v:?})",
            wheel
        ))
    }
}

fn validate_corner(wheel: WheelIndex, c: &CornerHardpoints) -> Result<(), String> {
    finite_vec(wheel, "hub_center", c.hub_center)?;
    finite_vec(wheel, "lower.inner_front", c.lower.inner_front)?;
    finite_vec(wheel, "lower.inner_rear", c.lower.inner_rear)?;
    finite_vec(wheel, "lower.outer", c.lower.outer)?;
    finite_vec(wheel, "upper.inner_front", c.upper.inner_front)?;
    finite_vec(wheel, "upper.inner_rear", c.upper.inner_rear)?;
    finite_vec(wheel, "upper.outer", c.upper.outer)?;
    finite_vec(wheel, "trackrod_inner", c.trackrod_inner)?;
    finite_vec(wheel, "trackrod_outer", c.trackrod_outer)?;
    finite_vec(wheel, "rod_outer", c.rod_outer)?;
    finite_vec(wheel, "rocker_pivot", c.rocker_pivot)?;
    finite_vec(wheel, "rocker_axis", c.rocker_axis)?;
    finite_vec(wheel, "rocker_pushrod_arm", c.rocker_pushrod_arm)?;
    finite_vec(wheel, "rocker_damper_arm", c.rocker_damper_arm)?;
    finite_vec(wheel, "damper_chassis", c.damper_chassis)?;
    if let Some(v) = c.driveshaft_inner {
        finite_vec(wheel, "driveshaft_inner", v)?;
    }
    if let Some(v) = c.driveshaft_outer {
        finite_vec(wheel, "driveshaft_outer", v)?;
    }

    let min_dim = 0.01;
    let lower_axis_len = (c.lower.inner_rear - c.lower.inner_front).length();
    let upper_axis_len = (c.upper.inner_rear - c.upper.inner_front).length();
    if lower_axis_len < min_dim {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.lower axis too short ({:.4}m)",
            wheel, lower_axis_len
        ));
    }
    if upper_axis_len < min_dim {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.upper axis too short ({:.4}m)",
            wheel, upper_axis_len
        ));
    }
    let l_axis = (c.lower.inner_rear - c.lower.inner_front).normalized();
    let u_axis = (c.upper.inner_rear - c.upper.inner_front).normalized();
    if l_axis.length_squared() < 0.5 || u_axis.length_squared() < 0.5 {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.wishbone axis degenerate",
            wheel
        ));
    }
    let l_center = closest_point_on_axis(c.lower.outer, c.lower.inner_front, l_axis);
    let u_center = closest_point_on_axis(c.upper.outer, c.upper.inner_front, u_axis);
    let l_radius = (c.lower.outer - l_center).length();
    let u_radius = (c.upper.outer - u_center).length();
    let d_lh = (c.lower.outer - c.hub_center).length();
    let d_uh = (c.upper.outer - c.hub_center).length();
    let d_lu = (c.lower.outer - c.upper.outer).length();
    for (name, v) in [
        ("l_radius", l_radius),
        ("u_radius", u_radius),
        ("d_lh", d_lh),
        ("d_uh", d_uh),
        ("d_lu", d_lu),
    ] {
        if !(v > min_dim) {
            return Err(format!(
                "suspension.geometry_physical.corners.{:?}.{name} too small ({:.4}m)",
                wheel, v
            ));
        }
    }
    let l_push = (c.rod_outer - c.rocker_pushrod_arm).length();
    if !(l_push > min_dim) {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.rod length too small ({:.4}m)",
            wheel, l_push
        ));
    }
    let l_track = (c.trackrod_inner - c.trackrod_outer).length();
    if !(l_track > min_dim) {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.trackrod length too small ({:.4}m)",
            wheel, l_track
        ));
    }
    if c.rocker_axis.length() < 1e-6 {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.rocker_axis degenerate",
            wheel
        ));
    }
    if wheel.is_front() && (c.driveshaft_inner.is_some() || c.driveshaft_outer.is_some()) {
        return Err(format!(
            "suspension.geometry_physical.corners.{:?}.driveshaft must be rear-only",
            wheel
        ));
    }
    if wheel.is_rear() {
        match (c.driveshaft_inner, c.driveshaft_outer) {
            (Some(_), Some(_)) => {}
            (None, None) => {}
            _ => {
                return Err(format!(
                    "suspension.geometry_physical.corners.{:?}.driveshaft inner/outer must both be set or both absent",
                    wheel
                ))
            }
        }
    }
    Ok(())
}

fn check_mirror(
    base_wheel: WheelIndex,
    mirror_wheel: WheelIndex,
    base: &CornerHardpoints,
    mirrored: &CornerHardpoints,
) -> Result<(), String> {
    // Only enforced when the mirrored corner declares a mirror source; the
    // resolver tags `*_source` accordingly. Direct-authored symmetric corners
    // are allowed as long as each validates on its own.
    let tol = 1e-6;
    let mirror = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
    let pairs = [
        ("hub_center", base.hub_center, mirrored.hub_center),
        ("lower.inner_front", base.lower.inner_front, mirrored.lower.inner_front),
        ("lower.inner_rear", base.lower.inner_rear, mirrored.lower.inner_rear),
        ("lower.outer", base.lower.outer, mirrored.lower.outer),
        ("upper.inner_front", base.upper.inner_front, mirrored.upper.inner_front),
        ("upper.inner_rear", base.upper.inner_rear, mirrored.upper.inner_rear),
        ("upper.outer", base.upper.outer, mirrored.upper.outer),
        ("trackrod_inner", base.trackrod_inner, mirrored.trackrod_inner),
        ("trackrod_outer", base.trackrod_outer, mirrored.trackrod_outer),
        ("rod_outer", base.rod_outer, mirrored.rod_outer),
        ("rocker_pivot", base.rocker_pivot, mirrored.rocker_pivot),
        ("rocker_pushrod_arm", base.rocker_pushrod_arm, mirrored.rocker_pushrod_arm),
        ("rocker_damper_arm", base.rocker_damper_arm, mirrored.rocker_damper_arm),
        ("damper_chassis", base.damper_chassis, mirrored.damper_chassis),
    ];
    // Soft check: report the first mismatch beyond tolerance.
    for (name, b, m) in pairs {
        let expected = mirror(b);
        if (expected - m).length() > 1e-3 {
            // Not a hard error for hand-authored symmetric corners; the
            // kinematics tests enforce exact symmetry for mirrored profiles.
            // Keep a tight diagnostic when sources claim mirroring.
            let src = match mirror_wheel {
                WheelIndex::FrontRight | WheelIndex::RearRight => "mirror",
                _ => "direct",
            };
            if src == "mirror" && (expected - m).length() > tol {
                // The resolver guarantees this; a mismatch here means the
                // resolver was bypassed (programmatic construction).
                return Err(format!(
                    "suspension.geometry_physical.corners.{:?} must mirror {:?}.{name} (err {:.6}m)",
                    mirror_wheel,
                    base_wheel,
                    (expected - m).length()
                ));
            }
        }
    }
    Ok(())
}

pub fn closest_point_on_axis(p: Vec3, axis_point: Vec3, axis_dir: Vec3) -> Vec3 {
    if axis_dir.length_squared() < 1e-12 {
        return axis_point;
    }
    let d = axis_dir.normalized();
    axis_point + d * ((p - axis_point).dot(d))
}

pub fn mirror_vec(v: Vec3) -> Vec3 {
    Vec3::new(-v.x, v.y, v.z)
}

pub fn mirror_corner(base: &CornerHardpoints) -> CornerHardpoints {
    let m = mirror_vec;
    CornerHardpoints {
        hub_center: m(base.hub_center),
        lower: WishbonePoints {
            inner_front: m(base.lower.inner_front),
            inner_rear: m(base.lower.inner_rear),
            outer: m(base.lower.outer),
        },
        upper: WishbonePoints {
            inner_front: m(base.upper.inner_front),
            inner_rear: m(base.upper.inner_rear),
            outer: m(base.upper.outer),
        },
        trackrod_inner: m(base.trackrod_inner),
        trackrod_outer: m(base.trackrod_outer),
        rod_outer: m(base.rod_outer),
        rod_mount: base.rod_mount,
        rod_kind_label: base.rod_kind_label,
        rocker_pivot: m(base.rocker_pivot),
        rocker_axis: m(base.rocker_axis),
        rocker_pushrod_arm: m(base.rocker_pushrod_arm),
        rocker_damper_arm: m(base.rocker_damper_arm),
        damper_chassis: m(base.damper_chassis),
        driveshaft_inner: base.driveshaft_inner.map(m),
        driveshaft_outer: base.driveshaft_outer.map(m),
        provenance: CornerProvenance {
            origin: CornerProvenanceOrigin::Mirrored,
            note: String::new(),
        },
    }
}

// ── JSON parsing (deny_unknown_fields everywhere) ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonVec {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl From<JsonVec> for Vec3 {
    fn from(v: JsonVec) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<&Vec3> for JsonVec {
    fn from(v: &Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

pub fn vec_from_json_array(a: &[f64]) -> Result<Vec3, String> {
    if a.len() != 3 {
        return Err(format!("expected [x,y,z], got len {}", a.len()));
    }
    let v = Vec3::new(a[0], a[1], a[2]);
    if !(v.x.is_finite() && v.y.is_finite() && v.z.is_finite()) {
        return Err("non-finite [x,y,z]".to_string());
    }
    Ok(v)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonWishbone {
    inner_front: [f64; 3],
    inner_rear: [f64; 3],
    outer: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonTrackrod {
    inner: [f64; 3],
    outer: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonRod {
    outer: [f64; 3],
    #[serde(default = "default_rod_mount")]
    attachment: String,
    #[serde(default)]
    #[serde(rename = "type")]
    kind: Option<String>,
}

fn default_rod_mount() -> String {
    "lower".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonRocker {
    pivot: [f64; 3],
    axis: [f64; 3],
    pushrod_arm: [f64; 3],
    damper_arm: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonDamper {
    chassis: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonDriveshaft {
    inner: [f64; 3],
    outer: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonProvenance {
    #[serde(default = "default_origin")]
    origin: String,
    #[serde(default)]
    note: String,
}

fn default_origin() -> String {
    "measured".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonCornerFull {
    hub_center: [f64; 3],
    lower_wishbone: JsonWishbone,
    upper_wishbone: JsonWishbone,
    trackrod: JsonTrackrod,
    #[serde(default)]
    pushrod: Option<JsonRod>,
    #[serde(default)]
    rod: Option<JsonRod>,
    rocker: JsonRocker,
    damper: JsonDamper,
    #[serde(default)]
    driveshaft: Option<JsonDriveshaft>,
    #[serde(default)]
    provenance: Option<JsonProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum JsonCornerOrMirror {
    Mirror { mirror_of: String },
    Full(JsonCornerFull),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonAxlePhysical {
    spring_rate_N_per_m: f64,
    spring_free_length_m: f64,
    spring_installed_length_m: f64,
    damper_bump_Ns_per_m: f64,
    damper_rebound_Ns_per_m: f64,
    #[serde(default = "default_knee")]
    damper_knee_m_per_s: f64,
    #[serde(default = "default_fast")]
    damper_fast_factor: f64,
    wheel_droop_m: f64,
    wheel_bump_m: f64,
    damper_min_m: f64,
    damper_max_m: f64,
    #[serde(default = "default_bump_stop_mult")]
    bump_stop_mult: f64,
}

fn default_knee() -> f64 {
    0.127
}
fn default_fast() -> f64 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonArb {
    #[serde(default = "default_arb_mode")]
    mode: String,
    #[serde(default)]
    ratio: Option<f64>,
    #[serde(default)]
    bar_rate_N_per_m: Option<f64>,
    #[serde(default)]
    motion_ratio: Option<f64>,
}

fn default_arb_mode() -> String {
    "legacy_ratio".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonGeometricSuspension {
    #[serde(default = "geo_version_default")]
    version: u32,
    front: JsonAxlePhysical,
    rear: JsonAxlePhysical,
    #[serde(default = "default_legacy_arb_json")]
    front_arb: JsonArb,
    #[serde(default = "default_legacy_arb_json")]
    rear_arb: JsonArb,
    corners: HashMap<String, JsonCornerOrMirror>,
}

fn geo_version_default() -> u32 {
    1
}

fn default_legacy_arb_json() -> JsonArb {
    JsonArb {
        mode: "legacy_ratio".to_string(),
        ratio: Some(0.1),
        bar_rate_N_per_m: None,
        motion_ratio: None,
    }
}

fn parse_mount(s: &str, wheel: &str) -> Result<RodMount, String> {
    match s.to_ascii_lowercase().as_str() {
        "lower" => Ok(RodMount::Lower),
        "upper" => Ok(RodMount::Upper),
        _ => Err(format!(
            "suspension.geometry_physical.corners.{wheel}.rod.attachment must be upper|lower (got {s})"
        )),
    }
}

fn parse_rod_label(s: &Option<String>, wheel: &str) -> Result<RodKindLabel, String> {
    match s {
        None => Ok(RodKindLabel::Pushrod),
        Some(v) => RodKindLabel::from_str_loose(v).ok_or_else(|| {
            format!(
                "suspension.geometry_physical.corners.{wheel}.rod.type must be pushrod|pullrod|push|pull (got {v})"
            )
        }),
    }
}

fn parse_origin(s: &str, wheel: &str) -> Result<CornerProvenanceOrigin, String> {
    match s.to_ascii_lowercase().as_str() {
        "measured" => Ok(CornerProvenanceOrigin::Measured),
        "reconstructed" => Ok(CornerProvenanceOrigin::Reconstructed),
        "mirrored" => Ok(CornerProvenanceOrigin::Mirrored),
        _ => Err(format!(
            "suspension.geometry_physical.corners.{wheel}.provenance.origin must be measured|reconstructed|mirrored (got {s})"
        )),
    }
}

fn corner_from_full(wheel: &str, f: &JsonCornerFull) -> Result<(CornerHardpoints, String), String> {
    let rod_json = f.rod.as_ref().or(f.pushrod.as_ref()).ok_or_else(|| {
        format!(
            "suspension.geometry_physical.corners.{wheel}.rod (or legacy pushrod) is required"
        )
    })?;
    let rod_outer = vec_from_json_array(&rod_json.outer)
        .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.rod.outer: {e}"))?;
    let mount = parse_mount(&rod_json.attachment, wheel)?;
    let label = parse_rod_label(&rod_json.kind, wheel)?;
    let prov = match &f.provenance {
        None => CornerProvenance::default(),
        Some(p) => CornerProvenance {
            origin: parse_origin(&p.origin, wheel)?,
            note: p.note.clone(),
        },
    };
    let c = CornerHardpoints {
        hub_center: vec_from_json_array(&f.hub_center)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.hub_center: {e}"))?,
        lower: WishbonePoints {
            inner_front: vec_from_json_array(&f.lower_wishbone.inner_front)
                .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.lower_wishbone.inner_front: {e}"))?,
            inner_rear: vec_from_json_array(&f.lower_wishbone.inner_rear)
                .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.lower_wishbone.inner_rear: {e}"))?,
            outer: vec_from_json_array(&f.lower_wishbone.outer)
                .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.lower_wishbone.outer: {e}"))?,
        },
        upper: WishbonePoints {
            inner_front: vec_from_json_array(&f.upper_wishbone.inner_front)
                .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.upper_wishbone.inner_front: {e}"))?,
            inner_rear: vec_from_json_array(&f.upper_wishbone.inner_rear)
                .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.upper_wishbone.inner_rear: {e}"))?,
            outer: vec_from_json_array(&f.upper_wishbone.outer)
                .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.upper_wishbone.outer: {e}"))?,
        },
        trackrod_inner: vec_from_json_array(&f.trackrod.inner)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.trackrod.inner: {e}"))?,
        trackrod_outer: vec_from_json_array(&f.trackrod.outer)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.trackrod.outer: {e}"))?,
        rod_outer,
        rod_mount: mount,
        rod_kind_label: label,
        rocker_pivot: vec_from_json_array(&f.rocker.pivot)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.rocker.pivot: {e}"))?,
        rocker_axis: vec_from_json_array(&f.rocker.axis)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.rocker.axis: {e}"))?,
        rocker_pushrod_arm: vec_from_json_array(&f.rocker.pushrod_arm)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.rocker.pushrod_arm: {e}"))?,
        rocker_damper_arm: vec_from_json_array(&f.rocker.damper_arm)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.rocker.damper_arm: {e}"))?,
        damper_chassis: vec_from_json_array(&f.damper.chassis)
            .map_err(|e| format!("suspension.geometry_physical.corners.{wheel}.damper.chassis: {e}"))?,
        driveshaft_inner: f
            .driveshaft
            .as_ref()
            .map(|d| {
                vec_from_json_array(&d.inner).map_err(|e| {
                    format!(
                        "suspension.geometry_physical.corners.{wheel}.driveshaft.inner: {e}"
                    )
                })
            })
            .transpose()?,
        driveshaft_outer: f
            .driveshaft
            .as_ref()
            .map(|d| {
                vec_from_json_array(&d.outer).map_err(|e| {
                    format!(
                        "suspension.geometry_physical.corners.{wheel}.driveshaft.outer: {e}"
                    )
                })
            })
            .transpose()?,
        provenance: prov,
    };
    Ok((c, "direct".to_string()))
}

fn axle_from_json(name: &str, j: &JsonAxlePhysical) -> Result<AxlePhysicalElements, String> {
    let a = AxlePhysicalElements {
        spring_rate_N_per_m: j.spring_rate_N_per_m,
        spring_free_length_m: j.spring_free_length_m,
        spring_installed_length_m: j.spring_installed_length_m,
        damper_bump_Ns_per_m: j.damper_bump_Ns_per_m,
        damper_rebound_Ns_per_m: j.damper_rebound_Ns_per_m,
        damper_knee_m_per_s: j.damper_knee_m_per_s,
        damper_fast_factor: j.damper_fast_factor,
        wheel_droop_m: j.wheel_droop_m,
        wheel_bump_m: j.wheel_bump_m,
        damper_min_m: j.damper_min_m,
        damper_max_m: j.damper_max_m,
        bump_stop_mult: j.bump_stop_mult,
    };
    validate_axle(name, &a)?;
    Ok(a)
}

fn arb_from_json(name: &str, j: &JsonArb) -> Result<AntiRollConfig, String> {
    let cfg = match j.mode.to_ascii_lowercase().as_str() {
        "legacy_ratio" => {
            let ratio = j.ratio.ok_or_else(|| {
                format!("suspension.geometry_physical.{name}.ratio is required for legacy_ratio")
            })?;
            AntiRollConfig::LegacyRatio { ratio }
        }
        "motion_ratio" => {
            let bar = j.bar_rate_N_per_m.ok_or_else(|| {
                format!("suspension.geometry_physical.{name}.bar_rate_N_per_m is required for motion_ratio")
            })?;
            let mr = j.motion_ratio.ok_or_else(|| {
                format!("suspension.geometry_physical.{name}.motion_ratio is required for motion_ratio")
            })?;
            AntiRollConfig::MotionRatio {
                bar_rate_N_per_m: bar,
                motion_ratio: mr,
            }
        }
        other => {
            return Err(format!(
                "suspension.geometry_physical.{name}.mode must be legacy_ratio|motion_ratio (got {other})"
            ))
        }
    };
    validate_arb(name, &cfg)?;
    Ok(cfg)
}

/// Parse + resolve mirrors + validate. `wheel_names` must be FL/FR/RL/RR.
pub fn geometric_from_json_value(v: &serde_json::Value) -> Result<GeometricSuspensionConfig, String> {
    let j: JsonGeometricSuspension = serde_json::from_value(v.clone())
        .map_err(|e| format!("suspension.geometry_physical: {e}"))?;
    if j.version != GeometricSuspensionConfig::VERSION {
        return Err(format!(
            "suspension.geometry_physical.version must be {} (got {})",
            GeometricSuspensionConfig::VERSION,
            j.version
        ));
    }
    for key in j.corners.keys() {
        if !["FL", "FR", "RL", "RR"].contains(&key.as_str()) {
            return Err(format!(
                "suspension.geometry_physical.corners.{key} must be one of FL/FR/RL/RR"
            ));
        }
    }
    for required in ["FL", "FR", "RL", "RR"] {
        if !j.corners.contains_key(required) {
            return Err(format!(
                "suspension.geometry_physical.corners.{required} is required"
            ));
        }
    }
    // Resolve in two passes: full corners first, then mirrors (no chains).
    let mut resolved: HashMap<String, (CornerHardpoints, String)> = HashMap::new();
    for name in ["FL", "FR", "RL", "RR"] {
        if let Some(JsonCornerOrMirror::Full(f)) = j.corners.get(name) {
            let (c, src) = corner_from_full(name, f)?;
            resolved.insert(name.to_string(), (c, src));
        }
    }
    for name in ["FL", "FR", "RL", "RR"] {
        if resolved.contains_key(name) {
            continue;
        }
        match j.corners.get(name) {
            Some(JsonCornerOrMirror::Mirror { mirror_of }) => {
                if !["FL", "FR", "RL", "RR"].contains(&mirror_of.as_str()) {
                    return Err(format!(
                        "suspension.geometry_physical.corners.{name}.mirror_of must be FL/FR/RL/RR (got {mirror_of})"
                    ));
                }
                if mirror_of == name {
                    return Err(format!(
                        "suspension.geometry_physical.corners.{name}.mirror_of must not be self"
                    ));
                }
                // Target must be a directly-authored corner (no mirror chains).
                match j.corners.get(mirror_of.as_str()) {
                    Some(JsonCornerOrMirror::Full(_)) => {}
                    Some(JsonCornerOrMirror::Mirror { .. }) => {
                        return Err(format!(
                            "suspension.geometry_physical.corners.{name}.mirror_of={mirror_of} must point at a directly-authored corner (no chains)"
                        ))
                    }
                    None => {
                        return Err(format!(
                            "suspension.geometry_physical.corners.{name}.mirror_of target {mirror_of} missing"
                        ))
                    }
                }
                let (base, _) = resolved.get(mirror_of.as_str()).ok_or_else(|| {
                    format!("suspension.geometry_physical.corners.{name}.mirror_of target {mirror_of} missing")
                })?;
                let mirrored = mirror_corner(base);
                resolved.insert(name.to_string(), (mirrored, format!("mirror_of:{mirror_of}")));
            }
            _ => {
                return Err(format!(
                    "suspension.geometry_physical.corners.{name} is required"
                ))
            }
        }
    }
    let get = |n: &str| resolved.get(n).unwrap().clone();
    let (fl, fl_src) = get("FL");
    let (fr, fr_src) = get("FR");
    let (rl, rl_src) = get("RL");
    let (rr, rr_src) = get("RR");
    let cfg = GeometricSuspensionConfig {
        version: j.version,
        front: axle_from_json("front", &j.front)?,
        rear: axle_from_json("rear", &j.rear)?,
        front_arb: arb_from_json("front_arb", &j.front_arb)?,
        rear_arb: arb_from_json("rear_arb", &j.rear_arb)?,
        corners: GeometricCornerSet {
            fl,
            fr,
            fl_source: fl_src,
            fr_source: fr_src,
            rl,
            rr,
            rl_source: rl_src,
            rr_source: rr_src,
        },
    };
    cfg.validate()?;
    Ok(cfg)
}

/// Serialize back to a JSON value (round-trip preserving).
pub fn geometric_to_json_value(cfg: &GeometricSuspensionConfig) -> serde_json::Value {
    let corner_to_json = |c: &CornerHardpoints| {
        serde_json::json!({
            "hub_center": [c.hub_center.x, c.hub_center.y, c.hub_center.z],
            "lower_wishbone": {
                "inner_front": [c.lower.inner_front.x, c.lower.inner_front.y, c.lower.inner_front.z],
                "inner_rear": [c.lower.inner_rear.x, c.lower.inner_rear.y, c.lower.inner_rear.z],
                "outer": [c.lower.outer.x, c.lower.outer.y, c.lower.outer.z],
            },
            "upper_wishbone": {
                "inner_front": [c.upper.inner_front.x, c.upper.inner_front.y, c.upper.inner_front.z],
                "inner_rear": [c.upper.inner_rear.x, c.upper.inner_rear.y, c.upper.inner_rear.z],
                "outer": [c.upper.outer.x, c.upper.outer.y, c.upper.outer.z],
            },
            "trackrod": {
                "inner": [c.trackrod_inner.x, c.trackrod_inner.y, c.trackrod_inner.z],
                "outer": [c.trackrod_outer.x, c.trackrod_outer.y, c.trackrod_outer.z],
            },
            "rod": {
                "outer": [c.rod_outer.x, c.rod_outer.y, c.rod_outer.z],
                "attachment": match c.rod_mount { RodMount::Lower => "lower", RodMount::Upper => "upper" },
                "type": match c.rod_kind_label { RodKindLabel::Pushrod => "pushrod", RodKindLabel::Pullrod => "pullrod" },
            },
            "rocker": {
                "pivot": [c.rocker_pivot.x, c.rocker_pivot.y, c.rocker_pivot.z],
                "axis": [c.rocker_axis.x, c.rocker_axis.y, c.rocker_axis.z],
                "pushrod_arm": [c.rocker_pushrod_arm.x, c.rocker_pushrod_arm.y, c.rocker_pushrod_arm.z],
                "damper_arm": [c.rocker_damper_arm.x, c.rocker_damper_arm.y, c.rocker_damper_arm.z],
            },
            "damper": { "chassis": [c.damper_chassis.x, c.damper_chassis.y, c.damper_chassis.z] },
            "provenance": {
                "origin": match c.provenance.origin {
                    CornerProvenanceOrigin::Measured => "measured",
                    CornerProvenanceOrigin::Reconstructed => "reconstructed",
                    CornerProvenanceOrigin::Mirrored => "mirrored",
                },
                "note": c.provenance.note,
            },
        })
    };
    let mut fl_json = corner_to_json(&cfg.corners.fl);
    let mut rl_json = corner_to_json(&cfg.corners.rl);
    // Preserve driveshaft when present.
    for (corner, json) in [(&cfg.corners.fl, &mut fl_json), (&cfg.corners.rl, &mut rl_json)] {
        if let (Some(inner), Some(outer)) = (corner.driveshaft_inner, corner.driveshaft_outer) {
            json["driveshaft"] = serde_json::json!({
                "inner": [inner.x, inner.y, inner.z],
                "outer": [outer.x, outer.y, outer.z],
            });
        }
    }
    // FR/RR: emit mirror_of when the source tag says so, else full.
    let fr_json = if cfg.corners.fr_source.starts_with("mirror_of:") {
        let target = cfg.corners.fr_source["mirror_of:".len()..].to_string();
        serde_json::json!({ "mirror_of": target })
    } else {
        let mut v = corner_to_json(&cfg.corners.fr);
        if let (Some(inner), Some(outer)) = (cfg.corners.fr.driveshaft_inner, cfg.corners.fr.driveshaft_outer) {
            v["driveshaft"] = serde_json::json!({
                "inner": [inner.x, inner.y, inner.z],
                "outer": [outer.x, outer.y, outer.z],
            });
        }
        v
    };
    let rr_json = if cfg.corners.rr_source.starts_with("mirror_of:") {
        let target = cfg.corners.rr_source["mirror_of:".len()..].to_string();
        serde_json::json!({ "mirror_of": target })
    } else {
        let mut v = corner_to_json(&cfg.corners.rr);
        if let (Some(inner), Some(outer)) = (cfg.corners.rr.driveshaft_inner, cfg.corners.rr.driveshaft_outer) {
            v["driveshaft"] = serde_json::json!({
                "inner": [inner.x, inner.y, inner.z],
                "outer": [outer.x, outer.y, outer.z],
            });
        }
        v
    };
    let arb_to_json = |a: &AntiRollConfig| match *a {
        AntiRollConfig::LegacyRatio { ratio } => {
            serde_json::json!({ "mode": "legacy_ratio", "ratio": ratio })
        }
        AntiRollConfig::MotionRatio {
            bar_rate_N_per_m,
            motion_ratio,
        } => serde_json::json!({
            "mode": "motion_ratio",
            "bar_rate_N_per_m": bar_rate_N_per_m,
            "motion_ratio": motion_ratio,
        }),
    };
    serde_json::json!({
        "version": cfg.version,
        "front": {
            "spring_rate_N_per_m": cfg.front.spring_rate_N_per_m,
            "spring_free_length_m": cfg.front.spring_free_length_m,
            "spring_installed_length_m": cfg.front.spring_installed_length_m,
            "damper_bump_Ns_per_m": cfg.front.damper_bump_Ns_per_m,
            "damper_rebound_Ns_per_m": cfg.front.damper_rebound_Ns_per_m,
            "damper_knee_m_per_s": cfg.front.damper_knee_m_per_s,
            "damper_fast_factor": cfg.front.damper_fast_factor,
            "wheel_droop_m": cfg.front.wheel_droop_m,
            "wheel_bump_m": cfg.front.wheel_bump_m,
            "damper_min_m": cfg.front.damper_min_m,
            "damper_max_m": cfg.front.damper_max_m,
            "bump_stop_mult": cfg.front.bump_stop_mult,
        },
        "rear": {
            "spring_rate_N_per_m": cfg.rear.spring_rate_N_per_m,
            "spring_free_length_m": cfg.rear.spring_free_length_m,
            "spring_installed_length_m": cfg.rear.spring_installed_length_m,
            "damper_bump_Ns_per_m": cfg.rear.damper_bump_Ns_per_m,
            "damper_rebound_Ns_per_m": cfg.rear.damper_rebound_Ns_per_m,
            "damper_knee_m_per_s": cfg.rear.damper_knee_m_per_s,
            "damper_fast_factor": cfg.rear.damper_fast_factor,
            "wheel_droop_m": cfg.rear.wheel_droop_m,
            "wheel_bump_m": cfg.rear.wheel_bump_m,
            "damper_min_m": cfg.rear.damper_min_m,
            "damper_max_m": cfg.rear.damper_max_m,
            "bump_stop_mult": cfg.rear.bump_stop_mult,
        },
        "front_arb": arb_to_json(&cfg.front_arb),
        "rear_arb": arb_to_json(&cfg.rear_arb),
        "corners": {
            "FL": fl_json,
            "FR": fr_json,
            "RL": rl_json,
            "RR": rr_json,
        },
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn test_corner(hub_x: f64, with_shaft: bool) -> CornerHardpoints {
        CornerHardpoints {
            hub_center: Vec3::new(hub_x, 0.0, if hub_x.abs() > 0.73 { -1.475 } else { 1.475 }),
            lower: WishbonePoints {
                inner_front: Vec3::new(-0.165, -0.032, -1.52),
                inner_rear: Vec3::new(-0.191, -0.031, -1.06),
                outer: Vec3::new(-0.586, -0.032, -1.479),
            },
            upper: WishbonePoints {
                inner_front: Vec3::new(-0.165, 0.136, -1.52),
                inner_rear: Vec3::new(-0.191, 0.137, -1.197),
                outer: Vec3::new(-0.586, 0.137, -1.479),
            },
            trackrod_inner: Vec3::new(-0.164, -0.033, -1.606),
            trackrod_outer: Vec3::new(-0.582, -0.033, -1.586),
            rod_outer: Vec3::new(-0.55, -0.019, -1.439),
            rod_mount: RodMount::Lower,
            rod_kind_label: RodKindLabel::Pushrod,
            rocker_pivot: Vec3::new(-0.163, 0.22, -1.446),
            rocker_axis: Vec3::new(0.0, 1.0, 0.0),
            rocker_pushrod_arm: Vec3::new(-0.183, 0.22, -1.356),
            rocker_damper_arm: Vec3::new(-0.108, 0.22, -1.446),
            damper_chassis: Vec3::new(-0.108, 0.22, -1.12),
            driveshaft_inner: if with_shaft {
                Some(Vec3::new(-0.089, -0.003, 1.488))
            } else {
                None
            },
            driveshaft_outer: if with_shaft {
                Some(Vec3::new(hub_x, 0.0, 1.475))
            } else {
                None
            },
            provenance: CornerProvenance::default(),
        }
    }

    #[test]
    fn rod_label_does_not_affect_validation_shape() {
        let a = test_corner(-0.753, false);
        let mut b = a.clone();
        b.rod_kind_label = RodKindLabel::Pullrod;
        assert!(validate_corner(WheelIndex::FrontLeft, &a).is_ok());
        assert!(validate_corner(WheelIndex::FrontLeft, &b).is_ok());
    }

    #[test]
    fn mirror_negates_x() {
        let base = test_corner(-0.753, false);
        let m = mirror_corner(&base);
        assert!((m.hub_center.x + base.hub_center.x).abs() < 1e-12);
        assert!((m.hub_center.y - base.hub_center.y).abs() < 1e-12);
        assert_eq!(m.provenance.origin, CornerProvenanceOrigin::Mirrored);
    }
}

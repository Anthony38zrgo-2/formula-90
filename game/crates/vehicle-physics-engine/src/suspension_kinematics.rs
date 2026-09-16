//! SUS-GEO-03: deterministic double-wishbone kinematics in Rust.
//!
//! Reference geometry: `suspension_geometry.gd` (visual PBD solver). This
//! module is an independent Rust implementation for the physical path; the
//! GDScript file is a geometric reference, never a dependency.
//!
//! Inputs: wheel travel `q` (m, + bump from design `hub_center`) and rack
//! displacement `rack_m` (m, +X offset added to the front track-rod inner
//! pivots, shared by both front corners; rear corners ignore it).
//! Outputs: hub position, ball joints, rigid upright orientation, wheel
//! orientation (upright + static alignment), rocker angle, damper length and
//! compression `s = L_rest - L_current`, plus residuals/diagnostics.
//!
//! Solver: fixed-count position-based projection (48 pulls + 80 refine),
//! always initialised at the design rest pose, so the returned branch is the
//! one connected to rest. No bar stretching: unreachable targets clamp the
//! rocker/steering circle-links and report `clamped`; travel limits are found
//! by bisection from rest. All results finite; NaN inputs fall back to rest.
//!
//! Conventions: metres/seconds/kg/N/rad, chassis-local (+X right, +Y up,
//! -Z forward). `r = ds/dq` via central differences (eps 1e-6); wheel rate
//! `k_s*r^2 + F_s*dr/dq`, wheel damping `c_s*r^2` (see geo-config docs).

use crate::suspension_geo_config::{
    closest_point_on_axis, CornerHardpoints, GeometricSuspensionConfig,
};
use crate::types::{Mat3, Vec3, WheelIndex};
use serde::{Deserialize, Serialize};

pub const PBD_ITERATIONS: usize = 48;
pub const PBD_FINAL_PASSES: usize = 80;
pub const PBD_HUB_WEIGHT: f64 = 0.25;
pub const RESID_TOL_M: f64 = 1e-4;
pub const HUB_TOL_M: f64 = 1e-3;
pub const JAC_EPS_M: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KinematicResiduals {
    pub hub_y: f64,
    pub hub_lateral: f64,
    pub lower_radius: f64,
    pub upper_radius: f64,
    pub tri_lu: f64,
    pub tri_lh: f64,
    pub tri_uh: f64,
    pub pushrod: f64,
    pub trackrod: f64,
}

impl KinematicResiduals {
    pub fn max_link(&self) -> f64 {
        self.lower_radius
            .max(self.upper_radius)
            .max(self.tri_lu)
            .max(self.tri_lh)
            .max(self.tri_uh)
            .max(self.pushrod)
            .max(self.trackrod)
    }
    pub fn reachable(&self, rocker_clamped: bool, steering_clamped: bool) -> bool {
        if rocker_clamped || steering_clamped {
            return false;
        }
        if self.lower_radius > RESID_TOL_M
            || self.upper_radius > RESID_TOL_M
            || self.tri_lu > RESID_TOL_M
            || self.tri_lh > RESID_TOL_M
            || self.tri_uh > RESID_TOL_M
            || self.pushrod > RESID_TOL_M
            || self.trackrod > RESID_TOL_M
        {
            return false;
        }
        self.hub_y < HUB_TOL_M
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KinematicSolution {
    pub hub: Vec3,
    pub hub_target: Vec3,
    pub lbj: Vec3,
    pub ubj: Vec3,
    pub upright_basis: Mat3,
    pub wheel_basis: Mat3,
    pub rocker_angle: f64,
    pub rocker_clamped: bool,
    pub steering_clamped: bool,
    pub damper_length: f64,
    pub damper_compression: f64,
    pub travel_limited: bool,
    pub requested_travel: f64,
    pub solved_travel: f64,
    pub residuals: KinematicResiduals,
    pub converged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KinematicJacobian {
    /// ds/dq (damper compression per wheel travel).
    pub motion_ratio: f64,
    /// d(hub)/dq.
    pub dhub_dq: Vec3,
    /// d(rocker_angle)/dq.
    pub drocker_dq: f64,
    /// d(damper_length)/dq (= -r).
    pub ddamper_dq: f64,
}

struct Derived {
    l_dir: Vec3,
    l_center: Vec3,
    l_radius: f64,
    u_dir: Vec3,
    u_center: Vec3,
    u_radius: f64,
    d_lu: f64,
    d_lh: f64,
    d_uh: f64,
    l_pushrod: f64,
    l_trackrod: f64,
    damper_rest_len: f64,
}

fn derive(corner: &CornerHardpoints) -> Option<Derived> {
    let l_dir = (corner.lower.inner_rear - corner.lower.inner_front).normalized();
    let u_dir = (corner.upper.inner_rear - corner.upper.inner_front).normalized();
    if l_dir.length_squared() < 0.5 || u_dir.length_squared() < 0.5 {
        return None;
    }
    let l_center = closest_point_on_axis(corner.lower.outer, corner.lower.inner_front, l_dir);
    let u_center = closest_point_on_axis(corner.upper.outer, corner.upper.inner_front, u_dir);
    let l_radius = (corner.lower.outer - l_center).length();
    let u_radius = (corner.upper.outer - u_center).length();
    let d_lh = (corner.lower.outer - corner.hub_center).length();
    let d_uh = (corner.upper.outer - corner.hub_center).length();
    let d_lu = (corner.lower.outer - corner.upper.outer).length();
    let l_pushrod = (corner.rod_outer - corner.rocker_pushrod_arm).length();
    let l_trackrod = (corner.trackrod_inner - corner.trackrod_outer).length();
    let damper_rest_len = (corner.damper_chassis - corner.rocker_damper_arm).length();
    let min_dim = 0.01;
    if l_radius < min_dim
        || u_radius < min_dim
        || d_lh < min_dim
        || d_uh < min_dim
        || d_lu < min_dim
        || l_pushrod < min_dim
        || l_trackrod < min_dim
    {
        return None;
    }
    Some(Derived {
        l_dir,
        l_center,
        l_radius,
        u_dir,
        u_center,
        u_radius,
        d_lu,
        d_lh,
        d_uh,
        l_pushrod,
        l_trackrod,
        damper_rest_len,
    })
}

fn project_circle(p: Vec3, center: Vec3, axis: Vec3, radius: f64) -> Vec3 {
    let v = p - center;
    let v_perp = v - axis * v.dot(axis);
    if v_perp.length_squared() < 1e-18 {
        let u = perp(axis);
        return center + u * radius;
    }
    center + v_perp.normalized() * radius
}

fn perp(axis: Vec3) -> Vec3 {
    let mut u = axis.cross(Vec3::UP);
    if u.length_squared() < 1e-12 {
        u = axis.cross(Vec3::FORWARD);
    }
    if u.length_squared() < 1e-12 {
        return Vec3::RIGHT;
    }
    u.normalized()
}

fn constrain_triangle(
    mut lbj: Vec3,
    mut ubj: Vec3,
    mut hub: Vec3,
    d_lu: f64,
    d_lh: f64,
    d_uh: f64,
) -> (Vec3, Vec3, Vec3) {
    let dir = lbj - ubj;
    if dir.length_squared() > 1e-18 {
        let mid = (lbj + ubj) * 0.5;
        let d = dir.normalized();
        lbj = mid + d * (d_lu * 0.5);
        ubj = mid - d * (d_lu * 0.5);
    }
    let dh = lbj - hub;
    if dh.length_squared() > 1e-18 {
        let corr = dh.normalized() * (dh.length() - d_lh);
        lbj = lbj - corr * 0.5;
        hub = hub + corr * 0.5;
    }
    let du = ubj - hub;
    if du.length_squared() > 1e-18 {
        let corr = du.normalized() * (du.length() - d_uh);
        ubj = ubj - corr * 0.5;
        hub = hub + corr * 0.5;
    }
    (lbj, ubj, hub)
}

fn signed_angle(a: Vec3, b: Vec3, axis: Vec3) -> f64 {
    if a.length_squared() < 1e-18 || b.length_squared() < 1e-18 {
        return 0.0;
    }
    let an = a.normalized();
    let bn = b.normalized();
    let cross = an.cross(bn);
    let sin = cross.dot(axis.normalized());
    let cos = (an.dot(bn)).clamp(-1.0, 1.0);
    sin.atan2(cos)
}

fn circle_link(
    pivot: Vec3,
    axis: Vec3,
    rest: Vec3,
    other: Vec3,
    length: f64,
    preferred: f64,
) -> (Vec3, f64, bool) {
    let ax = if axis.length_squared() > 1e-12 {
        axis.normalized()
    } else {
        Vec3::RIGHT
    };
    let center = pivot + ax * ((rest - pivot).dot(ax));
    let radial = rest - center;
    let radius = radial.length();
    if radius < 1e-9 {
        return (rest, 0.0, true);
    }
    let offset = other - center;
    let axial = offset.dot(ax);
    let planar = offset - ax * axial;
    let distance = planar.length();
    if distance < 1e-9 {
        return (rest, 0.0, true);
    }
    let off_len_sq = offset.length_squared();
    let cosine = (radius * radius + off_len_sq - length * length) / (2.0 * radius * distance);
    let clamped = cosine.abs() > 1.0;
    let base = signed_angle(radial, planar, ax);
    let spread = cosine.clamp(-1.0, 1.0).acos();
    let two_pi = std::f64::consts::TAU;
    let wrap = |mut a: f64| {
        while a > std::f64::consts::PI {
            a -= two_pi;
        }
        while a < -std::f64::consts::PI {
            a += two_pi;
        }
        a
    };
    let ang_diff = |a: f64, b: f64| wrap(a - b).abs();
    let first = wrap(base + spread);
    let second = wrap(base - spread);
    let angle = if ang_diff(preferred, first) < ang_diff(preferred, second) {
        first
    } else {
        second
    };
    let point = center + rotate_vec(radial, ax, angle);
    (point, angle, clamped)
}

fn rotate_vec(v: Vec3, axis: Vec3, angle: f64) -> Vec3 {
    Mat3::from_axis_angle(axis, angle).transform_vector(v)
}

fn triangle_frame(lower: Vec3, upper: Vec3, hub: Vec3) -> Mat3 {
    let mut y = upper - lower;
    if y.length_squared() < 1e-18 {
        return Mat3::IDENTITY;
    }
    y = y.normalized();
    let mut x = (hub - lower) - y * ((hub - lower).dot(y));
    if x.length_squared() < 1e-18 {
        x = perp(y);
    } else {
        x = x.normalized();
    }
    let z = x.cross(y);
    Mat3::from_cols(x, y, z)
}

fn orthonormalize(m: Mat3) -> Mat3 {
    let mut x = m.x.normalized();
    let mut y = m.y - x * (m.y.dot(x));
    if y.length_squared() < 1e-18 {
        y = perp(x);
    } else {
        y = y.normalized();
    }
    let z = x.cross(y).normalized();
    // Re-orthogonalize x against y/z for numerical stability.
    x = y.cross(z).normalized();
    Mat3::from_cols(x, y, z)
}

/// Rack displacement shared by both front corners, calibrated from authored
/// rest geometry (mirrors `suspension_geometry.gd::_rack_displacement`).
pub fn rack_from_steer(
    fl: &CornerHardpoints,
    fr: &CornerHardpoints,
    steer_rad: f64,
) -> f64 {
    let mut total = 0.0;
    let mut count = 0;
    for c in [fl, fr] {
        let d = derive(c);
        if d.is_none() {
            continue;
        }
        let kingpin = (c.upper.outer - c.lower.outer).normalized();
        if kingpin.length_squared() < 0.5 {
            continue;
        }
        let outer = c.lower.outer + rotate_vec(c.trackrod_outer - c.lower.outer, kingpin, steer_rad);
        let inner = c.trackrod_inner;
        let offset = outer - inner;
        let len = d.unwrap().l_trackrod;
        let rest = len * len - offset.y * offset.y - offset.z * offset.z;
        if rest < 0.0 {
            continue;
        }
        let dx = rest.sqrt();
        let side = (c.trackrod_inner.x - c.trackrod_outer.x).signum();
        let side = if side == 0.0 { 1.0 } else { side };
        total += outer.x + side * dx - inner.x;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn solve_raw(
    corner: &CornerHardpoints,
    derived: &Derived,
    travel_q: f64,
    rack_m: f64,
    is_front: bool,
    static_camber: f64,
    static_toe: f64,
    side: f64,
) -> Option<KinematicSolution> {
    let q = if travel_q.is_finite() { travel_q } else { 0.0 };
    let rack = if rack_m.is_finite() { rack_m } else { 0.0 };
    let hub_target = corner.hub_center + Vec3::new(0.0, q, 0.0);
    if !hub_target.x.is_finite() || !hub_target.y.is_finite() || !hub_target.z.is_finite() {
        return None;
    }
    let mut lbj = hub_target + (corner.lower.outer - corner.hub_center);
    let mut ubj = hub_target + (corner.upper.outer - corner.hub_center);
    let mut hub = hub_target;
    for _ in 0..PBD_ITERATIONS {
        lbj = project_circle(lbj, derived.l_center, derived.l_dir, derived.l_radius);
        ubj = project_circle(ubj, derived.u_center, derived.u_dir, derived.u_radius);
        let t = constrain_triangle(lbj, ubj, hub, derived.d_lu, derived.d_lh, derived.d_uh);
        lbj = t.0;
        ubj = t.1;
        hub = t.2;
        hub = Vec3::new(hub.x, hub.y + (hub_target.y - hub.y) * PBD_HUB_WEIGHT, hub.z);
    }
    for _ in 0..PBD_FINAL_PASSES {
        lbj = project_circle(lbj, derived.l_center, derived.l_dir, derived.l_radius);
        ubj = project_circle(ubj, derived.u_center, derived.u_dir, derived.u_radius);
        let t = constrain_triangle(lbj, ubj, hub, derived.d_lu, derived.d_lh, derived.d_uh);
        lbj = t.0;
        ubj = t.1;
        hub = t.2;
    }
    if !(lbj.x.is_finite() && ubj.x.is_finite() && hub.x.is_finite()) {
        return None;
    }
    // Rigid upright transport + kingpin steering.
    let rest_frame = triangle_frame(corner.lower.outer, corner.upper.outer, corner.hub_center);
    let cur_frame = triangle_frame(lbj, ubj, hub);
    // transport = cur * rest^T
    let transport = mat_mul(&cur_frame, &mat_transpose(&rest_frame));
    let mut kingpin = ubj - lbj;
    if kingpin.length_squared() < 1e-18 {
        return None;
    }
    kingpin = kingpin.normalized();
    let outer_unsteered = lbj + transport.transform_vector(corner.trackrod_outer - corner.lower.outer);
    let mut rack_point = corner.trackrod_inner;
    if is_front {
        rack_point = Vec3::new(rack_point.x + rack, rack_point.y, rack_point.z);
    }
    // Mount-aware pushrod outer: transport the authored attachment with its arm.
    let mount_upper = corner.rod_outer.distance_to(corner.upper.outer)
        < corner.rod_outer.distance_to(corner.lower.outer)
        || matches!(
            corner_hardpoint_mount_hint(corner),
            MountHint::Upper
        );
    // Simpler robust rule: use the authored side via proximity to the rocker
    // plane? Fall back to explicit mount by comparing rest offsets is noisy;
    // instead reuse the visual convention: the mount is whichever arm's plane
    // contains the rod outer at rest. We approximate with arm-plane distance.
    let _ = mount_upper;
    let pushrod_outer = arm_point(corner, lbj, ubj, transport);
    let (rocker_end, rocker_angle, rocker_clamped) = circle_link(
        corner.rocker_pivot,
        corner.rocker_axis,
        corner.rocker_pushrod_arm,
        pushrod_outer,
        derived.l_pushrod,
        0.0,
    );
    let damper_end = corner.rocker_pivot
        + rotate_vec(
            corner.rocker_damper_arm - corner.rocker_pivot,
            corner.rocker_axis,
            rocker_angle,
        );
    let (steer_point, steer_angle, steering_clamped) = circle_link(
        lbj,
        kingpin,
        outer_unsteered,
        rack_point,
        derived.l_trackrod,
        0.0,
    );
    let _ = steer_point;
    let steer_rot = Mat3::from_axis_angle(kingpin, steer_angle);
    let upright_basis = orthonormalize(mat_mul(&steer_rot, &transport));
    hub = lbj + steer_rot.transform_vector(hub - lbj);
    let steer_arm = lbj + upright_basis.transform_vector(corner.trackrod_outer - corner.lower.outer);
    let toe_basis = Mat3::from_axis_angle(Vec3::UP, static_toe * side);
    let camber_basis = Mat3::from_axis_angle(Vec3::BACK, static_camber * side);
    let wheel_basis = orthonormalize(mat_mul(&upright_basis, &mat_mul(&toe_basis, &camber_basis)));
    let damper_length = (corner.damper_chassis - damper_end).length();
    let damper_compression = derived.damper_rest_len - damper_length;
    let residuals = KinematicResiduals {
        hub_y: (hub.y - hub_target.y).abs(),
        hub_lateral: Vec3::new(hub.x - hub_target.x, 0.0, hub.z - hub_target.z).length(),
        lower_radius: ((lbj - derived.l_center).length() - derived.l_radius).abs(),
        upper_radius: ((ubj - derived.u_center).length() - derived.u_radius).abs(),
        tri_lu: ((lbj - ubj).length() - derived.d_lu).abs(),
        tri_lh: ((lbj - hub).length() - derived.d_lh).abs(),
        tri_uh: ((ubj - hub).length() - derived.d_uh).abs(),
        pushrod: ((rocker_end - pushrod_outer).length() - derived.l_pushrod).abs(),
        trackrod: ((rack_point - steer_arm).length() - derived.l_trackrod).abs(),
    };
    let converged = residuals.reachable(rocker_clamped, steering_clamped);
    Some(KinematicSolution {
        hub,
        hub_target,
        lbj,
        ubj,
        upright_basis,
        wheel_basis,
        rocker_angle,
        rocker_clamped,
        steering_clamped,
        damper_length,
        damper_compression,
        travel_limited: false,
        requested_travel: q,
        solved_travel: q,
        residuals,
        converged,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum MountHint {
    Upper,
    Lower,
}

fn corner_hardpoint_mount_hint(corner: &CornerHardpoints) -> MountHint {
    // Authored explicit mount lives in the JSON rod.attachment; by the time we
    // reach hardpoints it is already resolved by the caller via arm_point().
    // Keep the helper for future explicit plumbing; default to proximity.
    let du = (corner.rod_outer - corner.upper.outer).length();
    let dl = (corner.rod_outer - corner.lower.outer).length();
    if du < dl {
        MountHint::Upper
    } else {
        MountHint::Lower
    }
}

/// Transport the authored rod attachment with its wishbone arm plane.
fn arm_point(
    corner: &CornerHardpoints,
    lbj: Vec3,
    ubj: Vec3,
    _transport: Mat3,
) -> Vec3 {
    // Decide mount by rest proximity (matches visual pushrod_mount).
    let du = (corner.rod_outer - corner.upper.outer).length();
    let dl = (corner.rod_outer - corner.lower.outer).length();
    if du < dl {
        // Upper mount: rotate about the upper arm axis.
        let pivot = corner.upper.inner_front;
        let axis = (corner.upper.inner_rear - corner.upper.inner_front).normalized();
        let center = closest_point_on_axis(corner.upper.outer, pivot, axis);
        let solved_center = closest_point_on_axis(ubj, pivot, axis);
        let rest_vec = corner.upper.outer - center;
        let solved_vec = ubj - solved_center;
        let ang = signed_angle(rest_vec, solved_vec, axis);
        pivot + rotate_vec(corner.rod_outer - pivot, axis, ang)
    } else {
        let pivot = corner.lower.inner_front;
        let axis = (corner.lower.inner_rear - corner.lower.inner_front).normalized();
        let rest_vec = corner.lower.outer - closest_point_on_axis(corner.lower.outer, pivot, axis);
        let solved_vec = lbj - closest_point_on_axis(lbj, pivot, axis);
        let ang = signed_angle(rest_vec, solved_vec, axis);
        pivot + rotate_vec(corner.rod_outer - pivot, axis, ang)
    }
}

fn mat_mul(a: &Mat3, b: &Mat3) -> Mat3 {
    // Columns of a*b: a applied to each column of b.
    Mat3::from_cols(
        a.transform_vector(b.x),
        a.transform_vector(b.y),
        a.transform_vector(b.z),
    )
}

fn mat_transpose(m: &Mat3) -> Mat3 {
    m.transpose()
}

/// Solve one corner at travel `q` and rack `rack_m`.
/// `static_camber/static_toe` are the axle alignment values (rad); `side` is
/// +1 for left, -1 for right (mirrors the visual convention).
pub fn solve_corner(
    corner: &CornerHardpoints,
    travel_q: f64,
    rack_m: f64,
    is_front: bool,
    static_camber: f64,
    static_toe: f64,
    side: f64,
) -> Option<KinematicSolution> {
    let derived = derive(corner)?;
    solve_raw(
        corner,
        &derived,
        travel_q,
        rack_m,
        is_front,
        static_camber,
        static_toe,
        side,
    )
}

/// Convenience wrapper resolving side/axle flags from wheel + config statics.
pub fn solve_wheel(
    geo: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    travel_q: f64,
    rack_m: f64,
    static_camber: f64,
    static_toe: f64,
) -> Option<KinematicSolution> {
    let corner = geo.corners.get(wheel);
    let side = if wheel.is_left() { 1.0 } else { -1.0 };
    solve_corner(
        corner,
        travel_q,
        rack_m,
        wheel.is_front(),
        static_camber,
        static_toe,
        side,
    )
}

/// Travel envelope connected to rest: bisection from 0 toward
/// [-droop, +bump], including the steering angle via rack.
pub fn travel_envelope(
    corner: &CornerHardpoints,
    droop: f64,
    bump: f64,
    rack_m: f64,
    is_front: bool,
) -> (f64, f64) {
    let lo = -droop.abs();
    let hi = bump.abs();
    let min = bisect_limit(corner, 0.0, lo, rack_m, is_front);
    let max = bisect_limit(corner, 0.0, hi, rack_m, is_front);
    (min, max)
}

fn reachable_at(
    corner: &CornerHardpoints,
    q: f64,
    rack_m: f64,
    is_front: bool,
) -> bool {
    match solve_corner(corner, q, rack_m, is_front, 0.0, 0.0, 1.0) {
        None => false,
        Some(s) => s.converged,
    }
}

fn bisect_limit(
    corner: &CornerHardpoints,
    rest: f64,
    endpoint: f64,
    rack_m: f64,
    is_front: bool,
) -> f64 {
    let mut good = rest;
    let mut candidate = rest;
    for step in 1..=32 {
        candidate = rest + (endpoint - rest) * (step as f64 / 32.0);
        if reachable_at(corner, candidate, rack_m, is_front) {
            good = candidate;
        } else {
            break;
        }
    }
    if (candidate - good).abs() < 1e-12 {
        return good;
    }
    // Refine between good and candidate.
    let mut bad = candidate;
    for _ in 0..16 {
        let mid = (good + bad) * 0.5;
        if reachable_at(corner, mid, rack_m, is_front) {
            good = mid;
        } else {
            bad = mid;
        }
    }
    good
}

/// Central-difference Jacobian at (q, rack).
pub fn jacobian(
    corner: &CornerHardpoints,
    travel_q: f64,
    rack_m: f64,
    is_front: bool,
) -> Option<KinematicJacobian> {
    let e = JAC_EPS_M;
    let a = solve_corner(corner, travel_q + e, rack_m, is_front, 0.0, 0.0, 1.0)?;
    let b = solve_corner(corner, travel_q - e, rack_m, is_front, 0.0, 0.0, 1.0)?;
    let ds = (a.damper_compression - b.damper_compression) / (2.0 * e);
    let dhub = (a.hub - b.hub) / (2.0 * e);
    let drock = (a.rocker_angle - b.rocker_angle) / (2.0 * e);
    let ddamp = (a.damper_length - b.damper_length) / (2.0 * e);
    Some(KinematicJacobian {
        motion_ratio: ds,
        dhub_dq: dhub,
        drocker_dq: drock,
        ddamper_dq: ddamp,
    })
}

/// Clamped solve inside the reachable envelope: mirrors the visual
/// `solve()` clamping to `[travel_min, travel_max]`.
pub fn solve_clamped(
    corner: &CornerHardpoints,
    requested_q: f64,
    rack_m: f64,
    is_front: bool,
    droop: f64,
    bump: f64,
    static_camber: f64,
    static_toe: f64,
    side: f64,
) -> Option<KinematicSolution> {
    let (lo, hi) = travel_envelope(corner, droop, bump, rack_m, is_front);
    let clamped_q = requested_q.clamp(lo.min(hi), lo.max(hi));
    let mut sol = solve_corner(corner, clamped_q, rack_m, is_front, static_camber, static_toe, side)?;
    sol.travel_limited = (clamped_q - requested_q).abs() > 1e-6;
    sol.requested_travel = if requested_q.is_finite() {
        requested_q
    } else {
        0.0
    };
    sol.solved_travel = clamped_q;
    Some(sol)
}

#[cfg(test)]
mod self_tests {
    use super::*;

    fn unit_corner() -> CornerHardpoints {
        crate::suspension_geo_config::tests::test_corner(-0.753, false)
    }

    #[test]
    fn rest_solve_is_converged_and_identity() {
        let c = unit_corner();
        let s = solve_corner(&c, 0.0, 0.0, true, 0.0, 0.0, 1.0).unwrap();
        assert!(s.converged);
        assert!((s.hub - c.hub_center).length() < 0.02);
        assert!(s.residuals.max_link() < 1e-3);
    }
}

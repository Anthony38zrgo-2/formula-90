//! SUS-GEO-05: chassis anchor reactions, body modes, load transfer, energy.
//!
//! # Equations of motion (defined before code, per backlog)
//!
//! Frames: chassis-local (+X right, +Y up, -Z forward, metres/radians/seconds).
//! CG at `center_of_mass_local` (task: com_z from weight distribution).
//!
//! ## Per-wheel linkage statics (quasi-static, ideal ball joints, rigid links)
//!
//! Upright free body (wheel centre W, contact C):
//! ```text
//!   sum_i f_i * u_i + F_rod_up + F_c + W_vec = m_u * a_vec     (3 force eqs)
//!   sum_i f_i * ((P_i - W) x u_i) + (M_rod + M_c) = 0          (3 moment eqs about W)
//! ```
//! - `f_i`: axial scalars for the wishbone legs (LF,LR,UF,UR) + trackrod (+
//!   driveshaft on the rear). `u_i`: unit direction inner->outer at the
//!   current `q` + steer. `f_i > 0` = leg in compression (pushes upright).
//! - `F_rod_up`: rod force on the upright from SUS-GEO-04 (tension +).
//! - `F_c`: contact force vector on the wheel (tire lateral/longitudinal +
//!   normal `Fz`), applied at the contact point.
//! - `W_vec = (0,-m_u*g,0)`: unsprung weight; `a_vec = (0,a_u,0)`: vertical
//!   virtual-filter inertia from the suspension state.
//! - Front: 5 link unknowns + known rod = 5x6 overdetermined -> least squares.
//!   Rear: 6 link unknowns (+shaft) + known rod -> 6x6 exact.
//!   (The rod itself is solved on the rocker side in SUS-GEO-04; here it is an
//!   input, and the residual is a consistency diagnostic, never hidden.)
//!
//! ## Rocker path (massless rocker + damper, from SUS-GEO-04 forces)
//!
//! ```text
//!   R_pivot  = -(F_rod_rock + F_elem_rock)     (on chassis at rocker pivot)
//!   R_damper = -F_elem_rock                    (on chassis at damper mount)
//! ```
//! with `F_elem_rock = F_elem * (A-C).norm` (compression pushes the arm away
//! from the chassis) and `F_rod_rock = T * (O-E).norm` (tension pulls the
//! rocker toward the outer joint).
//!
//! ## Body wrench (geometric mode)
//!
//! ```text
//!   F_body = sum_anchors R + F_aero + F_drag
//!   M_body = sum_anchors (r_anchor - r_CG) x R + M_aero + M_align
//! ```
//! Contact forces are REPLACED by anchor reactions (they are the same load,
//! transmitted). Whole-system balance (validation, not assumption):
//! `|sum_wheel C + F_c + W - m*a| ~= 0`. Total force matches legacy within
//! unsprung terms; total MOMENT differs by design (anchors vs contact patch:
//! this is where anti-dive/squat and jacking live).
//!
//! Airborne wheels (`!grounded`): same solver with `F_c = 0`, so the hanging
//! unsprung weight distributes to the anchors (pulls the body down at the
//! mounts) instead of vanishing.
//!
//! ## Body modes (telemetry decomposition of q[4], metres)
//!
//! ```text
//!   heave = (qFL+qFR+qRL+qRR)/4
//!   roll  = ((qFL+qRL)-(qFR+qRR))/4     (+ = body rolls left-down? see fn)
//!   pitch = ((qRL+qRR)-(qFL+qFR))/4     (+ = nose-up / rear compressed)
//!   warp  = ((qFL+qRR)-(qFR+qRL))/4     (diagonal, chassis torsion input)
//! ```
//! Roll/pitch stiffness increase (acceptance) comes from springs (k*r^2),
//! ARB (roll only) and stops; verified by asymmetric-load tests.
//!
//! ## Kinematic centres (DIAGNOSTIC ONLY, never drive forces)
//!
//! Front-view instant centre per side from the lower/upper wishbone leg planes
//! (least-squares intersection in the transverse plane at the wheel z), roll
//! centre from the IC-to-contact lines of both sides. Side-view pitch centre
//! analogously. Reported heights only.
//!
//! ## Load paths (registered per wheel, vertical shares)
//!
//! - element path: rod + damper-mount + rocker-pivot vertical reactions
//!   (this is the thrust that sets the aero platform via ride height);
//! - wishbone path: 4 leg vertical reactions;
//! - other path: trackrod (+driveshaft) vertical reactions.
//! Shares sum to ~1 by the balance identity above.
//!
//! ## Energy (per wheel + axle, joules/watts)
//!
//! - spring stored: `E = 1/2 k total^2` (analytic, total from free length);
//! - damper dissipated: accumulated `sum F_d * v_s * dt` (>= 0);
//! - ARB axle: `E = 1/2 k_arb (dq)^2` with LegacyRatio `k_arb = k_rest*ratio`
//!   or MotionRatio `k_arb = k_bar*mr^2`;
//! - anchor power audit: `P = sum R . v_anchor` with rigid-body anchor
//!   velocities (external-mode audit; standalone integrates the same body).
//!
//! ## Limits (no false precision, task 9)
//!
//! Truss idealization (no wishbone bending, no joint play/friction, massless
//! rocker/links), upright inertia only vertical (matches the 1-DOF filter),
//! shaft-outer rigid with the upright (plunge absorbs the rest in reality),
//! steer applied to the trackrod outer only. Reconstructed hardpoints
//! (rear inboard, visual rockers) flow through the same solver but are
//! labelled by provenance and blocked from activation until SUS-GEO-09.

use crate::suspension_geo_config::{CornerHardpoints, GeometricSuspensionConfig};
use crate::suspension_kinematics::KinematicSolution;
use crate::types::Vec3;
use serde::{Deserialize, Serialize};

/// One chassis anchor reaction (force ON the chassis).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnchorReaction {
    pub position: Vec3,
    pub force: Vec3,
    /// 0..3 leg index / 4 trackrod / 5 driveshaft / 6 rocker pivot / 7 damper.
    pub kind: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct WheelLoadPaths {
    /// Vertical share through rod+damper+rocker (sets the aero platform).
    pub element_share: f64,
    /// Vertical share through the 4 wishbone legs.
    pub wishbone_share: f64,
    /// Vertical share through trackrod (+driveshaft).
    pub other_share: f64,
    /// Balance residual |G - (F_c + W - m a)| (N, ~solve error when consistent).
    pub balance_residual_n: f64,
    /// |rod_05 - rod_04| (N): upright-side vs rocker-side rod force.
    pub rod_mismatch_n: f64,
    /// True when the link solve fell back to damped least squares.
    pub singular: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct BodyModes {
    pub heave_m: f64,
    pub roll_m: f64,
    pub pitch_m: f64,
    pub warp_m: f64,
    /// Diagnostic roll-centre heights (m, chassis-local Y at axle z).
    pub rc_front_m: f64,
    pub rc_rear_m: f64,
    /// Diagnostic side-view pitch-centre height (m).
    pub pc_m: f64,
    pub transfer_front_n: f64,
    pub transfer_rear_n: f64,
}

/// Link axes at the current pose: 4 legs + trackrod (+shaft) unit directions
/// inner->outer, outer points, and the upright-side rod force (tension +).
pub struct LinkFrame {
    /// (inner_point, outer_point, unit_dir) for LF,LR,UF,UR,TRACK,(SHAFT?),
    /// plus the pushrod as the final entry (rocker_end -> outer, tension +).
    pub links: Vec<(Vec3, Vec3, Vec3)>,
    pub has_shaft: bool,
    /// Index of the rod entry inside `links` (5 front, 6 rear).
    pub rod_index: usize,
    pub wheel_center: Vec3,
}

/// Build the link frame from hardpoints + a kinematics solution.
/// `trackrod_outer_steered`: steered outer joint (kingpin rotation applied).
/// `shaft_outer`: driveshaft outer joint at current pose (rear only).
/// `rod_force_n`: tension + from SUS-GEO-04; direction rocker_end->outer.
pub fn link_frame(
    corner: &CornerHardpoints,
    sol: &KinematicSolution,
    trackrod_outer_steered: Vec3,
    shaft_outer: Option<Vec3>,
) -> LinkFrame {
    let mut links = Vec::with_capacity(7);
    let leg = |inner: Vec3, outer: Vec3| {
        let d = outer - inner;
        let n = if d.length_squared() > 1e-12 {
            d.normalized()
        } else {
            Vec3::UP
        };
        (inner, outer, n)
    };
    links.push(leg(corner.lower.inner_front, sol.lbj));
    links.push(leg(corner.lower.inner_rear, sol.lbj));
    links.push(leg(corner.upper.inner_front, sol.ubj));
    links.push(leg(corner.upper.inner_rear, sol.ubj));
    links.push(leg(corner.trackrod_inner, trackrod_outer_steered));
    let has_shaft = shaft_outer.is_some() && corner.driveshaft_inner.is_some();
    if let (Some(outer), Some(inner)) = (shaft_outer, corner.driveshaft_inner) {
        links.push(leg(inner, outer));
    }
    // Pushrod as the final link (rocker_end -> outer). Tension + pulls the
    // upright toward the rocker: force on upright = +t * u with
    // u = (rocker_end - outer).norm. Solved as an unknown (6th front,
    // 7th rear); the SUS-GEO-04 rocker-side value is only a cross-check
    // (rod_mismatch_n), never an input.
    let rod_dir = sol.rocker_end - sol.pushrod_outer;
    let rod_u = if rod_dir.length_squared() > 1e-12 {
        rod_dir.normalized()
    } else {
        Vec3::UP
    };
    let rod_index = links.len();
    links.push((sol.rocker_end, sol.pushrod_outer, rod_u));
    LinkFrame {
        links,
        has_shaft,
        rod_index,
        wheel_center: sol.hub,
    }
}

/// Solve link axials: unknowns f_i along `frame.links` directions such that
/// the upright balances `resultant_on_upright` (force on upright at W, i.e.
/// `-(F_c + W_vec - m a)` side moved) with moment `moment_about_w` about W.
/// Front (5 links): least squares. Rear (6 links): exact 6x6.
/// Returns (axials in link order, residual_norm, singular_flag).
pub fn solve_links(
    frame: &LinkFrame,
    resultant_on_upright: Vec3,
    moment_about_w: Vec3,
) -> (Vec<f64>, f64, bool) {
    let n = frame.links.len();
    // Rows: Fx,Fy,Fz,Mx,My,Mz. Cols: link axials.
    let mut a = vec![vec![0.0; n]; 6];
    for (j, (inner, outer, u)) in frame.links.iter().enumerate() {
        let _ = inner;
        let r = *outer - frame.wheel_center;
        let m = r.cross(*u);
        a[0][j] = u.x;
        a[1][j] = u.y;
        a[2][j] = u.z;
        a[3][j] = m.x;
        a[4][j] = m.y;
        a[5][j] = m.z;
    }
    // The caller folds known forces (rod on upright, contact, weight,
    // inertia) into resultant/moment (see reactions()).
    let b = [
        resultant_on_upright.x,
        resultant_on_upright.y,
        resultant_on_upright.z,
        moment_about_w.x,
        moment_about_w.y,
        moment_about_w.z,
    ];
    if n == 6 {
        match solve_6x6(&a, &b) {
            Some(x) => {
                let r = residual(&a, &x, &b);
                let owned = x;
                (owned, r, false)
            }
            None => {
                let (x, r) = damped_least_squares(&a, &b, 1e-6);
                (x, r, true)
            }
        }
    } else {
        // Under/over-determined (front 5, or degenerate): damped least
        // squares. Flag when the residual is large instead of by shape.
        let (x, r) = damped_least_squares(&a, &b, 1e-9);
        (x, r, r > 1.0)
    }
}

fn residual(a: &[Vec<f64>], x: &[f64], b: &[f64; 6]) -> f64 {
    let mut s = 0.0;
    for i in 0..6 {
        let mut ax = 0.0;
        for (j, v) in x.iter().enumerate() {
            ax += a[i][j] * v;
        }
        s += (ax - b[i]) * (ax - b[i]);
    }
    s.sqrt()
}

/// Gaussian elimination with partial pivoting for 6x6. None if singular.
fn solve_6x6(a: &[Vec<f64>], b: &[f64; 6]) -> Option<Vec<f64>> {
    let n = 6;
    let mut m = [[0.0; 7]; 6];
    for i in 0..n {
        for j in 0..n {
            m[i][j] = a[i][j];
        }
        m[i][n] = b[i];
    }
    for col in 0..n {
        let mut piv = col;
        for row in col..n {
            if m[row][col].abs() > m[piv][col].abs() {
                piv = row;
            }
        }
        if m[piv][col].abs() < 1e-9 {
            return None;
        }
        m.swap(col, piv);
        let d = m[col][col];
        for row in 0..n {
            if row == col {
                continue;
            }
            let f = m[row][col] / d;
            if f != 0.0 {
                for k in col..=n {
                    m[row][k] -= f * m[col][k];
                }
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in 0..n {
        if m[i][i].abs() < 1e-12 {
            return None;
        }
        x[i] = m[i][n] / m[i][i];
        if !x[i].is_finite() {
            return None;
        }
    }
    Some(x)
}

/// Damped least squares (Tikhonov) via normal equations + solve_6x6.
/// Works for any n>=1; damping keeps singular poses finite + flagged by caller.
fn damped_least_squares(a: &[Vec<f64>], b: &[f64; 6], lambda: f64) -> (Vec<f64>, f64) {
    let n = a[0].len();
    // AtA (n x n) + b via Atb. For n != 6 solve via small Gauss-Jordan on n.
    let mut at_a = vec![vec![0.0; n]; n];
    let mut at_b = vec![0.0; n];
    for i in 0..6 {
        for j in 0..n {
            at_b[j] += a[i][j] * b[i];
            for k in 0..n {
                at_a[j][k] += a[i][j] * a[i][k];
            }
        }
    }
    for j in 0..n {
        at_a[j][j] += lambda;
    }
    let x = gauss_jordan(&at_a, &at_b).unwrap_or_else(|| vec![0.0; n]);
    (x.clone(), residual(a, &x, b))
}

fn gauss_jordan(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a.to_vec();
    let mut v = b.to_vec();
    for col in 0..n {
        let mut piv = col;
        for row in col..n {
            if m[row][col].abs() > m[piv][col].abs() {
                piv = row;
            }
        }
        if m[piv][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, piv);
        v.swap(col, piv);
        let d = m[col][col];
        for k in col..n {
            m[col][k] /= d;
        }
        v[col] /= d;
        for row in 0..n {
            if row == col {
                continue;
            }
            let f = m[row][col];
            if f != 0.0 {
                for k in col..n {
                    m[row][k] -= f * m[col][k];
                }
                v[row] -= f * v[col];
            }
        }
    }
    Some(v)
}

/// Full per-wheel reaction set ON the chassis.
/// Inputs in chassis-local metres/newtons; `contact_force` ON the wheel at
/// `contact_point` (zero when airborne); `contact_couple` (aligning torque,
/// free vector, zero when airborne); `unsprung_weight_n` (+);
/// `unsprung_inertia_n` (signed, + = upward m*a on the wheel).
/// `element_force_n`: spring+damper compression (+) from SUS-GEO-04 along
/// `damper_dir` (unit, arm minus chassis). `rod_reference_n`: rocker-side rod
/// tension (+) from SUS-GEO-04, cross-check only (rod_mismatch_n).
/// Returns (reactions: legs/track/shaft inners + rocker pivot + damper mount,
/// load paths with the whole-wheel balance residual).
#[allow(clippy::too_many_arguments)]
pub fn reactions(
    frame: &LinkFrame,
    contact_force: Vec3,
    contact_point: Vec3,
    contact_couple: Vec3,
    unsprung_weight_n: f64,
    unsprung_inertia_n: f64,
    element_force_n: f64,
    damper_dir: Vec3,
    rocker_pivot: Vec3,
    damper_chassis: Vec3,
    rod_reference_n: f64,
) -> (Vec<AnchorReaction>, WheelLoadPaths) {
    // Upright equilibrium (all links incl. rod as unknowns):
    //   S_links + F_c + W - m a = 0  (force)
    //   M_links + (C - W) x F_c + T_align = 0  (moment about W)
    let w = Vec3::new(0.0, -unsprung_weight_n + unsprung_inertia_n, 0.0);
    let target_f = Vec3::new(
        -contact_force.x - w.x,
        -contact_force.y - w.y,
        -contact_force.z - w.z,
    );
    let rc = contact_point - frame.wheel_center;
    let mc = rc.cross(contact_force) + contact_couple;
    // Weight/inertia act at W: no moment.
    let target_m = Vec3::new(-mc.x, -mc.y, -mc.z);
    let n = frame.links.len();
    let mut a = vec![vec![0.0; n]; 6];
    for (j, (_, outer, u)) in frame.links.iter().enumerate() {
        let r = *outer - frame.wheel_center;
        let m = r.cross(*u);
        a[0][j] = u.x;
        a[1][j] = u.y;
        a[2][j] = u.z;
        a[3][j] = m.x;
        a[4][j] = m.y;
        a[5][j] = m.z;
    }
    let b = [target_f.x, target_f.y, target_f.z, target_m.x, target_m.y, target_m.z];
    let (axials, resid_eq, singular) = if n == 6 {
        match solve_6x6(&a, &b) {
            Some(x) => {
                let r = residual(&a, &x, &b);
                let owned = x;
                (owned, r, false)
            }
            None => {
                let (x, r) = damped_least_squares(&a, &b, 1e-6);
                (x, r, true)
            }
        }
    } else {
        let (x, r) = damped_least_squares(&a, &b, 1e-9);
        (x, r, r > 1.0)
    };
    let mut out = Vec::with_capacity(n + 2);
    let mut wish_y = 0.0;
    let mut other_y = 0.0;
    // Solved rod tension (tension +). Cross-check vs the rocker side below.
    let rod_solved = if frame.rod_index < axials.len() {
        let v = axials[frame.rod_index];
        if v.is_finite() { v } else { 0.0 }
    } else {
        0.0
    };
    for (j, (inner, _, u)) in frame.links.iter().enumerate() {
        if j == frame.rod_index {
            continue;
        }
        let f = if axials[j].is_finite() { axials[j] } else { 0.0 };
        // Link in compression pushes the chassis mount away from the outer:
        // force ON chassis = -f * u.
        let r = Vec3::new(-f * u.x, -f * u.y, -f * u.z);
        let kind = if j < 4 {
            wish_y += r.y;
            j as u8
        } else if j == 4 {
            other_y += r.y;
            4
        } else {
            other_y += r.y;
            5
        };
        out.push(AnchorReaction {
            position: *inner,
            force: r,
            kind,
        });
    }
    // Rocker path from the SOLVED rod (uniform tension, both ends) and the
    // SUS-GEO-04 element force: pivot balances, mount takes the damper.
    // Force on rocker from rod (tension pulls E toward O):
    let (_, _, rod_u) = frame.links[frame.rod_index];
    let f_rock_rod = rod_u * (-rod_solved);
    let f_elem = if element_force_n.is_finite() {
        element_force_n.max(0.0)
    } else {
        0.0
    };
    let ddu = if damper_dir.length_squared() > 1e-12 {
        damper_dir.normalized()
    } else {
        Vec3::UP
    };
    let f_rock_elem = ddu * f_elem;
    // Force ON chassis at pivot = -(force on rocker) = +(sum on rocker).
    let r_pivot = f_rock_rod + f_rock_elem;
    // Force ON chassis at damper mount (opposite end of the damper).
    let r_damper = ddu * (-f_elem);
    out.push(AnchorReaction {
        position: rocker_pivot,
        force: r_pivot,
        kind: 6,
    });
    out.push(AnchorReaction {
        position: damper_chassis,
        force: r_damper,
        kind: 7,
    });
    let _ = resid_eq;
    // Element path vertical = rocker pivot + damper mount vertical (the rod
    // load reaches the chassis through the pivot, already included).
    let elem_y = r_pivot.y + r_damper.y;
    let total_y = wish_y + other_y + elem_y;
    let inv = if total_y.abs() > 1e-9 { 1.0 / total_y } else { 0.0 };
    // Balance check: the chassis reactions must equal the external load
    // (whole-system equilibrium: G = F_c + W - m a). Residual ~ solve error.
    let mut sum = Vec3::ZERO;
    for r in &out {
        sum = sum + r.force;
    }
    let wvec = Vec3::new(0.0, -unsprung_weight_n + unsprung_inertia_n, 0.0);
    let bal = (sum - contact_force - wvec).length();
    let rod_mm = if rod_reference_n.is_finite() {
        (rod_solved - rod_reference_n).abs()
    } else {
        0.0
    };
    let paths = WheelLoadPaths {
        element_share: elem_y * inv,
        wishbone_share: wish_y * inv,
        other_share: other_y * inv,
        balance_residual_n: if bal.is_finite() { bal } else { f64::INFINITY },
        rod_mismatch_n: rod_mm,
        singular,
    };
    let _ = resid_eq;
    (out, paths)
}

/// Modal decomposition of wheel travels q[FL,FR,RL,RR] (m).
pub fn body_modes(q: [f64; 4]) -> BodyModes {
    BodyModes {
        heave_m: (q[0] + q[1] + q[2] + q[3]) * 0.25,
        roll_m: ((q[0] + q[2]) - (q[1] + q[3])) * 0.25,
        pitch_m: ((q[2] + q[3]) - (q[0] + q[1])) * 0.25,
        warp_m: ((q[0] + q[3]) - (q[1] + q[2])) * 0.25,
        rc_front_m: 0.0,
        rc_rear_m: 0.0,
        pc_m: 0.0,
        transfer_front_n: 0.0,
        transfer_rear_n: 0.0,
    }
}

/// Front-view roll-centre height (diagnostic) for one axle from the two
/// wishbone leg planes. Uses leg midpoints/directions projected at the axle z.
/// Returns None when the geometry is degenerate (parallel eyes).
pub fn roll_centre_height(
    left: &CornerHardpoints,
    right: &CornerHardpoints,
    contact_half_track: f64,
) -> Option<f64> {
    // Instant centre per side: intersect lower-leg line with upper-leg line in
    // the transverse (x-y) plane. Legs as segments inner->outer (midpoint of
    // the two inner pivots per wishbone).
    let ic = |c: &CornerHardpoints| -> Option<(f64, f64)> {
        let l0 = (
            (c.lower.inner_front.x + c.lower.inner_rear.x) * 0.5,
            (c.lower.inner_front.y + c.lower.inner_rear.y) * 0.5,
        );
        let l1 = (c.lower.outer.x, c.lower.outer.y);
        let u0 = (
            (c.upper.inner_front.x + c.upper.inner_rear.x) * 0.5,
            (c.upper.inner_front.y + c.upper.inner_rear.y) * 0.5,
        );
        let u1 = (c.upper.outer.x, c.upper.outer.y);
        let d1 = (l1.0 - l0.0, l1.1 - l0.1);
        let d2 = (u1.0 - u0.0, u1.1 - u0.1);
        let denom = d1.0 * d2.1 - d1.1 * d2.0;
        if denom.abs() < 1e-9 {
            return None;
        }
        let t = ((u0.0 - l0.0) * d2.1 - (u0.1 - l0.1) * d2.0) / denom;
        Some((l0.0 + t * d1.0, l0.1 + t * d1.1))
    };
    let li = ic(left)?;
    let ri = ic(right)?;
    // Contact patches at (±track, 0) in (x, y=hub height ref 0).
    let cl = (-contact_half_track, 0.0);
    let cr = (contact_half_track, 0.0);
    // Line IC->contact per side; roll centre = their intersection.
    let d1 = (cl.0 - li.0, cl.1 - li.1);
    let d2 = (cr.0 - ri.0, cr.1 - ri.1);
    let denom = d1.0 * d2.1 - d1.1 * d2.0;
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = ((ri.0 - li.0) * d2.1 - (ri.1 - li.1) * d2.0) / denom;
    Some(li.1 + t * d1.1)
}

/// ARB axle energy (J): LegacyRatio 1/2 k_rest*ratio dq^2, MotionRatio
/// 1/2 k_bar mr^2 dq^2, with dq = q_left - q_right.
pub fn arb_energy(
    geo: &GeometricSuspensionConfig,
    front: bool,
    dq: f64,
    k_wheel_rest: f64,
) -> f64 {
    if !dq.is_finite() {
        return 0.0;
    }
    let k = match if front { &geo.front_arb } else { &geo.rear_arb } {
        crate::suspension_geo_config::AntiRollConfig::LegacyRatio { ratio } => {
            k_wheel_rest * ratio
        }
        crate::suspension_geo_config::AntiRollConfig::MotionRatio {
            bar_rate_N_per_m,
            motion_ratio,
        } => bar_rate_N_per_m * motion_ratio * motion_ratio,
    };
    0.5 * k.max(0.0) * dq * dq
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspension_geo_config::tests::test_corner;

    fn frame_fl() -> (CornerHardpoints, LinkFrame) {
        let c = test_corner(-0.753, false);
        // Rest-pose outers: use hardpoints directly (q = 0).
        let sol = crate::suspension_kinematics::solve_corner(&c, 0.0, 0.0, true, 0.0, 0.0, 1.0)
            .expect("rest solve");
        let track = c.trackrod_outer;
        let shaft = None;
        let f = link_frame(&c, &sol, track, shaft);
        (c, f)
    }

    #[test]
    fn symmetric_vertical_load_gives_symmetric_reactions() {
        let (c, f) = frame_fl();
        // Pure vertical 1500 N on the upright at W (no moment).
        let res = Vec3::new(0.0, -1500.0, 0.0);
        let (x, r, sing) = solve_links(&f, res, Vec3::ZERO);
        assert!(!sing);
        assert!(r < 1e-6, "residual {r}");
        // Force balance reconstructs the load.
        let mut sum = Vec3::ZERO;
        for (j, (_, _, u)) in f.links.iter().enumerate() {
            sum = sum + *u * x[j];
        }
        assert!((sum - res).length() < 1e-6);
        // Mirrored corner under the mirrored load yields identical axials.
        let cm = crate::suspension_geo_config::mirror_corner(&c);
        let solm = crate::suspension_kinematics::solve_corner(&cm, 0.0, 0.0, true, 0.0, 0.0, -1.0)
            .expect("rest solve mirrored");
        let fm = link_frame(&cm, &solm, cm.trackrod_outer, None);
        let (xm, rm, singm) = solve_links(&fm, res, Vec3::ZERO);
        assert!(!singm);
        assert!(rm < 1e-6, "mirrored residual {rm}");
        for (a, b) in x.iter().zip(xm.iter()) {
            assert!((a - b).abs() < 1e-6, "mirrored axials differ: {x:?} vs {xm:?}");
        }
    }

    #[test]
    fn singular_pose_falls_back_finite() {
        let (c, mut f) = frame_fl();
        // Collapse all directions to +Y (rank 1): must stay finite + flagged.
        for (_, _, u) in f.links.iter_mut() {
            *u = Vec3::UP;
        }
        let (x, _, sing) = solve_links(&f, Vec3::new(0.0, -1000.0, 0.0), Vec3::ZERO);
        assert!(sing);
        assert!(x.iter().all(|v| v.is_finite()));
        let _ = c;
    }

    #[test]
    fn reactions_balance_whole_wheel_load() {
        // Front corner at rest, vertical contact 1500 N at the contact patch
        // (below the hub), no element load: chassis reactions must equal the
        // external load and the shares must sum to ~1.
        let (c, f) = frame_fl();
        let contact = Vec3::new(0.0, 1500.0, 0.0);
        let sol = crate::suspension_kinematics::solve_corner(&c, 0.0, 0.0, true, 0.0, 0.0, 1.0)
            .expect("rest solve");
        let contact_pt = sol.hub + Vec3::new(0.0, -0.33, 0.0);
        let (list, paths) = reactions(
            &f,
            contact,
            contact_pt,
            Vec3::ZERO,
            21.0 * 9.80665,
            0.0,
            0.0,
            Vec3::UP,
            c.rocker_pivot,
            c.damper_chassis,
            0.0,
        );
        // 5 link inners + pivot + damper = 7 entries (rod goes via pivot).
        assert_eq!(list.len(), 7);
        assert!(paths.balance_residual_n < 5.0, "balance {}", paths.balance_residual_n);
        let share_sum = paths.element_share + paths.wishbone_share + paths.other_share;
        assert!((share_sum - 1.0).abs() < 0.02, "shares sum {share_sum}");
        assert!(!paths.singular);
    }
}
